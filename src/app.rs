use std::path::PathBuf;

pub struct AppConfig {
    pub safe_mode: bool,
    pub tick_rate: u64,
    pub log_tail_lines: u64,
    pub hosts: Vec<HostConfig>,
}

pub struct HostConfig {
    pub name: String,
    pub connection: ConnectionType,
}

pub struct ConnectionType {
    pub host: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub ca_cert: Option<PathBuf>,
    pub verify: bool,
}

impl AppConfig {
    pub fn new(safe_mode: bool, tick_rate: u64, log_tail_lines: u64, hosts: Vec<HostConfig>,) -> Self {
        Self {
            safe_mode,
            tick_rate,
            log_tail_lines,
            hosts,
        }
    }

    pub fn load(&self) -> Result<AppConfig> {
        // Todo
    }

    pub fn save(&self) -> Result<()> {
        
    }
}
