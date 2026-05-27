use std::path;
// docker.rs
use std::{path::PathBuf, time::Duration};

use anyhow::{Ok, Result};
use bollard::Docker;
use bollard::models::ContainerSummary;
use tokio::sync::mpsc::{Sender, Receiver};

use crate::docker;
use crate::types::{
    ConnectionType,
    ContainerInfo,
    HostCommand,
    HostConfig,
    HostStatus,
    HostUpdate,
};

#[derive(Default, Clone)]
struct StatsSnapshot {
    cpu_total:      u64,
    system_total:   u64,
    num_cpus:       u64,
}

pub struct HostTask {
    config:             HostConfig,
    tx:                 Sender<HostUpdate>,
    rx:                 Receiver<HostCommand>,
    retry_intervals:    u64,
}

impl HostTask {
    pub fn new(config: HostConfig, tx: Sender<HostUpdate>, rx: Receiver<HostCommand>) -> Self {
        Self { 
            config, 
            tx, 
            rx, 
            retry_intervals: 0 
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.retry_loop().await;
        Ok(())
    }

    async fn retry_loop(&mut self) {
        todo!()
    }

    async fn connect(&self) -> Result<Docker> {
        match &self.config.connection {
            ConnectionType::UnixSocket(path) => {
                let docker = Docker::connect_with_unix(
                    path.to_str().unwrap_or("/var/run/docker.sock"), 
                    120, 
                    bollard::API_DEFAULT_VERSION,
                )?;
                Ok(docker)
            }
            ConnectionType::Tcp { host, port, tls: None } => {

            } 
            ConnectionType::Tcp { host, port, tls: Somm(tls_config) }  = {
                
            }
        }
    }
}