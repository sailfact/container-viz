//! Per-host async worker.
//!
//! One [`HostTask`] per Docker host: connects (with retries), polls the
//! container list, streams stats, tails logs, and services [`HostCommand`]
//! messages from the main loop. Outbound via `mpsc::Sender<HostUpdate>`,
//! inbound via `mpsc::Receiver<HostCommand>`.
//!
//! Structured as an outer retry loop (`run`) wrapping a per-connection
//! session (`run_connected`) so backoff never tangles with steady-state polling.

mod connection;
mod logs;
mod stats;

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::{Context, Result};
use bollard::query_parameters::{
    CreateImageOptionsBuilder, ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
    RestartContainerOptions, StartContainerOptions, StopContainerOptionsBuilder,
};
use bollard::Docker;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::model::{ContainerInfo, HostCommand, HostConfig, HostStatus, HostUpdate};

use logs::spawn_log_task;
use stats::spawn_stats_task;

const POLL_INTERVAL_S: u64 = 2;
const RETRY_INTERVAL_S: u64 = 10;

pub struct HostTask {
    config: HostConfig,
    tx: mpsc::Sender<HostUpdate>,
    rx: mpsc::Receiver<HostCommand>,
    retry_interval_s: u64,
    poll_interval_s: u64,
}

/// Live per-session sub-tasks, torn down on disconnect or shutdown.
struct SessionTasks {
    stats: HashMap<String, JoinHandle<()>>,
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

    fn abort_all(&mut self) {
        for (_, h) in self.stats.drain() {
            h.abort();
        }
        if let Some(LogStream { handle, .. }) = self.log.take() {
            handle.abort();
        }
    }
}

enum SessionExit {
    Shutdown,
    Disconnected(String),
}

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

    /// Connect → run until disconnected → back off → repeat. Returns only on
    /// a clean shutdown.
    pub async fn run(mut self) -> Result<()> {
        loop {
            self.emit_status(HostStatus::Connecting).await;

            match connection::connect(&self.config).await {
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
                    self.emit_status(HostStatus::Unreachable(format!("connection failed: {e:#}")))
                        .await;
                }
            }

            if self.backoff_or_shutdown().await {
                return Ok(());
            }
        }
    }

    async fn run_connected(&mut self, docker: Docker) -> SessionExit {
        let mut tasks = SessionTasks::new();
        let mut poll_ticker = time::interval(Duration::from_secs(self.poll_interval_s));
        poll_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let exit = loop {
            tokio::select! {
                _ = poll_ticker.tick() => {
                    if let Err(e) = self.poll_containers(&docker, &mut tasks.stats).await {
                        break SessionExit::Disconnected(format!("poll failed: {e:#}"));
                    }
                }

                cmd = self.rx.recv() => match cmd {
                    Some(HostCommand::Shutdown) | None => break SessionExit::Shutdown,
                    // Action failures are non-fatal. TODO: route to the UI via
                    // a HostUpdate::ActionResult variant for a statusline toast.
                    Some(other) => { let _ = self.handle_command(&docker, other, &mut tasks).await; }
                }
            }
        };

        tasks.abort_all();
        exit
    }

    /// Wait out the retry interval, but still honour Shutdown. Returns `true`
    /// if shutdown was requested.
    async fn backoff_or_shutdown(&mut self) -> bool {
        tokio::select! {
            _ = time::sleep(Duration::from_secs(self.retry_interval_s)) => false,
            cmd = self.rx.recv() => match cmd {
                Some(HostCommand::Shutdown) | None => true,
                Some(_) => false, // can't serve commands while disconnected
            }
        }
    }

    async fn poll_containers(&self, docker: &Docker, stats_tasks: &mut HashMap<String, JoinHandle<()>>) -> Result<()> {
        let opts = ListContainersOptionsBuilder::default().all(true).build();
        let summaries = docker.list_containers(Some(opts)).await.context("list_containers")?;

        let containers: Vec<ContainerInfo> =
            summaries.into_iter().map(ContainerInfo::from_bollard).collect();

        reconcile_stats(docker, &containers, stats_tasks, &self.tx);

        // Send after reconciliation so later stats refer to known containers.
        // Blocking send (not try_send): the list is low-frequency.
        let _ = self.tx.send(HostUpdate::ContainerList(containers)).await;

        Ok(())
    }

    async fn handle_command(&self, docker: &Docker, cmd: HostCommand, tasks: &mut SessionTasks) -> Result<()> {
        match cmd {
            HostCommand::StartContainer(id) => {
                docker
                    .start_container(&id, None::<StartContainerOptions>)
                    .await
                    .context("start_container")?;
            }
            HostCommand::StopContainer(id) => {
                let opts = StopContainerOptionsBuilder::default().t(10).build();
                docker.stop_container(&id, Some(opts)).await.context("stop_container")?;
            }
            HostCommand::RestartContainer(id) => {
                docker
                    .restart_container(&id, None::<RestartContainerOptions>)
                    .await
                    .context("restart_container")?;
            }
            HostCommand::RemoveContainer(id) => {
                let opts = RemoveContainerOptionsBuilder::default().force(true).build();
                docker.remove_container(&id, Some(opts)).await.context("remove_container")?;
            }
            HostCommand::ExecShell(_id) => {
                // Needs the TUI suspended (leave raw mode, spawn blocking child,
                // re-enter); the main loop should intercept before dispatch.
                anyhow::bail!("ExecShell must be handled by the UI layer");
            }
            HostCommand::PullImage(image) => {
                let opts = CreateImageOptionsBuilder::default().from_image(&image).build();
                let mut stream = docker.create_image(Some(opts), None, None);
                while let Some(item) = stream.next().await {
                    item.context("pull_image")?;
                }
            }
            HostCommand::TailLogs { id, lines } => {
                let already_tailing = tasks
                    .log
                    .as_ref()
                    .is_some_and(|l| l.container_id == id && !l.handle.is_finished());

                if !already_tailing {
                    if let Some(LogStream { handle, .. }) = tasks.log.take() {
                        handle.abort();
                    }
                    let handle = spawn_log_task(docker.clone(), id.clone(), lines, self.tx.clone());
                    tasks.log = Some(LogStream { container_id: id, handle });
                }
            }
            HostCommand::Shutdown => unreachable!("Shutdown intercepted in run_connected"),
        }
        Ok(())
    }

    async fn emit_status(&self, status: HostStatus) {
        let _ = self.tx.send(HostUpdate::StatusChange(status)).await;
    }
}

/// Make `stats_tasks` mirror the running-container set: spawn for newcomers,
/// abort for the gone.
fn reconcile_stats(docker: &Docker, containers: &[ContainerInfo], stats_tasks: &mut HashMap<String, JoinHandle<()>>, tx: &mpsc::Sender<HostUpdate>) {
    let live: HashSet<&str> = containers
        .iter()
        .filter(|c| c.is_running())
        .map(|c| c.id.as_str())
        .collect();

    stats_tasks.retain(|id, handle| {
        let keep = live.contains(id.as_str()) && !handle.is_finished();
        if !keep {
            handle.abort();
        }
        keep
    });

    for id in live {
        if !stats_tasks.contains_key(id) {
            let handle = spawn_stats_task(docker.clone(), id.to_string(), tx.clone());
            stats_tasks.insert(id.to_string(), handle);
        }
    }
}