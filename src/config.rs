use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::ConnectionType;
// use crate::types::TlsConfig;

// AppConfig
// The top-level configuration object, 
// loaded from ~/.config/container-viz/config.toml. 
// It holds all user preferences and the list of Docker hosts to connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "config", rename_all = "lowercase")]
pub struct AppConfig {
    #[serde(default)]
    pub safe_mode:      bool,
    pub tick_rate:      u64,
    pub log_tail_lines: u64,
    pub hosts:          Vec<HostConfig>,
}

// Represents a single Docker host entry from config — its name and how to connect to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "host", rename_all = "lowercase")]
pub struct HostConfig {
    #[serde(flatten)]
    pub name:       String,
    pub connection: ConnectionType,
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
    // load()
    // Reads and parses the TOML config file from disk, 
    // returning an error if the file is missing or malformed
    pub fn load(&self) -> Result<AppConfig> {
        let path = self.path();
        let contents = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    // save()
    // Writes the current config back to disk, 
    // used after adding/removing hosts
    pub fn save(&self) -> Result<()> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
    
    // add_host()
    // Appends a new HostConfig to the hosts list
    pub fn add_host(&mut self, host: HostConfig) {
        self.hosts.push(host);
    }
    
    // removes_host()
    // Finds and removes a host by name, 
    // returns bool indicating whether it was found
    pub fn remove_host(&mut self, name: &str) -> bool {
        self.hosts.iter().position(|h| h.name == name).map_or(false, |idx| {
            self.hosts.remove(idx);
            true
        })
    }
    
    // path()
    // Returns the resolved filesystem path to the config file
    pub fn path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("container-viz").join("config.toml")
    }
}

impl HostConfig {
    pub fn new(&self, name: String, connection: ConnectionType) -> Self {
        Self { name, connection, }
    }
    //Returns a human-readable label for the tab bar (could be name or derived from connection details)
    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.connection.bollard_addr())
    }
    // Returns true if the connection is a local Unix socket, used to skip TLS logic
    pub fn is_local(&self) -> bool {
        self.connection.is_local()
    }
}