use super::*;

use std::collections::HashMap;

const LOG_BUFFER_CAPACITY: usize = 1000;

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

// Represents a single Docker host entry from config — its name and how to connect to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "host", rename_all = "lowercase")]
pub struct HostConfig {
    #[serde(flatten)]
    pub name:       String,
    pub connection: ConnectionType,
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

pub struct ComposeGroup {
    pub name: String,
    pub containers: Vec<ContainerInfo>,
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
            containers: Vec::new(),
            selected: 0,
            composed_groups: HashMap::new(),
        }
    }

    // Replaces the entire container list with a fresh snapshot from Docker.
    // Called when HostTask polls and gets back a new ContainerList update.
    pub fn apply_container_list(&mut self, containers: Vec<ContainerInfo>) {
        self.containers = containers;
        self.selected = 0; // reset selection to top on new list
    }

    // Finds the container matching `id` and updates its live stats —
    // CPU%, memory usage, and network rx/tx byte counts.
    pub fn apply_stats_update(&mut self, id: &str, cpu: f64, mem: u64, net_rx: u64, net_tx: u64){
        self.containers.iter_mut()
            .find(|container| container.id == id)
            .map(|container| {
                container.cpu_percent = cpu;
                container.mem_usage = mem;
                container.net_rx = net_rx;
                container.net_tx = net_tx;
            });
    }

    // Finds the container matching `id` and appends a new log line to its
    // log_lines ring buffer, dropping the oldest line if at capacity.
    pub fn append_log_line(&mut self, id: &str, line: String) {
        let Some(container) = self.containers.iter_mut().find(|c| c.id == id) else {
            return;
        };

        container.log_lines.push_back(line);

        if container.log_lines.len() > LOG_BUFFER_CAPACITY {
            container.log_lines.pop_front();
        }
    }
    
    // Returns a reference to the currently selected container, or None
    // if the container list is empty.
    pub fn selected_container(&self) -> Option<ContainerInfo> {
        if self.containers.is_empty() {
            None
        } else {
            Some(self.containers[self.selected].clone())
        }
    }

    // Moves the selection down one row, stopping at the last container.
    pub fn next_container(&mut self) {
        if self.containers.is_empty() {
            return;
        }
        if self.selected < self.containers.len() - 1 {
            self.selected += 1;
        }
    }

    // Moves the selection up one row, stopping at the first container.
    pub fn prev_container(&mut self) {
        if self.containers.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    // Jumps selection to the first container in the list.

    pub fn jump_top(&mut self) {
        self.selected = 0;
    }

    // Jumps selection to the last container in the list.
    pub fn jump_bottom(&mut self) {
        if self.containers.is_empty() {
            return;
        } 
        self.selected = self.containers.len() - 1
    }

    // Returns the count of containers currently in Running state.
    pub fn running_count(&self) -> usize {
        self.containers
            .iter()
            .filter(|c| c.is_running())
            .count()
    }

    // Returns the total number of containers regardless of state.
    pub fn total_count(&self) -> usize {
        self.containers.len()
    }

    // Groups containers by their Compose project label and returns them
    // as a sorted list of ComposeGroup. Containers with no project are
    // grouped separately.
    pub fn grouped_by_compose(&self) -> Vec<ComposeGroup> {
        let mut map: HashMap<String, Vec<ContainerInfo>> = HashMap::new();

        for container in &self.containers {
            let key = container
                .compose_project
                .clone()
                .unwrap_or_else(|| "ungrouped".to_string());

            map.entry(key)
                .or_insert_with(Vec::new)
                .push(container.clone());
        }

        let mut groups: Vec<ComposeGroup> = map
            .into_iter()
            .map(|(name, containers)| ComposeGroup { name, containers })
            .collect();

        groups.sort_by(|a, b| match (a.name.as_str(), b.name.as_str()) {
            ("ungrouped", _) => std::cmp::Ordering::Greater,
            (_, "ungrouped") => std::cmp::Ordering::Less,
            _ => a.name.cmp(&b.name),
        });

        groups
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