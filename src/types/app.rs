use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum AppMode {
    Normal,
    Logs,
    Command,
    HostManager,
    Help,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MessageLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct PendingAction {
    pub label:      String,
    pub command:    HostCommand,
    pub host_index: usize,
}
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

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: MessageLevel,
    pub ttl_ticks: u8,
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

impl StatusMessage {
    pub fn new(text: String, level: MessageLevel) -> Self {
        Self { text, level, ttl_ticks: 3 }  // 3 ticks matches spec's 3s auto-dismiss
    }

    pub fn is_expired(&self) -> bool {
        self.ttl_ticks == 0
    }

    pub fn decrement(&mut self) {
        self.ttl_ticks = self.ttl_ticks.saturating_sub(1);
    }
}