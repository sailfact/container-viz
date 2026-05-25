// app.rs
// Implments AppState

// use std::path::PathBuf;
// use serde::{Deserialize, Serialize};
// use ratatui::style::Color;
use std::collections::VecDeque;
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::types::app::{AppMode, AppState, MessageLevel, PendingAction, StatusMessage};
use crate::types::messages::{HostCommand, HostUpdate};
use crate::types::HostState;


impl AppState {
    pub fn new(config: AppConfig, command_tx: Vec<mpsc::Sender<HostCommand>>) -> Self {
        let hosts = config.hosts
            .iter()
            .map(|host_config| HostState::new(host_config.clone()))
            .collect();
        Self { 
            hosts,
            command_tx,
            active_tab: 0, 
            mode: AppMode::Normal, 
            safe_mode: config.safe_mode, 
            log_buffer: VecDeque::new(), 
            log_follow: true, 
            log_filter: None, 
            show_details: false, 
            pending_action: None, 
            command_query: String::new(), 
            status_messages: None, 
        }
    }
    /* Tab navigation */
    // active_host()
    // Returns a shared reference to the currently active `HostState`.
    pub fn active_host(&self) -> &HostState {
        &self.hosts[self.active_tab]
    }
    
    // active_host_mut()
    // Returns a mutable reference to the currently active `HostState`.
    pub fn active_host_mut(&mut self) -> Option<&mut HostState> {
        self.hosts.get_mut(self.active_tab)
    }
    
    // next_tab()
    // Move to next host tab wrapping around
    pub fn next_tab(&mut self) {
        if self.hosts.is_empty() {
            return;
        } else if self.active_tab == self.hosts.len() - 1 {
            self.active_tab = 0;
        } else {
            self.active_tab += 1;
        }
    }
    // prev_tab()
    // Move to previous host tab wrapping around 
    pub fn prev_tab(&mut self) {
        if self.hosts.is_empty() {
            return;
        } else if self.active_tab == 0 {
            self.active_tab = self.hosts.len() - 1;
        } else {
            self.active_tab -= 1;
        }
    }
    
    /* Inbound Updates */
    // apply_update()
    // Apply a 'HostUpdate' from a 'HostTask' to the matching host
    pub fn apply_update(&mut self, host_idx: usize, update: HostUpdate) {
        let Some(host) = self.hosts.get_mut(host_idx) else {
        self.set_status(
            format!("Received update for unknown host index {host_idx}"),
            MessageLevel::Warn,
        );
        return;
    };

    match update {
        HostUpdate::ContainerList(containers) => {
            host.apply_container_list(containers);
        }

        HostUpdate::StatsUpdate { id, cpu, mem, net_rx, net_tx } => {
            host.apply_stats_update(&id, cpu, mem, net_rx, net_tx);
        }

        HostUpdate::LogLine { id, line } => {
            host.append_log_line(&id, line);
        }

        HostUpdate::StatusChange(status) => {
            host.status = status;
        }
    }  
    }
    
    /* Outbound Commands */
    // dispatch_command()
    // dispatch a 'HostCommand' to the given hosts back-channel
    pub fn dispatch_command(&mut self, host_idx: usize, cmd: HostCommand) {
        if self.safe_mode && cmd.is_destructive() {
            // hold it, show confirmation dialog
            self.set_pending_action(PendingAction { 
                    label: cmd.label().to_string(), 
                    command: cmd, 
                    host_index: host_idx 
            });
            return;
        }
        self.send_command(host_idx, cmd); // only reaches here if safe to proceed
    }
    // confirm_pending_action
    pub fn confirm_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            self.dispatch_command(action.host_index, action.command);
        }
    }
    
    fn send_command(&mut self, host_idx: usize, cmd: HostCommand) {
        if let Some(tx) = self.command_tx.get(host_idx) {
            let _ = tx.try_send(cmd);
        }
    }
    /* Mode Shifts */
    // set_mode
    pub fn set_mode(&mut self, mode: AppMode) {
        if self.mode == AppMode::Command {
            self.command_query.clear();
        }
        self.mode = mode;
    }
    // toggle_safe_mode
    pub fn toggle_safe_mode(&mut self) {
        self.safe_mode = !self.safe_mode;
        let msg = if self.safe_mode {
            "Safe mode ON"
        } else {
            "Safe mode OFF"
        };
        self.set_status(msg.to_string(), MessageLevel::Info);
    }
    // toggle_detail
    pub fn toggle_detail(&mut self) {
        self.show_details = !self.show_details;
    } 
    // set_pending_action
    pub fn set_pending_action(&mut self, action: PendingAction) {
        self.pending_action = Some(action);
    }
    // clear_pending_action
    pub fn clear_pending_action(&mut self) {
        self.pending_action = None;
    }
    // set_status
    pub fn set_status(&mut self, text: String, level: MessageLevel) {
        self.status_messages = Some(StatusMessage {
            text,
            level,
            ttl_ticks: 3,
        });
    }
    // tick_status
    pub fn tick_status(&mut self) {
        if let Some(msg) = &mut self.status_messages {
            msg.decrement();
            if msg.is_expired() {
                self.status_messages = None;
            }
        }
    }
}
