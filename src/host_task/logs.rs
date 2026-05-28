//! Log tailing for the selected container.

use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptionsBuilder;
use bollard::Docker;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::model::HostUpdate;

pub(super) fn spawn_log_task(docker: Docker, container_id: String, tail: u64, tx: mpsc::Sender<HostUpdate>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let opts = LogsOptionsBuilder::default()
            .follow(true)
            .stdout(true)
            .stderr(true)
            .tail(&tail.to_string())
            .timestamps(false)
            .build();
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

            // A chunk isn't guaranteed to be one line; split so bursts don't
            // arrive as a single blob.
            for line in chunk.split('\n') {
                let line = line.trim_end_matches('\r');
                if line.is_empty() {
                    continue;
                }
                let _ = tx.try_send(HostUpdate::LogLine {
                    id: container_id.clone(),
                    line: line.to_string(),
                });
            }
        }
    })
}