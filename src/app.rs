use crate::config::AppConfig;
use crate::types::app::{AppMode, AppState, MessageLevel, PendingAction, StatusMessage};
use crate::types::messages::{HostCommand, HostUpdate};


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
