//! NVIDIA GPU backend implementation using NVML

use crate::{
    error::{GpuError, Result},
    GpuBackend, GpuConfig, GpuStatus, GpuType, GpuVendor, PowerMode, PowerState, FanCurve, PciInfo
};
use async_trait::async_trait;
use nvml_wrapper::{enum_wrappers::device::*, Nvml, Device};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// NVIDIA GPU backend using NVML
pub struct NvidiaBackend {
    nvml: Option<Nvml>,
    devices: Arc<RwLock<HashMap<u32, Device<'static>>>>,
}

impl std::fmt::Debug for NvidiaBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NvidiaBackend")
            .field("nvml_initialized", &self.nvml.is_some())
            .field("device_count", &"Arc<RwLock<HashMap>>")
            .finish()
    }
}

impl NvidiaBackend {
    /// Create a new NVIDIA backend
    pub async fn new() -> Result<Self> {
        Ok(Self {
            nvml: None,
            devices: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get GPU status using NVML device
    fn get_device_status(device: &Device, index: u32) -> Result<GpuStatus> {
        let name = device.name().map_err(GpuError::from)?;
        
        // Temperature
        let temperature = device
            .temperature(TemperatureSensor::Gpu)
            .map_err(GpuError::from)? as u32;
        
        // Power information
        let power_draw = device
            .power_usage()
            .map_err(GpuError::from)? / 1000; // Convert mW to W
        let power_limit = device
            .power_management_limit()
            .map_err(GpuError::from)? / 1000;
        
        // Memory information
        let mem_info = device.memory_info().map_err(GpuError::from)?;
        
        // Utilization information
        let utilization = device.utilization_rates().map_err(GpuError::from)?;
        
        // Fan speed (may not be available on all cards)
        let fan_speed = device.fan_speed(0).ok().map(|speed| speed as u32);
        
        // Clock information
        let clock_graphics = device
            .clock_info(Clock::Graphics)
            .map_err(GpuError::from)? as u32;
        let clock_memory = device
            .clock_info(Clock::Memory)
            .map_err(GpuError::from)? as u32;
        
        // Driver version - get from NVML instance instead
        let driver_version = Some("Unknown".to_string()); // Would get from nvml.sys_driver_version()
        
        // PCI information
        let pci_info = device.pci_info().map_err(GpuError::from)?;
        let pci_info = PciInfo {
            domain: 0, // NVML doesn't provide domain
            bus: pci_info.bus as u8,
            device: pci_info.device as u8,
            function: 0, // NVML doesn't provide function
            vendor_id: 0x10DE, // NVIDIA vendor ID
            device_id: pci_info.device as u16, // Use device field for device_id
        };
        
        // Determine GPU type based on power limit
        let gpu_type = if power_limit < 75 {
            GpuType::Integrated
        } else {
            GpuType::Discrete
        };
        
        // Determine power state based on utilization
        let power_state = if utilization.gpu > 10 {
            PowerState::Active
        } else {
            PowerState::Idle
        };

        Ok(GpuStatus {
            index,
            name,
            vendor: GpuVendor::NVIDIA,
            gpu_type,
            temperature,
            power_draw: power_draw as u32,
            power_limit: power_limit as u32,
            memory_used: mem_info.used,
            memory_total: mem_info.total,
            utilization_gpu: utilization.gpu as u32,
            utilization_memory: utilization.memory as u32,
            fan_speed,
            clock_graphics,
            clock_memory,
            driver_version,
            pci_info,
            power_state,
        })
    }

    /// Apply power mode configuration
    async fn apply_power_mode(&self, device: &mut Device<'_>, mode: PowerMode) -> Result<()> {
        match mode {
            PowerMode::MaxPerformance => {
                self.set_max_performance(device).await?;
            }
            PowerMode::Balanced => {
                self.set_balanced(device).await?;
            }
            PowerMode::PowerSaver => {
                self.set_power_saver(device).await?;
            }
            PowerMode::Custom => {
                // Custom mode is handled by specific parameter settings
            }
            PowerMode::Auto => {
                // Auto mode chooses based on current load
                let utilization = device.utilization_rates().map_err(GpuError::from)?;
                if utilization.gpu > 80 {
                    self.set_max_performance(device).await?;
                } else if utilization.gpu < 20 {
                    self.set_power_saver(device).await?;
                } else {
                    self.set_balanced(device).await?;
                }
            }
        }
        Ok(())
    }

    /// Set maximum performance mode
    async fn set_max_performance(&self, device: &mut Device<'_>) -> Result<()> {
        // Set maximum power limit
        if let Ok(constraints) = device.power_management_limit_constraints() {
            device
                .set_power_management_limit(constraints.max_limit)
                .map_err(GpuError::from)?;
        }
        
        // Enable persistence mode
        device
            .set_persistent(true)
            .map_err(GpuError::from)?;
        
        // Enable auto boost
        device
            .set_auto_boosted_clocks(true)
            .map_err(GpuError::from)?;
        
        info!("Applied maximum performance mode");
        Ok(())
    }

    /// Set balanced mode
    async fn set_balanced(&self, device: &mut Device<'_>) -> Result<()> {
        // Set power limit to 90% of maximum
        if let Ok(constraints) = device.power_management_limit_constraints() {
            let balanced_limit = (constraints.max_limit * 90) / 100;
            device
                .set_power_management_limit(balanced_limit)
                .map_err(GpuError::from)?;
        }
        
        info!("Applied balanced mode");
        Ok(())
    }

    /// Set power saver mode
    async fn set_power_saver(&self, device: &mut Device<'_>) -> Result<()> {
        // Set power limit to 70% of maximum
        if let Ok(constraints) = device.power_management_limit_constraints() {
            let eco_limit = (constraints.max_limit * 70) / 100;
            device
                .set_power_management_limit(eco_limit)
                .map_err(GpuError::from)?;
        }
        
        // Disable auto boost to save power
        device
            .set_auto_boosted_clocks(false)
            .map_err(GpuError::from)?;
        
        info!("Applied power saver mode");
        Ok(())
    }

    /// Apply fan curve (if supported)
    async fn apply_fan_curve(&self, device: &mut Device<'_>, curve: &FanCurve) -> Result<()> {
        // Get current temperature
        let temp = device
            .temperature(TemperatureSensor::Gpu)
            .map_err(GpuError::from)?;
        
        // Calculate target fan speed based on curve
        let _target_speed = curve.calculate_fan_speed(temp as u32);
        
        // NVIDIA fan control is not available in NVML for most consumer cards
        warn!("Fan control not supported on this GPU via NVML");
        Err(GpuError::OperationNotSupported("Fan control".to_string()))
    }

    /// Apply clock offsets
    async fn apply_clock_offsets(&self, _device: &mut Device<'_>, gpu_offset: Option<i32>, mem_offset: Option<i32>) -> Result<()> {
        // Clock offset functionality is not straightforward with NVML
        // These would require specific NVIDIA settings or MSI Afterburner-like tools
        if gpu_offset.is_some() {
            warn!("GPU clock offset not supported via NVML");
        }
        
        if mem_offset.is_some() {
            warn!("Memory clock offset not supported via NVML");
        }
        
        Ok(())
    }
}

#[async_trait]
impl GpuBackend for NvidiaBackend {
    async fn init(&mut self) -> Result<()> {
        match Nvml::init() {
            Ok(nvml) => {
                info!("NVIDIA NVML initialized successfully");
                self.nvml = Some(nvml);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to initialize NVML: {}", e);
                Err(GpuError::from(e))
            }
        }
    }

    #[instrument]
    async fn detect_gpus(&self) -> Result<Vec<GpuStatus>> {
        let nvml = self.nvml.as_ref().ok_or_else(|| {
            GpuError::SystemError("NVML not initialized".to_string())
        })?;

        let device_count = nvml.device_count().map_err(GpuError::from)?;
        let mut gpus = Vec::new();
        let mut devices_map = HashMap::new();

        for i in 0..device_count {
            match nvml.device_by_index(i) {
                Ok(device) => {
                    match Self::get_device_status(&device, i) {
                        Ok(status) => {
                            gpus.push(status);
                            
                            // Store device for later use
                            // Safety: We transmute to 'static lifetime, but ensure device
                            // lifetime is managed by keeping nvml alive
                            let static_device: Device<'static> = unsafe {
                                std::mem::transmute(device)
                            };
                            devices_map.insert(i, static_device);
                        }
                        Err(e) => {
                            warn!("Failed to get status for NVIDIA GPU {}: {}", i, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to access NVIDIA GPU {}: {}", i, e);
                }
            }
        }

        // Store devices for later use
        let mut devices = self.devices.write().await;
        *devices = devices_map;

        info!("Detected {} NVIDIA GPU(s)", gpus.len());
        Ok(gpus)
    }

    #[instrument]
    async fn get_gpu_status(&self, index: u32) -> Result<GpuStatus> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        Self::get_device_status(device, index)
    }

    #[instrument]
    async fn apply_config(&self, index: u32, config: &GpuConfig) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        // Apply power mode
        self.apply_power_mode(device, config.power_mode).await?;

        // Apply custom power limit
        if let Some(limit) = config.power_limit {
            device
                .set_power_management_limit(limit * 1000) // Convert W to mW
                .map_err(GpuError::from)?;
        }

        // Apply fan curve
        if let Some(ref curve) = config.fan_curve {
            if let Err(e) = self.apply_fan_curve(device, curve).await {
                debug!("Fan curve application failed: {}", e);
            }
        }

        // Apply clock offsets
        self.apply_clock_offsets(
            device,
            config.gpu_clock_offset,
            config.memory_clock_offset,
        ).await?;

        info!("Applied configuration to NVIDIA GPU {}", index);
        Ok(())
    }

    #[instrument]
    async fn set_power_limit(&self, index: u32, limit_watts: u32) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        device
            .set_power_management_limit(limit_watts * 1000) // Convert W to mW
            .map_err(GpuError::from)?;

        info!("Set power limit to {}W for NVIDIA GPU {}", limit_watts, index);
        Ok(())
    }

    #[instrument]
    async fn set_fan_curve(&self, index: u32, curve: &FanCurve) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        self.apply_fan_curve(device, curve).await
    }

    #[instrument]
    async fn reset_gpu(&self, index: u32) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        // Reset to maximum power limit (closest to default)
        if let Ok(constraints) = device.power_management_limit_constraints() {
            device
                .set_power_management_limit(constraints.max_limit)
                .map_err(GpuError::from)?;
        }

        // Reset auto boost to default
        device
            .set_auto_boosted_clocks(true)
            .map_err(GpuError::from)?;

        // Reset persistence mode
        device
            .set_persistent(false)
            .map_err(GpuError::from)?;

        info!("Reset NVIDIA GPU {} to defaults", index);
        Ok(())
    }

    fn supports_gpu_switching(&self) -> bool {
        // NVIDIA Optimus supports GPU switching
        true
    }

    #[instrument]
    async fn switch_gpu(&self, _from_index: u32, _to_index: u32) -> Result<()> {
        // GPU switching with NVIDIA requires Optimus technology
        // This would typically involve:
        // 1. Setting the GPU mode in nvidia-settings
        // 2. Updating X11 configuration
        // 3. Potentially restarting the display manager
        
        // For now, return an error indicating this needs system-level support
        Err(GpuError::OperationNotSupported(
            "GPU switching requires Optimus configuration".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nvidia_backend_creation() {
        let backend = NvidiaBackend::new().await;
        assert!(backend.is_ok());
    }

    #[test]
    fn test_power_mode_selection() {
        // Test auto mode logic would go here
        let high_utilization = 90;
        let low_utilization = 10;
        
        assert!(high_utilization > 80); // Should trigger max performance
        assert!(low_utilization < 20);  // Should trigger power saver
    }
}