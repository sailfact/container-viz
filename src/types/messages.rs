use super::*;

#[derive(Debug, Clone)]
pub enum HostUpdate {
    ContainerList(Vec<ContainerInfo>),
    StatsUpdate {
        id: String,
        cpu: f64,
        mem: u64,
        net_rx: u64,
        net_tx: u64,
    },
    LogLine {
        id: String,
        line: String,
    },
    StatusChange(HostStatus),
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Tick,
    Key(crossterm::event::KeyEvent),
    HostUpdate(usize, HostUpdate),
    Resize(u16, u16),
}

#[derive(Debug, Clone)]
pub enum HostCommand {
    StartContainer(String),
    StopContainer(String),
    RestartContainer(String),
    RemoveContainer(String),
    ExecShell(String),
    PullImage(String),
    TailLogs { id: String, lines: u64 },
    Shutdown,
}

pub struct PaletteAction {
    pub label:          String,
    pub description:    String,
    pub command:        HostCommand,
    pub available:      bool,
}

impl HostCommand {
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Self::StopContainer(_)
            | Self::RestartContainer(_)
            | Self::RemoveContainer(_)
        )
    }

    pub fn label(&self) -> &str {
        match self {
            Self::StartContainer(_)     => "Start container",
            Self::StopContainer(_)      => "Stop container",
            Self::RestartContainer(_)   => "Restart container",
            Self::RemoveContainer(_)    => "Remove container",
            Self::ExecShell(_)          => "Exec shell",
            Self::PullImage(_)          => "Pull image",
            Self::TailLogs { .. }       => "Tail logs",
            Self::Shutdown              => "Shutdown",
        }
    }
}