use super::Color;

use std::collections::VecDeque;
use std::time::Duration;

const SPARKLINE_LEN: usize = 60;

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ContainerState {
    Running,
    Paused,
    Exited, 
    Restarting, 
    Dead, 
    Unknown, 
    
}

#[derive(Debug, Clone)]
pub struct PortBinding {
    pub host_ip:        String,
    pub host_port:      u16,
    pub container_port: u16,
    pub protocol:       String,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub status: String,
    pub uptime: Option<Duration>,
    pub compose_project: Option<String>,
    pub ports: Vec<PortBinding>,
    pub cpu_percent: f64,
    pub mem_usage: u64,
    pub mem_limit: u64,
    pub cpu_history: VecDeque<f64>,
    pub mem_history: VecDeque<u64>,
    pub net_rx: u64,
    pub net_tx: u64,
    pub log_lines: VecDeque<String>,
}

impl ContainerInfo {
    pub fn push_cpu_sample(&mut self, cpu: f64) {
        self.cpu_percent = cpu;
        if self.cpu_history.len() >= SPARKLINE_LEN {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(cpu);
    }

    pub fn push_mem_sample(&mut self, mem: u64) {
        self.mem_usage = mem;
        if self.mem_history.len() >= SPARKLINE_LEN {
            self.mem_history.pop_front();
        }
        self.mem_history.push_back(mem);
    }

    pub fn cpu_sparkline(&self) -> Vec<f64> {
        self.cpu_history.iter().copied().collect()
    }

    pub fn mem_sparkline(&self) -> Vec<u64> {
        self.mem_history.iter().copied().collect()
    }

    pub fn mem_percent(&self) -> f64 {
        if self.mem_limit == 0 {
            return 0.0;
        }
        (self.mem_usage as f64 / self.mem_limit as f64) * 100.0
    }

    pub fn is_running(&self) -> bool {
        self.state == ContainerState::Running
    }

    pub fn short_id(&self) -> &str {
        &self.id[..self.id.len().min(12)]
    }

    pub fn formatted_uptime(&self) -> String {
        match self.uptime {
            None => "—".to_string(),
            Some(d) => {
                let secs = d.as_secs();
                let days = secs / 86400;
                let hours = (secs % 86400) / 3600;
                let mins = (secs % 3600) / 60;
                match (days, hours, mins) {
                    (d, h, _) if d > 0 => format!("{}d {}h", d, h),
                    (_, h, m) if h > 0 => format!("{}h {}m", h, m),
                    (_, _, m)           => format!("{}m", m),
                }
            }
        }
    }
}

impl ContainerState {
    pub fn colour(&self) -> Color {
        match self {
            Self::Running    => Color::Green,
            Self::Paused     => Color::Yellow,
            Self::Restarting => Color::Yellow,
            Self::Exited     => Color::DarkGray,
            Self::Dead       => Color::Red,
            Self::Unknown    => Color::Gray,
        }   
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Running    => "󰄬",  // nf-md-check_circle
            Self::Paused     => "󰏤",  // nf-md-pause_circle
            Self::Restarting => "󰑐",  // nf-md-restart
            Self::Exited     => "󱗜",  // nf-md-circle_off (matches design spec layout)
            Self::Dead       => "󰚌",  // nf-md-skull
            Self::Unknown    => "󰋗",  // nf-md-help_circle
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Running    => "running",
            Self::Paused     => "paused",
            Self::Restarting => "restarting",
            Self::Exited     => "exited",
            Self::Dead       => "dead",
            Self::Unknown    => "unknown",
        }
    }
}

impl From<&str> for ContainerState {
    fn from(s: &str) -> Self {
        match s {
            "running"    => Self::Running,
            "paused"     => Self::Paused,
            "restarting" => Self::Restarting,
            "exited"     => Self::Exited,
            "dead"       => Self::Dead,
            _            => Self::Unknown,
        }
    }
}


impl PortBinding {
    pub fn display(&self) -> String {
        format!(
            "{}:{}->{}/ {}",
            self.host_ip, self.host_port, self.container_port, self.protocol
        )
    }
}