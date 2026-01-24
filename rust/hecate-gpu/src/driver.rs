//! GPU driver management and automatic updates

use crate::error::{GpuError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command as AsyncCommand;
use tracing::{info, instrument, warn};
use which::which;

/// GPU driver manager for automatic updates and version management
#[derive(Debug)]
pub struct DriverManager {
    nvidia_driver: Option<NvidiaDriver>,
    amd_driver: Option<AmdDriver>,
}

impl DriverManager {
    /// Create a new driver manager
    pub fn new() -> Self {
        Self {
            nvidia_driver: None,
            amd_driver: None,
        }
    }

    /// Initialize driver detection
    pub async fn init(&mut self) -> Result<()> {
        // Detect NVIDIA driver
        if let Ok(nvidia) = NvidiaDriver::new().await {
            self.nvidia_driver = Some(nvidia);
        }

        // Detect AMD driver
        if let Ok(amd) = AmdDriver::new().await {
            self.amd_driver = Some(amd);
        }

        Ok(())
    }

    /// Check for available driver updates
    #[instrument]
    pub async fn check_updates(&self) -> Result<Vec<DriverUpdate>> {
        let mut updates = Vec::new();

        if let Some(ref nvidia) = self.nvidia_driver {
            if let Ok(update) = nvidia.check_update().await {
                if let Some(update) = update {
                    updates.push(update);
                }
            }
        }

        if let Some(ref amd) = self.amd_driver {
            if let Ok(update) = amd.check_update().await {
                if let Some(update) = update {
                    updates.push(update);
                }
            }
        }

        Ok(updates)
    }

    /// Check and automatically update drivers
    #[instrument]
    pub async fn check_and_update_drivers(&self) -> Result<Vec<String>> {
        let mut updated_drivers = Vec::new();

        // Check NVIDIA driver updates
        if let Some(ref nvidia) = self.nvidia_driver {
            if let Ok(Some(update)) = nvidia.check_update().await {
                info!("NVIDIA driver update available: {} -> {}", update.current_version, update.latest_version);
                
                if let Ok(()) = nvidia.update_driver().await {
                    updated_drivers.push(format!("NVIDIA: {} -> {}", update.current_version, update.latest_version));
                }
            }
        }

        // Check AMD driver updates
        if let Some(ref amd) = self.amd_driver {
            if let Ok(Some(update)) = amd.check_update().await {
                info!("AMD driver update available: {} -> {}", update.current_version, update.latest_version);
                
                if let Ok(()) = amd.update_driver().await {
                    updated_drivers.push(format!("AMD: {} -> {}", update.current_version, update.latest_version));
                }
            }
        }

        Ok(updated_drivers)
    }

    /// Get current driver versions
    pub async fn get_driver_versions(&self) -> HashMap<String, String> {
        let mut versions = HashMap::new();

        if let Some(ref nvidia) = self.nvidia_driver {
            if let Ok(version) = nvidia.get_current_version().await {
                versions.insert("nvidia".to_string(), version);
            }
        }

        if let Some(ref amd) = self.amd_driver {
            if let Ok(version) = amd.get_current_version().await {
                versions.insert("amd".to_string(), version);
            }
        }

        versions
    }
}

/// Driver update information
#[derive(Debug, Clone)]
pub struct DriverUpdate {
    pub vendor: String,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub critical: bool,
}

/// NVIDIA driver manager
#[derive(Debug)]
struct NvidiaDriver {
    current_version: Option<String>,
}

impl NvidiaDriver {
    async fn new() -> Result<Self> {
        let current_version = Self::detect_current_version().await?;
        Ok(Self {
            current_version: Some(current_version),
        })
    }

    async fn detect_current_version() -> Result<String> {
        // Try nvidia-smi first
        if let Ok(version) = Self::get_version_from_nvidia_smi().await {
            return Ok(version);
        }

        // Try modinfo
        if let Ok(version) = Self::get_version_from_modinfo().await {
            return Ok(version);
        }

        // Try dpkg (Ubuntu/Debian)
        if let Ok(version) = Self::get_version_from_dpkg().await {
            return Ok(version);
        }

        Err(GpuError::DriverNotFound("NVIDIA driver not found".to_string()))
    }

    async fn get_version_from_nvidia_smi() -> Result<String> {
        if which("nvidia-smi").is_err() {
            return Err(GpuError::DriverNotFound("nvidia-smi not found".to_string()));
        }

        let output = AsyncCommand::new("nvidia-smi")
            .arg("--query-gpu=driver_version")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await
            .map_err(GpuError::IoError)?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Ok(version);
            }
        }

        Err(GpuError::DriverNotFound("Failed to get NVIDIA driver version".to_string()))
    }

    async fn get_version_from_modinfo() -> Result<String> {
        let output = AsyncCommand::new("modinfo")
            .arg("nvidia")
            .output()
            .await
            .map_err(GpuError::IoError)?;

        if output.status.success() {
            let modinfo_output = String::from_utf8_lossy(&output.stdout);
            
            for line in modinfo_output.lines() {
                if line.starts_with("version:") {
                    let version = line.replace("version:", "").trim().to_string();
                    return Ok(version);
                }
            }
        }

        Err(GpuError::DriverNotFound("NVIDIA driver version not found in modinfo".to_string()))
    }

    async fn get_version_from_dpkg() -> Result<String> {
        let output = AsyncCommand::new("dpkg")
            .arg("-l")
            .arg("nvidia-driver-*")
            .output()
            .await
            .map_err(GpuError::IoError)?;

        if output.status.success() {
            let dpkg_output = String::from_utf8_lossy(&output.stdout);
            let re = Regex::new(r"nvidia-driver-(\d+)").unwrap();
            
            for line in dpkg_output.lines() {
                if line.contains("ii") {
                    if let Some(caps) = re.captures(line) {
                        return Ok(caps[1].to_string());
                    }
                }
            }
        }

        Err(GpuError::DriverNotFound("NVIDIA driver not found in dpkg".to_string()))
    }

    async fn get_current_version(&self) -> Result<String> {
        self.current_version
            .clone()
            .ok_or_else(|| GpuError::DriverNotFound("NVIDIA driver version unknown".to_string()))
    }

    async fn check_update(&self) -> Result<Option<DriverUpdate>> {
        let current_version = self.get_current_version().await?;
        
        // For now, we'll implement a basic check
        // In a real implementation, this would check NVIDIA's servers
        let latest_version = Self::get_latest_nvidia_version().await?;
        
        if current_version != latest_version {
            return Ok(Some(DriverUpdate {
                vendor: "NVIDIA".to_string(),
                current_version,
                latest_version,
                download_url: None,
                critical: false,
            }));
        }
        
        Ok(None)
    }

    async fn get_latest_nvidia_version() -> Result<String> {
        // This would typically query NVIDIA's API or scrape their website
        // For now, return a placeholder
        Ok("525.105.17".to_string())
    }

    async fn update_driver(&self) -> Result<()> {
        // Check if we're on Ubuntu and can use apt
        if Path::new("/usr/bin/apt").exists() {
            self.update_nvidia_ubuntu().await
        } else {
            Err(GpuError::OperationNotSupported("Automatic NVIDIA driver updates only supported on Ubuntu".to_string()))
        }
    }

    async fn update_nvidia_ubuntu(&self) -> Result<()> {
        info!("Updating NVIDIA driver on Ubuntu");
        
        // Add NVIDIA PPA if not present
        let add_ppa = AsyncCommand::new("sudo")
            .arg("add-apt-repository")
            .arg("-y")
            .arg("ppa:graphics-drivers/ppa")
            .status()
            .await
            .map_err(GpuError::IoError)?;

        if !add_ppa.success() {
            warn!("Failed to add NVIDIA PPA");
        }

        // Update package list
        let update_status = AsyncCommand::new("sudo")
            .arg("apt")
            .arg("update")
            .status()
            .await
            .map_err(GpuError::IoError)?;

        if !update_status.success() {
            return Err(GpuError::SystemError("Failed to update package list".to_string()));
        }

        // Install latest driver
        let install_status = AsyncCommand::new("sudo")
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg("nvidia-driver-525")
            .status()
            .await
            .map_err(GpuError::IoError)?;

        if !install_status.success() {
            return Err(GpuError::SystemError("Failed to install NVIDIA driver".to_string()));
        }

        info!("NVIDIA driver updated successfully");
        Ok(())
    }
}

