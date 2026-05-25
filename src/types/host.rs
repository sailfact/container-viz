use super::*;

use std::collections::HashMap;
// Represents a single Docker host entry from config — its name and how to connect to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "host", rename_all = "lowercase")]
pub struct HostConfig {
    #[serde(flatten)]
    pub name:       String,
    pub connection: ConnectionType,
}
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

#[derive(Debug, PartialEq, Clone)]
pub enum HostStatus {
    Connecting, 
    Connected,
    Unreachable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path:  PathBuf,
    pub ca_cert:    Option<PathBuf>,
    pub verify:     bool,
}

pub struct HostState {
    pub config: HostConfig,
    pub status: HostStatus,
    pub containers: Vec<ContainerInfo>,
    pub selected: usize,
    pub composed_groups: HashMap<String, Vec<ContainerInfo>>,
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


impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: PathBuf::new(),
            ca_cert: None,
            verify: true,  // safe default
        }
    }
}

impl HostState {
    pub fn new(config: HostConfig) -> Self {
        Self {
            config,
            status: HostStatus::Connecting,
            containers: Vec<ContainerInfo>::new(),
            selected: 0,
            composed_groups: HashMap<String, Vec<ContainerInfo>>::new(),
        }
    }
    pub fn apply_container_list(&self, containers: Vec<ContainerInfo>) {
        todo!()
    }
    pub fn apply_stats_update(&self, id: str, cpu: f64, mem: u64, net_rx: u64, net_tx: u64){
        todo!()
    }
    pub fn append_log_line(&self, id: str, line: String) {
        todo!()
    }
    pub fn selected_container(&self) -> Option<ContainerInfo> {
        todo!()
    }
    pub fn next_container(&self) {
        todo!()
    }
    pub fn prev_container(&self) {
        todo!()
    }
    pub fn jump_top(&self) {
        todo!()
    }
    pub fn jump_bottom(&self) {
        todo!()
    }
    pub fn running_count(&self) -> usize {
        todo!()
    }
    pub fn total_count(&self) -> usize {
        todo!()
    }
    pub fn grouped_by_compose(&self) -> Vec<ComposeGroup> {
        todo!()
    }
}

impl HostConfig {
    pub fn new(&self, name: String, connection: ConnectionType) -> Self {
        Self { name, connection, }
    }
    //Returns a human-readable label for the tab bar (could be name or derived from connection details)
    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.connection.bollard_addr())
    }
    // Returns true if the connection is a local Unix socket, used to skip TLS logic
    pub fn is_local(&self) -> bool {
        self.connection.is_local()
    }
}