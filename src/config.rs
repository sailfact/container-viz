// config.rs
// Implements AppConfig and HostConfig
// AppConfig is the top-level configuration object, loaded from ~/.config/container-viz/config.tom
use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::HostConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "config", rename_all = "lowercase")]
pub struct AppConfig {
    #[serde(default = "default_safe_mode")]
    pub safe_mode: bool,

    #[serde(default = "default_tick_rate")]
    pub tick_rate: u64,
    
    #[serde(default = "default_log_tail_lines")]
    pub log_tail_lines: u64,
    
    #[serde(default)]
    pub hosts: Vec<HostConfig>,
}

impl AppConfig {
    // Reads and parses the TOML config file from disk, 
    // returning an error if the file is missing or malformed
    pub fn load(&self) -> Result<AppConfig> {
        let path = self.path();
        let contents = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    
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
    
    // Appends a new HostConfig to the hosts list
    pub fn add_host(&mut self, host: HostConfig) {
        self.hosts.push(host);
    }
    
    // Finds and removes a host by name, 
    // returns bool indicating whether it was found
    pub fn remove_host(&mut self, name: &str) -> bool {
        let before = self.hosts.len();
        self.hosts.retain(|h| h.name != name);
        self.hosts.len() < before
    }
    
    // Returns the resolved filesystem path to the config file
    pub fn path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config").join("container-viz").join("config.toml")
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            safe_mode: default_safe_mode(),
            tick_rate: default_tick_rate(),
            log_tail_lines: default_log_tail_lines(),
            hosts: Vec::new(),
        }
    }
}

fn default_safe_mode() -> bool {
    true
}
 
fn default_tick_rate() -> u64 {
    1000
}
 
fn default_log_tail_lines() -> u64 {
    100
}