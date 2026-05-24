use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use ratatui::style::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "connection", rename_all = "lowercase")]
pub enum ConnectionType {
    #[serde(rename = "unix")]
    UnixSocket(PathBuf),
    Tcp {
        host:   String,
        port:   u16,
        tls:    Option<TlsConfig>,
    },
}
impl ConnectionType {
    pub fn is_local(&self) -> bool {
        matches!(self, Self::UnixSocket(_))
    }

    pub fn default_socket() -> Self {
        Self::UnixSocket(PathBuf::from("/var/run/docker.sock"))
    }

    /// Returns the address string bollard expects
    pub fn bollard_addr(&self) -> String {
        match self {
            Self::UnixSocket(path) => format!("unix://{}", path.display()),
            Self::Tcp { host, port, .. } => format!("tcp://{}:{}", host, port),
        }
    }
}
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ContainerState {
    Running,
    Paused,
    Exited, 
    Restarting, 
    Dead, 
    Unknown, 

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
#[derive(Debug, PartialEq, Clone)]
pub enum HostStatus {
    Connecting, 
    Connected,
    Unreachable(String),
}
impl HostStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Connecting        => "Connecting",
            Self::Connected         => "Connected",
            Self::Unreachable(_)    => "Unreachable",
        }
    }

    pub fn colour(&self) -> Color {
        match self {
            Self::Connecting  => Color::Yellow,
            Self::Connected  => Color::Green,
            Self::Unreachable(_) => Color::Red,
        }  
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Connecting        => "󰈀",  // nf-md-lan_pending
            Self::Connected         => "󰡕",  // nf-md-docker (matches tab bar in spec)
            Self::Unreachable(_)    => "󰅚",  // nf-md-close_circle
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum AppMode {
    
}
#[derive(Debug, PartialEq, Clone)]
pub enum MessageLevel {
    Info,
    Warn,
    Error,
}
impl MessageLevel {
    pub fn colour(&self) -> Color {
        match self {
            Self::Info  => Color::Gray,
            Self::Warn  => Color::Yellow,
            Self::Error => Color::Red,
        }  
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum HostUpdate {
    
}
#[derive(Debug, PartialEq, Clone)]
pub enum HostCommand {
   
}
#[derive(Debug, Clone)]
pub enum AppEvent {

}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path:  PathBuf,
    pub ca_cert:    Option<PathBuf>,
    pub verify:     bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::new(),
            ca_cert: None,
            verify: true,  // safe default
        }
    }
}
#[derive(Debug, Clone)]
pub struct PortBinding {

}
#[derive(Debug, Clone)]
pub struct ContainerInfo {

}
#[derive(Debug, Clone)]
pub struct PendingAction {

}
#[derive(Debug, Clone)]
pub struct StatusMessage {

}

pub struct PaletteAction {

}