/// AMD driver manager
#[derive(Debug)]
struct AmdDriver {
    current_version: Option<String>,
}

impl AmdDriver {
    async fn new() -> Result<Self> {
        let current_version = Self::detect_current_version().await.ok();
        Ok(Self {
            current_version,
        })
    }

    async fn detect_current_version() -> Result<String> {
        // Try modinfo for amdgpu
        if let Ok(version) = Self::get_version_from_modinfo().await {
            return Ok(version);
        }

        // Try dpkg (Ubuntu/Debian)
        if let Ok(version) = Self::get_version_from_dpkg().await {
            return Ok(version);
        }

        Err(GpuError::DriverNotFound("AMD driver not found".to_string()))
    }

    async fn get_version_from_modinfo() -> Result<String> {
        let output = AsyncCommand::new("modinfo")
            .arg("amdgpu")
            .output()
            .await
            .map_err(GpuError::IoError)?;

        if output.status.success() {
            let modinfo_output = String::from_utf8_lossy(&output.stdout);
            
            for line in modinfo_output.lines() {
                if line.starts_with("version:") {
                    let version = line.replace("version:", "").trim().to_string();
                    return Ok(version);
                }
            }
        }

        Err(GpuError::DriverNotFound("AMD driver version not found in modinfo".to_string()))
    }

