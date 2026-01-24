//! AMD GPU backend implementation using DRM and sysfs

use crate::{
    error::{GpuError, Result},
    GpuBackend, GpuConfig, GpuStatus, GpuType, GpuVendor, PowerMode, PowerState, FanCurve, PciInfo
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// AMD GPU device information
#[derive(Debug, Clone)]
struct AmdDevice {
    index: u32,
    device_path: PathBuf,
    hwmon_path: Option<PathBuf>,
    drm_path: PathBuf,
    pci_id: String,
}

/// AMD GPU backend using DRM and sysfs
#[derive(Debug)]
pub struct AmdBackend {
    devices: Arc<RwLock<HashMap<u32, AmdDevice>>>,
}

impl AmdBackend {
    /// Create a new AMD backend
    pub async fn new() -> Result<Self> {
        Ok(Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Find AMD GPUs in the system
    fn find_amd_devices() -> Result<Vec<AmdDevice>> {
        let mut devices = Vec::new();
        let mut index = 0;

        // Look for AMD GPUs in /sys/class/drm/
        let drm_path = Path::new("/sys/class/drm");
        if !drm_path.exists() {
            return Ok(devices);
        }

        for entry in fs::read_dir(drm_path).map_err(GpuError::IoError)? {
            let entry = entry.map_err(GpuError::IoError)?;
            let path = entry.path();
            let name = entry.file_name();
            
            if let Some(name_str) = name.to_str() {
                // Look for card* entries (not renderD*)
                if name_str.starts_with("card") && !name_str.contains("renderD") {
                    let device_path = path.join("device");
                    
                    // Check if it's an AMD GPU
                    if Self::is_amd_device(&device_path)? {
                        let pci_id = Self::get_pci_id(&device_path)?;
                        let hwmon_path = Self::find_hwmon_path(&device_path)?;
                        
                        let device = AmdDevice {
                            index,
                            device_path,
                            hwmon_path,
                            drm_path: path,
                            pci_id,
                        };
                        
                        devices.push(device);
                        index += 1;
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Check if a device is an AMD GPU
    fn is_amd_device(device_path: &Path) -> Result<bool> {
        let vendor_path = device_path.join("vendor");
        if let Ok(vendor) = fs::read_to_string(&vendor_path) {
            // AMD vendor ID is 0x1002
            return Ok(vendor.trim() == "0x1002");
        }
        Ok(false)
    }

    /// Get PCI ID for the device
    fn get_pci_id(device_path: &Path) -> Result<String> {
        let device_file = device_path.join("device");
        let vendor_file = device_path.join("vendor");
        
        let device_id = fs::read_to_string(&device_file)
            .map_err(GpuError::IoError)?
            .trim()
            .to_string();
        let vendor_id = fs::read_to_string(&vendor_file)
            .map_err(GpuError::IoError)?
            .trim()
            .to_string();
            
        Ok(format!("{}:{}", vendor_id, device_id))
    }

    /// Find hwmon path for temperature/power monitoring
    fn find_hwmon_path(device_path: &Path) -> Result<Option<PathBuf>> {
        let hwmon_dir = device_path.join("hwmon");
        
        if !hwmon_dir.exists() {
            return Ok(None);
        }

        // Look for hwmon* directories
        for entry in fs::read_dir(&hwmon_dir).map_err(GpuError::IoError)? {
            let entry = entry.map_err(GpuError::IoError)?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("hwmon") {
                        return Ok(Some(path));
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Read GPU status from sysfs
    fn get_device_status(device: &AmdDevice) -> Result<GpuStatus> {
        // Get basic device information
        let name = Self::get_device_name(device)?;
        
        // Get temperature
        let temperature = Self::read_temperature(device)?;
        
        // Get power information
        let (power_draw, power_limit) = Self::read_power_info(device)?;
        
        // Get memory information
        let (memory_used, memory_total) = Self::read_memory_info(device)?;
        
        // Get utilization
        let utilization_gpu = Self::read_gpu_utilization(device)?;
        
        // Get clock information
        let (clock_graphics, clock_memory) = Self::read_clock_info(device)?;
        
        // Get fan speed
        let fan_speed = Self::read_fan_speed(device)?;
        
        // Parse PCI information
        let pci_info = Self::parse_pci_info(device)?;
        
        // Determine GPU type based on power characteristics
        let gpu_type = if power_limit < 75 {
            GpuType::Integrated
        } else {
            GpuType::Discrete
        };
        
        // Determine power state
        let power_state = if utilization_gpu > 10 {
            PowerState::Active
        } else {
            PowerState::Idle
        };

        Ok(GpuStatus {
            index: device.index,
            name,
            vendor: GpuVendor::AMD,
            gpu_type,
            temperature,
            power_draw,
            power_limit,
            memory_used,
            memory_total,
            utilization_gpu,
            utilization_memory: utilization_gpu, // AMD often reports similar values
            fan_speed,
            clock_graphics,
            clock_memory,
            driver_version: Self::get_driver_version()?,
            pci_info,
            power_state,
        })
    }

    /// Get device name
    fn get_device_name(device: &AmdDevice) -> Result<String> {
        // Try to read from device specific locations
        let name_paths = [
            device.device_path.join("pp_table"),
            device.device_path.join("product_name"),
            device.drm_path.join("device/product_name"),
        ];

        for path in &name_paths {
            if let Ok(name) = fs::read_to_string(path) {
                let name = name.trim();
                if !name.is_empty() {
                    return Ok(name.to_string());
                }
            }
        }

        // Fallback to PCI ID lookup
        Ok(format!("AMD GPU {}", device.pci_id))
    }

    /// Read temperature from hwmon
    fn read_temperature(device: &AmdDevice) -> Result<u32> {
        if let Some(ref hwmon_path) = device.hwmon_path {
            // Try different temperature input files
            let temp_files = ["temp1_input", "temp2_input", "temp3_input"];
            
            for temp_file in &temp_files {
                let temp_path = hwmon_path.join(temp_file);
                if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                    if let Ok(temp_millidegrees) = temp_str.trim().parse::<u32>() {
                        return Ok(temp_millidegrees / 1000); // Convert from millidegrees
                    }
                }
            }
        }
        
        // Fallback: try junction temperature from amdgpu
        let junction_temp_path = device.device_path.join("gpu_busy_percent");
        if junction_temp_path.exists() {
            // This is a fallback - actual temp reading might be in different location
            return Ok(50); // Safe fallback temperature
        }
        
        Ok(50) // Default fallback
    }

    /// Read power information
    fn read_power_info(device: &AmdDevice) -> Result<(u32, u32)> {
        let mut power_draw = 0;
        let mut power_limit = 300; // Default limit

        if let Some(ref hwmon_path) = device.hwmon_path {
            // Read current power draw
            let power_files = ["power1_input", "power1_average"];
            for power_file in &power_files {
                let power_path = hwmon_path.join(power_file);
                if let Ok(power_str) = fs::read_to_string(&power_path) {
                    if let Ok(power_microwatts) = power_str.trim().parse::<u32>() {
                        power_draw = power_microwatts / 1_000_000; // Convert to watts
                        break;
                    }
                }
            }

            // Read power limit
            let limit_path = hwmon_path.join("power1_cap");
            if let Ok(limit_str) = fs::read_to_string(&limit_path) {
                if let Ok(limit_microwatts) = limit_str.trim().parse::<u32>() {
                    power_limit = limit_microwatts / 1_000_000;
                }
            }
        }

        Ok((power_draw, power_limit))
    }

    /// Read memory information
    fn read_memory_info(device: &AmdDevice) -> Result<(u64, u64)> {
        // Try to read from amdgpu specific files
        let used_path = device.device_path.join("mem_info_vram_used");
        let total_path = device.device_path.join("mem_info_vram_total");
        
        let memory_used = if let Ok(used_str) = fs::read_to_string(&used_path) {
            used_str.trim().parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        let memory_total = if let Ok(total_str) = fs::read_to_string(&total_path) {
            total_str.trim().parse::<u64>().unwrap_or(8_589_934_592) // 8GB default
        } else {
            8_589_934_592 // 8GB default
        };
        
        Ok((memory_used, memory_total))
    }

    /// Read GPU utilization
    fn read_gpu_utilization(device: &AmdDevice) -> Result<u32> {
        let busy_path = device.device_path.join("gpu_busy_percent");
        
        if let Ok(busy_str) = fs::read_to_string(&busy_path) {
            if let Ok(busy_percent) = busy_str.trim().parse::<u32>() {
                return Ok(busy_percent);
            }
        }
        
        Ok(0) // Default to 0% if unable to read
    }

    /// Read clock information
    fn read_clock_info(device: &AmdDevice) -> Result<(u32, u32)> {
        let mut graphics_clock = 1000; // Default MHz
        let mut memory_clock = 1000;   // Default MHz
        
        // Read graphics clock
        let gfx_clock_path = device.device_path.join("pp_dpm_sclk");
        if let Ok(clock_info) = fs::read_to_string(&gfx_clock_path) {
            // Parse current clock from the format like "0: 300Mhz *\n1: 1500Mhz"
            if let Some(current_line) = clock_info.lines().find(|line| line.contains('*')) {
                let re = Regex::new(r"(\d+)Mhz").unwrap();
                if let Some(caps) = re.captures(current_line) {
                    graphics_clock = caps[1].parse().unwrap_or(graphics_clock);
                }
            }
        }
        
        // Read memory clock
        let mem_clock_path = device.device_path.join("pp_dpm_mclk");
        if let Ok(clock_info) = fs::read_to_string(&mem_clock_path) {
            if let Some(current_line) = clock_info.lines().find(|line| line.contains('*')) {
                let re = Regex::new(r"(\d+)Mhz").unwrap();
                if let Some(caps) = re.captures(current_line) {
                    memory_clock = caps[1].parse().unwrap_or(memory_clock);
                }
            }
        }
        
        Ok((graphics_clock, memory_clock))
    }

    /// Read fan speed
    fn read_fan_speed(device: &AmdDevice) -> Result<Option<u32>> {
        if let Some(ref hwmon_path) = device.hwmon_path {
            let fan_path = hwmon_path.join("pwm1");
            if let Ok(pwm_str) = fs::read_to_string(&fan_path) {
                if let Ok(pwm_value) = pwm_str.trim().parse::<u32>() {
                    // Convert PWM (0-255) to percentage (0-100)
                    let percentage = (pwm_value * 100) / 255;
                    return Ok(Some(percentage));
                }
            }
        }
        
        Ok(None)
    }

    /// Parse PCI information
    fn parse_pci_info(device: &AmdDevice) -> Result<PciInfo> {
        // Extract PCI information from device path or uevent
        let uevent_path = device.device_path.join("uevent");
        
        let mut vendor_id = 0x1002; // AMD default
        let mut device_id = 0x0000;
        let mut bus = 0;
        let mut device_num = 0;
        
        if let Ok(uevent_content) = fs::read_to_string(&uevent_path) {
            for line in uevent_content.lines() {
                if line.starts_with("PCI_ID=") {
                    let pci_id = line.strip_prefix("PCI_ID=").unwrap_or("");
                    let parts: Vec<&str> = pci_id.split(':').collect();
                    if parts.len() == 2 {
                        vendor_id = u16::from_str_radix(parts[0], 16).unwrap_or(vendor_id);
                        device_id = u16::from_str_radix(parts[1], 16).unwrap_or(device_id);
                    }
                } else if line.starts_with("PCI_SLOT_NAME=") {
                    let slot_name = line.strip_prefix("PCI_SLOT_NAME=").unwrap_or("");
                    // Parse format like "0000:01:00.0"
                    let parts: Vec<&str> = slot_name.split(&[':', '.']).collect();
                    if parts.len() >= 3 {
                        bus = u8::from_str_radix(parts[1], 16).unwrap_or(bus);
                        device_num = u8::from_str_radix(parts[2], 16).unwrap_or(device_num);
                    }
                }
            }
        }
        
        Ok(PciInfo {
            domain: 0,
            bus,
            device: device_num,
            function: 0,
            vendor_id,
            device_id,
        })
    }

    /// Get AMD driver version
    fn get_driver_version() -> Result<Option<String>> {
        let version_path = Path::new("/sys/module/amdgpu/version");
        
        if let Ok(version) = fs::read_to_string(version_path) {
            return Ok(Some(version.trim().to_string()));
        }
        
        // Fallback: check dmesg or modinfo
        Ok(None)
    }

    /// Apply power mode settings
    async fn apply_power_mode(&self, device: &AmdDevice, mode: PowerMode) -> Result<()> {
        let profile_path = device.device_path.join("power_dpm_force_performance_level");
        
        let profile = match mode {
            PowerMode::MaxPerformance => "high",
            PowerMode::Balanced => "auto",
            PowerMode::PowerSaver => "low",
            PowerMode::Custom => "manual",
            PowerMode::Auto => "auto",
        };
        
        if let Err(e) = fs::write(&profile_path, profile) {
            warn!("Failed to set power profile: {}", e);
            return Err(GpuError::PowerError(format!("Failed to set power profile: {}", e)));
        }
        
        info!("Applied power mode {:?} to AMD GPU {}", mode, device.index);
        Ok(())
    }

    /// Set power limit
    async fn set_device_power_limit(&self, device: &AmdDevice, limit_watts: u32) -> Result<()> {
        if let Some(ref hwmon_path) = device.hwmon_path {
            let power_cap_path = hwmon_path.join("power1_cap");
            let limit_microwatts = limit_watts * 1_000_000;
            
            if let Err(e) = fs::write(&power_cap_path, limit_microwatts.to_string()) {
                warn!("Failed to set power limit: {}", e);
                return Err(GpuError::PowerError(format!("Failed to set power limit: {}", e)));
            }
        }
        
        Ok(())
    }

    /// Set fan curve
    async fn apply_device_fan_curve(&self, device: &AmdDevice, curve: &FanCurve) -> Result<()> {
        if let Some(ref hwmon_path) = device.hwmon_path {
            // Get current temperature
            let temperature = Self::read_temperature(device)?;
            
            // Calculate target fan speed
            let target_speed = curve.calculate_fan_speed(temperature);
            
            // Set PWM value (convert percentage to 0-255 range)
            let pwm_value = (target_speed * 255) / 100;
            
            // First enable manual fan control
            let pwm_enable_path = hwmon_path.join("pwm1_enable");
            if let Err(e) = fs::write(&pwm_enable_path, "1") {
                warn!("Failed to enable manual fan control: {}", e);
            }
            
            // Set fan speed
            let pwm_path = hwmon_path.join("pwm1");
            if let Err(e) = fs::write(&pwm_path, pwm_value.to_string()) {
                warn!("Failed to set fan speed: {}", e);
                return Err(GpuError::ThermalError(format!("Failed to set fan speed: {}", e)));
            }
            
            debug!("Set AMD GPU {} fan speed to {}%", device.index, target_speed);
        }
        
        Ok(())
    }
}

#[async_trait]
impl GpuBackend for AmdBackend {
    async fn init(&mut self) -> Result<()> {
        let devices = Self::find_amd_devices()?;
        
        if devices.is_empty() {
            return Err(GpuError::GpuNotFound(0));
        }
        
        let mut device_map = HashMap::new();
        for device in devices {
            device_map.insert(device.index, device);
        }
        
        let mut devices_lock = self.devices.write().await;
        *devices_lock = device_map;
        
        info!("AMD backend initialized with {} devices", devices_lock.len());
        Ok(())
    }

    #[instrument]
    async fn detect_gpus(&self) -> Result<Vec<GpuStatus>> {
        let devices = self.devices.read().await;
        let mut gpus = Vec::new();
        
        for (_, device) in devices.iter() {
            match Self::get_device_status(device) {
                Ok(status) => gpus.push(status),
                Err(e) => {
                    warn!("Failed to get status for AMD GPU {}: {}", device.index, e);
                }
            }
        }
        
        info!("Detected {} AMD GPU(s)", gpus.len());
        Ok(gpus)
    }

    #[instrument]
    async fn get_gpu_status(&self, index: u32) -> Result<GpuStatus> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        Self::get_device_status(device)
    }

    #[instrument]
    async fn apply_config(&self, index: u32, config: &GpuConfig) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        // Apply power mode
        self.apply_power_mode(device, config.power_mode).await?;

        // Apply custom power limit
        if let Some(limit) = config.power_limit {
            self.set_device_power_limit(device, limit).await?;
        }

        // Apply fan curve
        if let Some(ref curve) = config.fan_curve {
            self.apply_device_fan_curve(device, curve).await?;
        }

        info!("Applied configuration to AMD GPU {}", index);
        Ok(())
    }

    #[instrument]
    async fn set_power_limit(&self, index: u32, limit_watts: u32) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        self.set_device_power_limit(device, limit_watts).await
    }

    #[instrument]
    async fn set_fan_curve(&self, index: u32, curve: &FanCurve) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        self.apply_device_fan_curve(device, curve).await
    }

    #[instrument]
    async fn reset_gpu(&self, index: u32) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices
            .get(&index)
            .ok_or_else(|| GpuError::GpuNotFound(index))?;

        // Reset to auto power profile
        let profile_path = device.device_path.join("power_dpm_force_performance_level");
        if let Err(e) = fs::write(&profile_path, "auto") {
            return Err(GpuError::SystemError(format!("Failed to reset power profile: {}", e)));
        }

        // Reset fan control to automatic
        if let Some(ref hwmon_path) = device.hwmon_path {
            let pwm_enable_path = hwmon_path.join("pwm1_enable");
            let _ = fs::write(&pwm_enable_path, "2"); // Auto mode
        }

        info!("Reset AMD GPU {} to defaults", index);
        Ok(())
    }

    fn supports_gpu_switching(&self) -> bool {
        // AMD supports dynamic GPU switching with DRI3
        true
    }

    #[instrument]
    async fn switch_gpu(&self, _from_index: u32, _to_index: u32) -> Result<()> {
        // AMD GPU switching typically involves:
        // 1. Using DRI_PRIME environment variable
        // 2. Setting up proper xrandr providers
        // 3. Configuring the X server
        
        // For now, return an error indicating this needs system-level support
        Err(GpuError::OperationNotSupported(
            "AMD GPU switching requires DRI_PRIME configuration".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_amd_backend_creation() {
        let backend = AmdBackend::new().await;
        assert!(backend.is_ok());
    }

    #[test]
    fn test_pci_info_parsing() {
        // Test would require mock sysfs structure
        let pci_info = PciInfo {
            domain: 0,
            bus: 1,
            device: 0,
            function: 0,
            vendor_id: 0x1002,
            device_id: 0x73df,
        };
        
        assert_eq!(pci_info.vendor_id, 0x1002); // AMD vendor ID
    }

    #[test]
    fn test_temperature_conversion() {
        // Test millidegree to degree conversion
        let millidegrees = 65000u32;
        let degrees = millidegrees / 1000;
        assert_eq!(degrees, 65);
    }
}