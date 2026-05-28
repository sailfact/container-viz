//! Per-container stats sampling and CPU percentage derivation.

use bollard::models::ContainerStatsResponse;
use bollard::query_parameters::StatsOptionsBuilder;
use bollard::Docker;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::model::HostUpdate;

/// Raw counters needed to derive a CPU percentage, isolated so
/// `calc_cpu_percent` can be tested without a live daemon.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct StatsSnapshot {
    cpu_total: u64,
    system_cpu: u64,
    online_cpus: u64,
}

impl StatsSnapshot {
    fn from_response(s: &ContainerStatsResponse) -> Self {
        let cpu = s.cpu_stats.as_ref();
        Self {
            cpu_total: cpu
                .and_then(|c| c.cpu_usage.as_ref())
                .and_then(|u| u.total_usage)
                .unwrap_or(0),
            system_cpu: cpu.and_then(|c| c.system_cpu_usage).unwrap_or(0),
            online_cpus: cpu.and_then(|c| c.online_cpus).unwrap_or(1).max(1) as u64,
        }
    }
}

fn calc_cpu_percent(prev: StatsSnapshot, curr: StatsSnapshot) -> f64 {
    let cpu_delta = curr.cpu_total.saturating_sub(prev.cpu_total) as f64;
    let system_delta = curr.system_cpu.saturating_sub(prev.system_cpu) as f64;
    if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * curr.online_cpus as f64 * 100.0
    } else {
        0.0
    }
}

pub(super) fn spawn_stats_task(docker: Docker, container_id: String, tx: mpsc::Sender<HostUpdate>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let opts = StatsOptionsBuilder::default().stream(true).one_shot(false).build();
        let mut stream = docker.stats(&container_id, Some(opts));
        let mut prev: Option<StatsSnapshot> = None;

        while let Some(item) = stream.next().await {
            let stats = match item {
                Ok(s) => s,
                Err(_) => break,
            };

            let curr = StatsSnapshot::from_response(&stats);
            let cpu = match prev {
                Some(p) => calc_cpu_percent(p, curr),
                None => 0.0, // need two samples for a delta
            };
            prev = Some(curr);

            let mem = stats.memory_stats.as_ref().and_then(|m| m.usage).unwrap_or(0);
            let (net_rx, net_tx) = stats
                .networks
                .as_ref()
                .map(|nets| {
                    nets.values().fold((0u64, 0u64), |(rx, tx), n| {
                        (rx + n.rx_bytes.unwrap_or(0), tx + n.tx_bytes.unwrap_or(0))
                    })
                })
                .unwrap_or((0, 0));

            // try_send: a dropped sample is invisible, the next is ~1s away.
            let _ = tx.try_send(HostUpdate::StatsUpdate {
                id: container_id.clone(),
                cpu,
                mem,
                net_rx,
                net_tx,
            });
        }
    })
}

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
        assert!((calc_cpu_percent(snap(0, 0, 1), snap(25, 100, 1)) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_percent_scales_with_cpus() {
        assert!((calc_cpu_percent(snap(0, 0, 4), snap(25, 100, 4)) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn cpu_percent_zero_when_no_delta() {
        let s = snap(10, 100, 2);
        assert_eq!(calc_cpu_percent(s, s), 0.0);
    }

    #[test]
    fn cpu_percent_handles_counter_reset() {
        assert_eq!(calc_cpu_percent(snap(100, 1_000, 1), snap(50, 500, 1)), 0.0);
    }
}