    async fn get_version_from_dpkg() -> Result<String> {
        // Check for mesa drivers
        let output = AsyncCommand::new("dpkg")
            .arg("-l")
            .arg("libdrm-amdgpu1")
            .output()
            .await
            .map_err(GpuError::IoError)?;

        if output.status.success() {
            let dpkg_output = String::from_utf8_lossy(&output.stdout);
            
            for line in dpkg_output.lines() {
                if line.contains("ii") && line.contains("libdrm-amdgpu1") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        return Ok(parts[2].to_string());
                    }
                }
            }
        }

        Err(GpuError::DriverNotFound("AMD driver not found in dpkg".to_string()))
    }

    async fn get_current_version(&self) -> Result<String> {
        self.current_version
            .clone()
            .ok_or_else(|| GpuError::DriverNotFound("AMD driver version unknown".to_string()))
    }

    async fn check_update(&self) -> Result<Option<DriverUpdate>> {
        if let Ok(current_version) = self.get_current_version().await {
            // For AMD, updates usually come through kernel/mesa updates
            let latest_version = Self::get_latest_amd_version().await?;
            
            if current_version != latest_version {
                return Ok(Some(DriverUpdate {
                    vendor: "AMD".to_string(),
                    current_version,
                    latest_version,
                    download_url: None,
                    critical: false,
                }));
            }
        }
        
        Ok(None)
    }

    async fn get_latest_amd_version() -> Result<String> {
        // This would typically check for kernel/mesa updates
        // For now, return a placeholder
        Ok("6.1.0".to_string())
    }

    async fn update_driver(&self) -> Result<()> {
        // AMD drivers are typically updated through system updates
        if Path::new("/usr/bin/apt").exists() {
            self.update_amd_ubuntu().await
        } else {
            Err(GpuError::OperationNotSupported("Automatic AMD driver updates only supported on Ubuntu".to_string()))
        }
    }

    async fn update_amd_ubuntu(&self) -> Result<()> {
        info!("Updating AMD driver components on Ubuntu");
        
        // Update mesa and kernel components
        let update_status = AsyncCommand::new("sudo")
            .arg("apt")
            .arg("update")
            .status()
            .await
            .map_err(GpuError::IoError)?;

        if !update_status.success() {
            return Err(GpuError::SystemError("Failed to update package list".to_string()));
        }

        // Upgrade mesa drivers
        let upgrade_status = AsyncCommand::new("sudo")
            .arg("apt")
            .arg("install")
            .arg("-y")
            .arg("mesa-vulkan-drivers")
            .arg("libdrm-amdgpu1")
            .arg("xserver-xorg-video-amdgpu")
            .status()
            .await
            .map_err(GpuError::IoError)?;

        if !upgrade_status.success() {
            return Err(GpuError::SystemError("Failed to upgrade AMD drivers".to_string()));
        }

        info!("AMD driver components updated successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_driver_manager_creation() {
        let manager = DriverManager::new();
        assert!(manager.nvidia_driver.is_none());
        assert!(manager.amd_driver.is_none());
    }

    #[test]
    fn test_driver_update_structure() {
        let update = DriverUpdate {
            vendor: "NVIDIA".to_string(),
            current_version: "470.86".to_string(),
            latest_version: "525.105.17".to_string(),
            download_url: None,
            critical: false,
        };
        
        assert_eq!(update.vendor, "NVIDIA");
        assert!(!update.critical);
    }
}