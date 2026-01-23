//! HecateOS Core Library
//! 
//! Core functionality for hardware detection, profiling, and optimization

pub mod config;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use sysinfo::System;

/// System profile based on detected hardware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemProfile {
    /// High-end ML/AI workstation (RTX 4090+, 64GB+ RAM)
    AIFlagship,
    /// Professional workstation (RTX 4070+, 32GB+ RAM)
    ProWorkstation,
    /// Gaming/Content creation (RTX 4060+, 16GB+ RAM)
    HighPerformance,
    /// Development machine (Any GPU, 16GB+ RAM)
    Developer,
    /// Standard desktop
    Standard,
}

/// Detected hardware information
#[derive(Debug, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: Vec<GpuInfo>,
    pub storage: Vec<StorageInfo>,
    pub profile: SystemProfile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub vendor: String,
    pub model: String,
    pub cores: usize,
    pub threads: usize,
    pub base_frequency: f64,
    pub max_frequency: f64,
    pub generation: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub speed_mhz: Option<u32>,
    pub memory_type: Option<String>, // DDR4, DDR5
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model: String,
    pub vram_gb: f64,
    pub driver_version: Option<String>,
    pub compute_capability: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageInfo {
    pub device: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub storage_type: StorageType,
    pub nvme_gen: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    NvmeGen5,
    NvmeGen4,
    NvmeGen3,
    Sata,
    Hdd,
    Unknown,
}

/// Main hardware detector
pub struct HardwareDetector {
    system: System,
}

