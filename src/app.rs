// app.rs

use std::collections::VecDeque;
use std::collections::HashMap;

use crate::types::container::{ContainerInfo, ContainerState, PortBinding};
use crate::types::host::{HostStatus, ConnectionType, TlsConfig};
use crate::types::messaging::{AppEvent, HostCommand, HostUpdate};
use crate::types::ui::{AppMode, PendingAction, StatusMessage};
use crate::config::{AppConfig, HostConfig};

pub struct AppState {
    pub hosts:              Vec<HostState>,
    pub active_tab:         usize,
    pub mode:               AppMode,
    pub safe_mode:          bool,
    pub log_buffer:         VecDeque<String>,
    pub log_follow:         bool,
    pub log_filter:         Option<String>,
    pub show_details:       bool,
    pub pending_action:     Option<PendingAction>,
    pub command_query:      String,
    pub status_messages:    Option<StatusMessage>,
}

pub struct HostState {
    pub config: HostConfig,
    pub status: HostStatus,
    pub containers: Vec<ContainerInfo>,
    pub selected: usize,
    pub composed_groups: HashMap<String, Vec<ContainerInfo>>,
}