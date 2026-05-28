//! Per-host async worker.
//!
//! One [`HostTask`] is spawned per Docker host listed in the user's config.
//! Each task owns its own [`bollard::Docker`] client and is responsible for:
//!
//! * Connecting (with retries) to the host's Docker daemon
//! * Polling the container list on a fixed interval
//! * Streaming per-container resource stats
//! * Tailing logs for one selected container at a time
//! * Servicing [`HostCommand`] messages from the main loop
//!
//! All outbound communication happens through an `mpsc::Sender<HostUpdate>`;
//! all inbound commands arrive on an `mpsc::Receiver<HostCommand>`.
//!
//! The task is structured as an outer retry loop (`run`) wrapping a
//! per-connection session (`run_connected`). Keeping the two apart means
//! reconnect/backoff logic never gets entangled with steady-state polling.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::{Context, Result};
use bollard::container::{
    ListContainersOptions, LogOutput, LogsOptions, RemoveContainerOptions, Stats, StatsOptions,
    StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::types::{
    ConnectionType, ContainerInfo, HostCommand, HostConfig, HostStatus, HostUpdate, TlsConfig,
};

// ---------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------

/// Container list re-poll interval, in seconds.
const POLL_INTERVAL_S: u64 = 2;
/// Time to wait between reconnect attempts after a connection failure.
const RETRY_INTERVAL_S: u64 = 10;
/// Timeout (seconds) passed to bollard when constructing a client.
const CONNECT_TIMEOUT_S: u64 = 30;

// ---------------------------------------------------------------------------
// HostTask
// ---------------------------------------------------------------------------

/// Long-running async worker owning a single Docker host connection.
pub struct HostTask {
    config: HostConfig,
    tx: mpsc::Sender<HostUpdate>,
    rx: mpsc::Receiver<HostCommand>,
    retry_interval_s: u64,
    poll_interval_s: u64,
}

/// Per-session sub-task handles. Lives entirely inside `run_connected` and
/// is torn down whenever we disconnect or shut down. Pulled into a struct
/// purely so `handle_command` has one place to look for live work.
struct SessionTasks {
    /// One stats stream per running container, keyed by container id.
    stats: HashMap<String, JoinHandle<()>>,
    /// At most one active log tail, paired with its container id so we know
    /// whether the user has asked to switch to a different container.
    log: Option<LogStream>,
}

struct LogStream {
    container_id: String,
    handle: JoinHandle<()>,
}

impl SessionTasks {
    fn new() -> Self {
        Self {
            stats: HashMap::new(),
            log: None,
        }
    }

    /// Cancel every spawned sub-task so they don't outlive the connection.
    fn abort_all(&mut self) {
        for (_, h) in self.stats.drain() {
            h.abort();
        }
        if let Some(LogStream { handle, .. }) = self.log.take() {
            handle.abort();
        }
    }
}

/// How a `run_connected` session terminated.
enum SessionExit {
    /// Shutdown command received or command channel closed by the main loop.
    Shutdown,
    /// Connection-level error — caller should back off and reconnect.
    Disconnected(String),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

impl HostTask {
    pub fn new(config: HostConfig, tx: mpsc::Sender<HostUpdate>, rx: mpsc::Receiver<HostCommand>) -> Self {
        Self {
            config,
            tx,
            rx,
            retry_interval_s: RETRY_INTERVAL_S,
            poll_interval_s: POLL_INTERVAL_S,
        }
    }

    /// Outer retry loop. Connect → run until disconnected → back off → repeat.
    ///
    /// Returns `Ok(())` only on a clean shutdown — either a `Shutdown`
    /// command, or because the main loop dropped our command channel.
    pub async fn run(mut self) -> Result<()> {
        loop {
            self.emit_status(HostStatus::Connecting).await;

            match self.connect().await {
                Ok(docker) => {
                    self.emit_status(HostStatus::Connected).await;
                    match self.run_connected(docker).await {
                        SessionExit::Shutdown => return Ok(()),
                        SessionExit::Disconnected(msg) => {
                            self.emit_status(HostStatus::Unreachable(msg)).await;
                        }
                    }
                }
                Err(e) => {
                    self.emit_status(HostStatus::Unreachable(format!(
                        "connection failed: {e:#}"
                    )))
                    .await;
                }
            }

            // We still listen for Shutdown during the backoff so the app
            // can exit promptly even while a host is unreachable.
            if self.backoff_or_shutdown().await {
                return Ok(());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Connection / lifecycle
// ---------------------------------------------------------------------------

impl HostTask {
    async fn connect(&self) -> Result<Docker> {
        let docker = match &self.config.connection {
            ConnectionType::UnixSocket(path) => {
                let path_str = path.to_string_lossy();
                Docker::connect_with_unix(
                    &path_str,
                    CONNECT_TIMEOUT_S,
                    bollard::API_DEFAULT_VERSION,
                )
                .context("constructing unix socket client")?
            }
            ConnectionType::Tcp { host, port, tls } => {
                let addr = format!("tcp://{host}:{port}");
                match tls {
                    Some(tls_cfg) => connect_tls(&addr, tls_cfg)?,
                    None => Docker::connect_with_http(
                        &addr,
                        CONNECT_TIMEOUT_S,
                        bollard::API_DEFAULT_VERSION,
                    )
                    .context("constructing http client")?,
                }
            }
        };

        // The constructors above only build a client — they don't actually
        // talk to the daemon. Ping forces a round-trip so a misconfigured
        // host fails here rather than during the first poll.
        docker.ping().await.context("ping")?;
        Ok(docker)
    }

    /// Inner steady-state loop. Returns once the connection is lost or the
    /// main loop asks us to shut down.
    async fn run_connected(&mut self, docker: Docker) -> SessionExit {
        let mut tasks = SessionTasks::new();
        let mut poll_ticker = time::interval(Duration::from_secs(self.poll_interval_s));
        // If we ever fall behind (e.g. a slow command held the select),
        // skip the catch-up bursts that `interval` would otherwise produce.
        poll_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let exit = loop {
            tokio::select! {
                _ = poll_ticker.tick() => {
                    if let Err(e) = self.poll_containers(&docker, &mut tasks.stats).await {
                        break SessionExit::Disconnected(format!("poll failed: {e:#}"));
                    }
                }

                cmd = self.rx.recv() => match cmd {
                    Some(HostCommand::Shutdown) => break SessionExit::Shutdown,
                    None => break SessionExit::Shutdown, // main loop dropped us
                    Some(other) => {
                        // Action failures are non-fatal: the daemon is fine,
                        // the user just asked for something that didn't work.
                        // TODO: route these back to the UI via a new
                        // `HostUpdate::ActionResult` variant so the
                        // statusline can show an error toast. For now we
                        // swallow them silently — writing to stderr while
                        // crossterm holds raw mode corrupts the screen.
                        let _ = self.handle_command(&docker, other, &mut tasks).await;
                    }
                }
            }
        };

        tasks.abort_all();
        exit
    }

    /// Wait `retry_interval_s` while still listening for a Shutdown command.
    /// Returns `true` if shutdown was requested during the wait.
    async fn backoff_or_shutdown(&mut self) -> bool {
        tokio::select! {
            _ = time::sleep(Duration::from_secs(self.retry_interval_s)) => false,
            cmd = self.rx.recv() => match cmd {
                Some(HostCommand::Shutdown) | None => true,
                // Drop any non-Shutdown commands that arrive while we're
                // disconnected — they can't be served and the user will
                // see the red host indicator anyway.
                Some(_) => false,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Polling
// ---------------------------------------------------------------------------

impl HostTask {
    async fn poll_containers(&self, docker: &Docker, stats_tasks: &mut HashMap<String, JoinHandle<()>>) -> Result<()> {
        let opts = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        let summaries = docker
            .list_containers(Some(opts))
            .await
            .context("list_containers")?;

        let containers: Vec<ContainerInfo> = summaries
            .into_iter()
            .map(ContainerInfo::from_bollard)
            .collect();

        reconcile_stats(docker, &containers, stats_tasks, &self.tx);

        // Send the new list *after* reconciliation so any subsequent stats
        // updates always correspond to containers the UI already knows
        // about. Use blocking `send` here (not `try_send`): the list is
        // low-frequency and we'd rather apply backpressure than drop it.
        let _ = self.tx.send(HostUpdate::ContainerList(containers)).await;

        Ok(())
    }
}

/// Make `stats_tasks` reflect exactly the set of currently-running
/// containers: spawn streams for newcomers, abort streams for the gone.
fn reconcile_stats(docker: &Docker, containers: &[ContainerInfo], stats_tasks: &mut HashMap<String, JoinHandle<()>>, tx: &mpsc::Sender<HostUpdate>) {
    let live: HashSet<&str> = containers
        .iter()
        .filter(|c| c.is_running())
        .map(|c| c.id.as_str())
        .collect();

    // Drop streams for containers that are gone, no longer running, or
    // whose stream has already finished on its own.
    stats_tasks.retain(|id, handle| {
        let keep = live.contains(id.as_str()) && !handle.is_finished();
        if !keep {
            handle.abort();
        }
        keep
    });

    // Spawn streams for newly-running containers.
    for id in live {
        if !stats_tasks.contains_key(id) {
            let handle = spawn_stats_task(docker.clone(), id.to_string(), tx.clone());
            stats_tasks.insert(id.to_string(), handle);
        }
    }
}

// ---------------------------------------------------------------------------
// Stats streaming
// ---------------------------------------------------------------------------

/// Snapshot of the raw counters needed to derive a CPU percentage.
///
/// Extracted into its own type so [`calc_cpu_percent`] can be unit-tested
/// from a few literal values without needing a live Docker daemon.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct StatsSnapshot {
    /// Cumulative CPU time used by the container's cgroup, in nanoseconds.
    cpu_total: u64,
    /// Cumulative system-wide CPU time, in nanoseconds.
    system_cpu: u64,
    /// Number of CPUs visible to the container.
    online_cpus: u64,
}

impl StatsSnapshot {
    fn from_stats(stats: &Stats) -> Self {
        Self {
            cpu_total: stats.cpu_stats.cpu_usage.total_usage,
            system_cpu: stats.cpu_stats.system_cpu_usage.unwrap_or(0),
            // Older daemons may not report `online_cpus`; fall back to 1
            // and floor at 1 so the maths can't accidentally produce NaN.
            online_cpus: stats.cpu_stats.online_cpus.unwrap_or(1).max(1),
        }
    }
}

/// Compute CPU percent from two successive snapshots.
///
/// Follows the formula documented by Docker: scale the container's
/// CPU-time delta by its share of the system CPU-time delta, then multiply
/// by the number of available CPUs. The result is 0–(online_cpus * 100).
fn calc_cpu_percent(prev: StatsSnapshot, curr: StatsSnapshot) -> f64 {
    // `saturating_sub` rather than `-`: a (very rare) counter reset
    // shouldn't panic on underflow.
    let cpu_delta = curr.cpu_total.saturating_sub(prev.cpu_total) as f64;
    let system_delta = curr.system_cpu.saturating_sub(prev.system_cpu) as f64;
    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * curr.online_cpus as f64 * 100.0
    } else {
        0.0
    }
}

fn spawn_stats_task(docker: Docker, container_id: String, tx: mpsc::Sender<HostUpdate>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let opts = StatsOptions {
            stream: true,
            one_shot: false,
        };
        let mut stream = docker.stats(&container_id, Some(opts));
        let mut prev: Option<StatsSnapshot> = None;

        while let Some(item) = stream.next().await {
            let stats = match item {
                Ok(s) => s,
                Err(_) => break, // container stopped or daemon dropped us
            };

            let curr = StatsSnapshot::from_stats(&stats);
            let cpu = match prev {
                Some(p) => calc_cpu_percent(p, curr),
                None => 0.0, // need two samples before we can produce a delta
            };
            prev = Some(curr);

            let mem = stats.memory_stats.usage.unwrap_or(0);
            let (net_rx, net_tx) = stats
                .networks
                .as_ref()
                .map(|nets| {
                    nets.values().fold((0u64, 0u64), |(rx, tx), n| {
                        (rx + n.rx_bytes, tx + n.tx_bytes)
                    })
                })
                .unwrap_or((0, 0));

            let update = HostUpdate::StatsUpdate {
                id: container_id.clone(),
                cpu,
                mem,
                net_rx,
                net_tx,
            };

            // `try_send`: dropping a stats sample if the UI is briefly
            // behind is invisible — the next sample is a second away.
            let _ = tx.try_send(update);
        }
    })
}

// ---------------------------------------------------------------------------
// Log tailing
// ---------------------------------------------------------------------------

fn spawn_log_task(docker: Docker, container_id: String, tail: u64, tx: mpsc::Sender<HostUpdate>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let opts = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            timestamps: false,
            ..Default::default()
        };
        let mut stream = docker.logs(&container_id, Some(opts));

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(LogOutput::StdOut { message })
                | Ok(LogOutput::StdErr { message })
                | Ok(LogOutput::Console { message }) => {
                    String::from_utf8_lossy(&message).into_owned()
                }
                Ok(LogOutput::StdIn { .. }) => continue,
                Err(_) => break,
            };

            // One chunk from bollard usually maps to one log line, but it's
            // not guaranteed — split defensively so multi-line bursts don't
            // arrive at the UI as a single blob.
            for line in chunk.split('\n') {
                let line = line.trim_end_matches('\r');
                if line.is_empty() {
                    continue;
                }
                let update = HostUpdate::LogLine {
                    id: container_id.clone(),
                    line: line.to_string(),
                };
                let _ = tx.try_send(update);
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

impl HostTask {
    async fn handle_command(&self, docker: &Docker, cmd: HostCommand, tasks: &mut SessionTasks) -> Result<()> {
        match cmd {
            HostCommand::StartContainer(id) => {
                docker
                    .start_container::<String>(&id, None)
                    .await
                    .context("start_container")?;
            }
            HostCommand::StopContainer(id) => {
                docker
                    .stop_container(&id, Some(StopContainerOptions { t: 10 }))
                    .await
                    .context("stop_container")?;
            }
            HostCommand::RestartContainer(id) => {
                docker
                    .restart_container(&id, None)
                    .await
                    .context("restart_container")?;
            }
            HostCommand::RemoveContainer(id) => {
                let opts = RemoveContainerOptions {
                    force: true,
                    v: false,
                    link: false,
                };
                docker
                    .remove_container(&id, Some(opts))
                    .await
                    .context("remove_container")?;
            }
            HostCommand::ExecShell(_id) => {
                // Properly handing the terminal off to `docker exec` means
                // suspending the TUI (leaving raw mode, restoring the
                // cursor, spawning a blocking child, re-entering raw mode
                // on return). That's a UI-layer concern, not a HostTask
                // one — the main loop should intercept ExecShell before
                // dispatching here.
                anyhow::bail!("ExecShell must be handled by the UI layer");
            }
            HostCommand::PullImage(image) => {
                let opts = CreateImageOptions::<String> {
                    from_image: image.clone(),
                    ..Default::default()
                };
                let mut stream = docker.create_image(Some(opts), None, None);
                // Drain the progress stream. The first error short-circuits;
                // success messages are currently discarded. TODO: surface
                // layer-by-layer progress to the statusline once HostUpdate
                // has a slot for it.
                while let Some(item) = stream.next().await {
                    item.context("pull_image")?;
                }
            }
            HostCommand::TailLogs { id, lines } => {
                // If we're already tailing the same container, leave the
                // existing stream alone — no need to flicker the buffer.
                // Otherwise abort the previous tail (if any) and start
                // a new one.
                let already_tailing = tasks
                    .log
                    .as_ref()
                    .is_some_and(|l| l.container_id == id && !l.handle.is_finished());

                if !already_tailing {
                    if let Some(LogStream { handle, .. }) = tasks.log.take() {
                        handle.abort();
                    }
                    let handle =
                        spawn_log_task(docker.clone(), id.clone(), lines, self.tx.clone());
                    tasks.log = Some(LogStream {
                        container_id: id,
                        handle,
                    });
                }
            }
            HostCommand::Shutdown => {
                // Intercepted in `run_connected`'s select arm — reaching
                // here would be a bug in this file.
                unreachable!("Shutdown should not reach handle_command");
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TLS helper
// ---------------------------------------------------------------------------

/// Build a TLS-secured Docker client.
///
/// `cert_path` is treated as a directory containing `cert.pem` and
/// `key.pem`; the CA cert defaults to `ca.pem` in the same directory
/// unless explicitly overridden. This matches the Docker daemon's
/// own convention for `~/.docker/certs/<host>/`.
fn connect_tls(addr: &str, tls: &TlsConfig) -> Result<Docker> {
    let cert = tls.cert_path.join("cert.pem");
    let key = tls.cert_path.join("key.pem");
    let ca = tls
        .ca_cert
        .clone()
        .unwrap_or_else(|| tls.cert_path.join("ca.pem"));

    // NOTE: bollard's TLS connector always verifies the peer certificate.
    // The `verify: false` knob on TlsConfig isn't honoured yet — supporting
    // it would mean dropping to a custom hyper connector.
    Docker::connect_with_ssl(
        addr,
        &key,
        &cert,
        &ca,
        CONNECT_TIMEOUT_S,
        bollard::API_DEFAULT_VERSION,
    )
    .context("constructing TLS client")
}

// ---------------------------------------------------------------------------
// Outbound helpers
// ---------------------------------------------------------------------------

impl HostTask {
    /// Send a status change to the main loop. Uses blocking `send` so a
    /// status change can't be silently dropped under load — they're rare
    /// enough that any backpressure is acceptable.
    async fn emit_status(&self, status: HostStatus) {
        let _ = self.tx.send(HostUpdate::StatusChange(status)).await;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(cpu: u64, sys: u64, cpus: u64) -> StatsSnapshot {
        StatsSnapshot {
            cpu_total: cpu,
            system_cpu: sys,
            online_cpus: cpus,
        }
    }

    #[test]
    fn cpu_percent_basic() {
        // Container used 25% of one CPU between samples.
        let prev = snap(0, 0, 1);
        let curr = snap(25, 100, 1);
        assert!((calc_cpu_percent(prev, curr) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_percent_scales_with_cpus() {
        // Same deltas, but four CPUs visible — daemon reports a percentage
        // that can exceed 100 when work is parallelised across cores.
        let prev = snap(0, 0, 4);
        let curr = snap(25, 100, 4);
        assert!((calc_cpu_percent(prev, curr) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_percent_zero_when_no_delta() {
        let s = snap(10, 100, 2);
        assert_eq!(calc_cpu_percent(s, s), 0.0);
    }

    #[test]
    fn cpu_percent_handles_counter_reset() {
        // If a counter went backwards (shouldn't happen, but we don't want
        // to panic if it does) we want to read zero, not panic.
        let prev = snap(100, 1_000, 1);
        let curr = snap(50, 500, 1);
        assert_eq!(calc_cpu_percent(prev, curr), 0.0);
    }
}