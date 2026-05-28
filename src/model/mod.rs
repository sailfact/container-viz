pub mod container;
pub mod host;
pub mod messages;
pub mod app;

pub use container::{ContainerInfo, ContainerState, PortBinding};
pub use host::{HostConfig, ConnectionType, TlsConfig, HostState, HostStatus};
pub use messages::{AppEvent, HostCommand, HostUpdate, PaletteAction};
pub use app::{AppMode, MessageLevel, PendingAction, StatusMessage};

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use ratatui::style::Color;