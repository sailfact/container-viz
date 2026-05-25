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

impl AppState {
    pub fn new(config: AppConfig) -> Self {}
    pub fn active_host(&self) -> &HostState {}
    pub fn active_host_mut(&self) -> &mut HostState {}
    pub fn next_tab(&mut self) {}
    pub fn prev_tab(&mut self) {}
    pub fn apply_update(host_idx: usize, update: HostUpdate) {}
    pub fn dispatch_command(host_idx: usize, cmd: HostCommand) {}
    pub fn set_mode(mode: AppMode) {}
    pub fn toggle_safe_mode() {}
    pub fn toggle_detail() {}
    pub fn set_pending_action(action: PendingAction) {}
    pub fn clear_pending_action() {}
    pub fn set_status(msg: String, level: MessageLevel) {}
    pub fn tick_status() {}
}