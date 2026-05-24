use super::{PathBuf, Color, Serialize, Deserialize};

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