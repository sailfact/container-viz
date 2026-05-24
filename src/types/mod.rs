pub mod container;
pub mod host;
pub mod messaging;
pub mod ui;

pub use container::{ContainerInfo, ContainerState, PortBinding};
pub use host::{ConnectionType, HostStatus, TlsConfig};
pub use messaging::{AppEvent, HostCommand, HostUpdate};
pub use ui::{AppMode, MessageLevel, PendingAction, StatusMessage};

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use ratatui::style::Color;