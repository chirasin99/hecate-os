//! HecateOS Configuration Module
//! 
//! Central configuration for all HecateOS services

use serde::{Deserialize, Serialize};
use std::env;

/// HecateOS service ports configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecatePorts {
    /// System monitoring service port (default: 9313)
    pub monitor: u16,
    /// Package manager API port (default: 9314)
    pub pkg_api: u16,
    /// Remote management port (default: 9315)
    pub remote: u16,
    /// Benchmark coordinator port (default: 9316)
    pub bench: u16,
    /// GPU cluster manager port (default: 9317)
    pub gpu: u16,
}

impl Default for HecatePorts {
    fn default() -> Self {
        Self {
            monitor: 9313,  // Mystical number: 93 (IX=9), 13 (mystical)
            pkg_api: 9314,
            remote: 9315,
            bench: 9316,
            gpu: 9317,
        }
    }
}

impl HecatePorts {
    /// Load ports from environment variables
    pub fn from_env() -> Self {
        let mut ports = Self::default();
        
        if let Ok(port) = env::var("HECATE_MONITOR_PORT") {
            if let Ok(p) = port.parse() {
                ports.monitor = p;
            }
        }
        
        if let Ok(port) = env::var("HECATE_PKG_PORT") {
            if let Ok(p) = port.parse() {
                ports.pkg_api = p;
            }
        }
        
        if let Ok(port) = env::var("HECATE_REMOTE_PORT") {
            if let Ok(p) = port.parse() {
                ports.remote = p;
            }
        }
        
        if let Ok(port) = env::var("HECATE_BENCH_PORT") {
            if let Ok(p) = port.parse() {
                ports.bench = p;
            }
        }
        
        if let Ok(port) = env::var("HECATE_GPU_PORT") {
            if let Ok(p) = port.parse() {
                ports.gpu = p;
            }
        }
        
        ports
    }
    
    /// Get monitor service URL
    pub fn monitor_url(&self) -> String {
        format!("http://localhost:{}", self.monitor)
    }
    
    /// Get monitor WebSocket URL
    pub fn monitor_ws_url(&self) -> String {
        format!("ws://localhost:{}/ws", self.monitor)
    }
    
    /// Get package API URL
    pub fn pkg_api_url(&self) -> String {
        format!("http://localhost:{}", self.pkg_api)
    }
}

/// Global configuration for HecateOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HecateConfig {
    pub ports: HecatePorts,
    pub debug: bool,
    pub log_level: String,
}

impl Default for HecateConfig {
    fn default() -> Self {
        Self {
            ports: HecatePorts::default(),
            debug: false,
            log_level: "info".to_string(),
        }
    }
}

impl HecateConfig {
    /// Load configuration from environment
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.ports = HecatePorts::from_env();
        
        if let Ok(debug) = env::var("HECATE_DEBUG") {
            config.debug = debug.to_lowercase() == "true" || debug == "1";
        }
        
        if let Ok(level) = env::var("HECATE_LOG_LEVEL") {
            config.log_level = level;
        }
        
        config
    }
}