impl HardwareDetector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    /// Detect all hardware and create a profile
    pub fn detect(&mut self) -> Result<HardwareInfo> {
        let cpu = self.detect_cpu()?;
        let memory = self.detect_memory()?;
        let gpu = self.detect_gpu()?;
        let storage = self.detect_storage()?;
        
        let profile = Self::determine_profile(&cpu, &memory, &gpu);
        
        Ok(HardwareInfo {
            cpu,
            memory,
            gpu,
            storage,
            profile,
        })
    }

    fn detect_cpu(&self) -> Result<CpuInfo> {
        let cpus = self.system.cpus();
        if cpus.is_empty() {
            anyhow::bail!("No CPU detected");
        }

        let first_cpu = &cpus[0];
        let cpu_info = procfs::CpuInfo::new()?;
        
        // Parse vendor and model from /proc/cpuinfo
        let vendor = cpu_info
            .cpus
            .first()
            .and_then(|cpu| cpu.vendor_id.clone())
            .unwrap_or_else(|| "Unknown".to_string());
            
        let model = cpu_info
            .cpus
            .first()
            .and_then(|cpu| cpu.model_name.clone())
            .unwrap_or_else(|| first_cpu.brand().to_string());

        // Detect generation (Intel example)
        let generation = if vendor.to_lowercase().contains("intel") {
            model.split('-')
                .nth(1)
                .and_then(|s| s.chars().take(2).collect::<String>().parse().ok())
        } else {
            None
        };

        Ok(CpuInfo {
            vendor,
            model,
            cores: self.system.physical_core_count().unwrap_or(1),
            threads: cpus.len(),
            base_frequency: first_cpu.frequency() as f64,
            max_frequency: first_cpu.frequency() as f64, // Would need turbo freq
            generation,
        })
    }

    fn detect_memory(&self) -> Result<MemoryInfo> {
        let total_memory = self.system.total_memory();
        let total_gb = total_memory as f64 / 1024.0 / 1024.0 / 1024.0;
        
        // Try to detect memory speed from dmidecode (would need sudo)
        let (speed_mhz, memory_type) = self.detect_memory_details()
            .unwrap_or((None, None));
        
        Ok(MemoryInfo {
            total_gb,
            speed_mhz,
            memory_type,
        })
    }

    fn detect_memory_details(&self) -> Result<(Option<u32>, Option<String>)> {
        // This would parse dmidecode output
        // For now, return placeholders
        Ok((None, None))
    }

    fn detect_gpu(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        
        // Check for NVIDIA GPUs
        if let Ok(nvidia_gpus) = self.detect_nvidia_gpus() {
            gpus.extend(nvidia_gpus);
        }
        
        // Check for AMD GPUs
        if let Ok(amd_gpus) = self.detect_amd_gpus() {
            gpus.extend(amd_gpus);
        }
        
        // Check for Intel GPUs
        if let Ok(intel_gpus) = self.detect_intel_gpus() {
            gpus.extend(intel_gpus);
        }
        
        Ok(gpus)
    }

    fn detect_nvidia_gpus(&self) -> Result<Vec<GpuInfo>> {
        // Would use nvml-wrapper here
        // Placeholder implementation
        Ok(vec![])
    }

    fn detect_amd_gpus(&self) -> Result<Vec<GpuInfo>> {
        // Would parse /sys/class/drm
        Ok(vec![])
    }

    fn detect_intel_gpus(&self) -> Result<Vec<GpuInfo>> {
        // Would parse /sys/class/drm
        Ok(vec![])
    }

    fn detect_storage(&self) -> Result<Vec<StorageInfo>> {
        let mut storage_devices = Vec::new();
        
        // Parse /sys/block for storage devices
        let block_path = Path::new("/sys/block");
        if block_path.exists() {
            for entry in fs::read_dir(block_path)? {
                let entry = entry?;
                let device_name = entry.file_name().to_string_lossy().to_string();
                
                // Skip loop devices and ram disks
                if device_name.starts_with("loop") || device_name.starts_with("ram") {
                    continue;
                }
                
                if let Ok(info) = self.analyze_storage_device(&device_name) {
                    storage_devices.push(info);
                }
            }
        }
        
        Ok(storage_devices)
    }

    fn analyze_storage_device(&self, device: &str) -> Result<StorageInfo> {
        let device_path = format!("/sys/block/{}", device);
        
        // Detect if NVMe
        let storage_type = if device.starts_with("nvme") {
            // Try to detect NVMe generation
            StorageType::NvmeGen4 // Placeholder
        } else if device.starts_with("sd") {
            // Check if SSD or HDD via rotational flag
            let rotational_path = format!("{}/queue/rotational", device_path);
            let is_hdd = fs::read_to_string(rotational_path)
                .unwrap_or_else(|_| "1".to_string())
                .trim() == "1";
            
            if is_hdd {
                StorageType::Hdd
            } else {
                StorageType::Sata
            }
        } else {
            StorageType::Unknown
        };
        
        // Get size
        let size_path = format!("{}/size", device_path);
        let sectors = fs::read_to_string(size_path)
            .unwrap_or_else(|_| "0".to_string())
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        let total_gb = (sectors * 512) as f64 / 1024.0 / 1024.0 / 1024.0;
        
        Ok(StorageInfo {
            device: format!("/dev/{}", device),
            mount_point: "/".to_string(), // Would need to check mounts
            total_gb,
            storage_type,
            nvme_gen: None,
        })
    }

    fn determine_profile(cpu: &CpuInfo, memory: &MemoryInfo, gpus: &[GpuInfo]) -> SystemProfile {
        // Check for flagship AI system
        if memory.total_gb >= 64.0 {
            if let Some(gpu) = gpus.first() {
                if gpu.model.contains("5090") || gpu.model.contains("4090") || 
                   gpu.model.contains("A6000") || gpu.vram_gb >= 24.0 {
                    return SystemProfile::AIFlagship;
                }
            }
        }
        
        // Check for pro workstation
        if memory.total_gb >= 32.0 {
            if let Some(gpu) = gpus.first() {
                if gpu.model.contains("5080") || gpu.model.contains("4080") ||
                   gpu.model.contains("5070") || gpu.model.contains("4070") ||
                   gpu.vram_gb >= 12.0 {
                    return SystemProfile::ProWorkstation;
                }
            }
        }
        
        // Check for high performance
        if memory.total_gb >= 16.0 && !gpus.is_empty() {
            return SystemProfile::HighPerformance;
        }
        
        // Developer machine
        if memory.total_gb >= 16.0 {
            return SystemProfile::Developer;
        }
        
        SystemProfile::Standard
    }
}

/// Apply system optimizations based on profile
pub fn apply_optimizations(profile: &SystemProfile) -> Result<()> {
    match profile {
        SystemProfile::AIFlagship => apply_ai_flagship_optimizations(),
        SystemProfile::ProWorkstation => apply_pro_workstation_optimizations(),
        SystemProfile::HighPerformance => apply_high_performance_optimizations(),
        SystemProfile::Developer => apply_developer_optimizations(),
        SystemProfile::Standard => apply_standard_optimizations(),
    }
}

fn apply_ai_flagship_optimizations() -> Result<()> {
    // Set aggressive performance settings
    // - CPU governor to performance
    // - GPU to maximum performance
    // - Disable all power saving
    // - Maximize PCIe bandwidth
    // - Set memory to lowest latency
    println!("Applying AI Flagship optimizations...");
    Ok(())
}

fn apply_pro_workstation_optimizations() -> Result<()> {
    println!("Applying Pro Workstation optimizations...");
    Ok(())
}

fn apply_high_performance_optimizations() -> Result<()> {
    println!("Applying High Performance optimizations...");
    Ok(())
}

fn apply_developer_optimizations() -> Result<()> {
    println!("Applying Developer optimizations...");
    Ok(())
}

fn apply_standard_optimizations() -> Result<()> {
    println!("Applying Standard optimizations...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let mut detector = HardwareDetector::new();
        let info = detector.detect();
        assert!(info.is_ok());
    }
}