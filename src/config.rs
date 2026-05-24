use std::path::PathBuf;
use anyhow::Result;

use crate::types::ConnectionType;
// use crate::types::TlsConfig;

// AppConfig
// The top-level configuration object, 
// loaded from ~/.config/container-viz/config.toml. 
// It holds all user preferences and the list of Docker hosts to connect to.
pub struct AppConfig {
    pub safe_mode: bool,
    pub tick_rate: u64,
    pub log_tail_lines: u64,
    pub hosts: Vec<HostConfig>,
}

// Represents a single Docker host entry from config — its name and how to connect to it.
pub struct HostConfig {
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
        todo!()
    }
    // save()
    // Writes the current config back to disk, 
    // used after adding/removing hosts
    pub fn save(&self) -> Result<()> {
        todo!()
    }
    
    // add_host()
    // Appends a new HostConfig to the hosts list
    pub fn add_host(&self, host: HostConfig) {
        todo!()
    }
    
    // removes_host()
    // Finds and removes a host by name, 
    // returns bool indicating whether it was found
    pub fn remove_host(&self, name: &str) {
        todo!()
    }
    
    // path()
    // Returns the resolved filesystem path to the config file
    pub fn path(&self) -> PathBuf {
        todo!()
    }
}

impl HostConfig {
    pub fn new(&self, name: String, connection: ConnectionType) -> Self {
        Self { name, connection, }
    }
    //Returns a human-readable label for the tab bar (could be name or derived from connection details)
    pub fn display_name(&self) -> String {
        todo!()
    }
    // Returns true if the connection is a local Unix socket, used to skip TLS logic
    pub fn is_local(&self) -> bool {
        todo!()
    }
}