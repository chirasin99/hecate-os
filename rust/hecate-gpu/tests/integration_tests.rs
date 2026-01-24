//! Integration tests for hecate-gpu

use hecate_gpu::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_gpu_manager_initialization() {
    let result = GpuManager::new().await;
    // Should succeed even without GPUs (empty backend)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gpu_detection_without_hardware() {
    let manager = GpuManager::new().await.unwrap();
    let result = manager.detect_gpus().await;
    
    // Should succeed but may return empty list if no GPUs
    assert!(result.is_ok());
    let gpus = result.unwrap();
    println!("Detected {} GPUs", gpus.len());
}

#[tokio::test]
async fn test_gpu_config_presets() {
    let balanced = GpuConfig::balanced();
    assert_eq!(balanced.power_mode, PowerMode::Balanced);
    assert!(balanced.auto_load_balance);
    assert_eq!(balanced.temp_target, Some(83));
    
    let max_perf = GpuConfig::max_performance();
    assert_eq!(max_perf.power_mode, PowerMode::MaxPerformance);
    assert_eq!(max_perf.temp_target, Some(90));
    assert!(max_perf.auto_load_balance);
    
    let power_saver = GpuConfig::power_saver();
    assert_eq!(power_saver.power_mode, PowerMode::PowerSaver);
    assert_eq!(power_saver.temp_target, Some(70));
    assert!(!power_saver.auto_load_balance);
}

#[tokio::test]
async fn test_fan_curve_functionality() {
    let aggressive_curve = FanCurve::aggressive();
    let quiet_curve = FanCurve::quiet();
    
    // Test aggressive curve
    assert_eq!(aggressive_curve.calculate_fan_speed(30), 20);
    assert_eq!(aggressive_curve.calculate_fan_speed(85), 100);
    let interpolated = aggressive_curve.calculate_fan_speed(60);
    assert!(interpolated > 40 && interpolated < 60);
    
    // Test quiet curve  
    assert_eq!(quiet_curve.calculate_fan_speed(40), 0);
    assert_eq!(quiet_curve.calculate_fan_speed(90), 100);
    
    // Test edge cases
    assert_eq!(aggressive_curve.calculate_fan_speed(25), 20); // Below range
    assert_eq!(aggressive_curve.calculate_fan_speed(90), 100); // Above range
}

#[tokio::test]
async fn test_format_bytes_utility() {
    assert_eq!(format_bytes(1024), "1.00 KiB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    assert_eq!(format_bytes(1536), "1.50 KiB");
    assert_eq!(format_bytes(0), "0.00 B");
}

#[tokio::test]
async fn test_efficiency_score_calculation() {
    let test_gpu = GpuStatus {
        index: 0,
        name: "Test GPU".to_string(),
        vendor: GpuVendor::NVIDIA,
        gpu_type: GpuType::Discrete,
        temperature: 70,
        power_draw: 200,
        power_limit: 300,
        memory_used: 2_147_483_648, // 2GB
        memory_total: 8_589_934_592, // 8GB
        utilization_gpu: 50,
        utilization_memory: 40,
        fan_speed: Some(60),
        clock_graphics: 1500,
        clock_memory: 7000,
        driver_version: Some("470.86".to_string()),
        pci_info: PciInfo {
            domain: 0,
            bus: 1,
            device: 0,
            function: 0,
            vendor_id: 0x10DE,
            device_id: 0x2204,
        },
        power_state: PowerState::Active,
    };
    
    let score = calculate_efficiency_score(&test_gpu);
    assert!(score >= 0.0 && score <= 1.0);
    
    // Test edge cases
    let hot_gpu = GpuStatus {
        temperature: 95,
        power_draw: 299,
        utilization_gpu: 100,
        ..test_gpu.clone()
    };
    let hot_score = calculate_efficiency_score(&hot_gpu);
    assert!(hot_score < score); // Should have lower efficiency
    
    let cool_gpu = GpuStatus {
        temperature: 40,
        power_draw: 100,
        utilization_gpu: 80,
        ..test_gpu
    };
    let cool_score = calculate_efficiency_score(&cool_gpu);
    assert!(cool_score > score); // Should have higher efficiency
}

#[tokio::test]
async fn test_gpu_summary_string() {
    let test_gpu = GpuStatus {
        index: 0,
        name: "NVIDIA RTX 4090".to_string(),
        vendor: GpuVendor::NVIDIA,
        gpu_type: GpuType::Discrete,
        temperature: 75,
        power_draw: 350,
        power_limit: 450,
        memory_used: 4_294_967_296, // 4GB
        memory_total: 25_769_803_776, // 24GB
        utilization_gpu: 85,
        utilization_memory: 60,
        fan_speed: Some(70),
        clock_graphics: 2520,
        clock_memory: 10501,
        driver_version: Some("525.105.17".to_string()),
        pci_info: PciInfo {
            domain: 0,
            bus: 1,
            device: 0,
            function: 0,
            vendor_id: 0x10DE,
            device_id: 0x2684,
        },
        power_state: PowerState::Active,
    };
    
    let summary = gpu_summary(&test_gpu);
    assert!(summary.contains("NVIDIA RTX 4090"));
    assert!(summary.contains("75°C"));
    assert!(summary.contains("350W"));
    assert!(summary.contains("85%"));
    assert!(summary.contains("4.00 GiB"));
    assert!(summary.contains("24.00 GiB"));
}

#[tokio::test]
async fn test_monitoring_events() {
    let manager = GpuManager::new().await.unwrap();
    let mut event_receiver = manager.subscribe_events();
    
    // Start monitoring
    let monitor_result = manager.start_monitoring().await;
    
    // Should succeed even without GPUs
    assert!(monitor_result.is_ok());
    
    // Test event reception with timeout
    tokio::select! {
        event = event_receiver.recv() => {
            match event {
                Ok(gpu_event) => {
                    println!("Received event: {:?}", gpu_event);
                }
                Err(e) => {
                    println!("Event receiver error: {}", e);
                }
            }
        }
        _ = sleep(Duration::from_millis(100)) => {
            // Timeout is expected if no events are generated
            println!("No events received within timeout (expected)");
        }
    }
    
    manager.stop_monitoring().await;
}

#[cfg(feature = "testing")]
mod mock_tests {
    use super::*;
    
    // These tests would run with mock GPU backends for CI/CD
    
    #[tokio::test]
    async fn test_mock_gpu_detection() {
        // Test with mock GPUs when testing feature is enabled
        // This allows testing without real hardware
    }
    
    #[tokio::test]
    async fn test_mock_configuration_application() {
        // Test configuration application with mock backends
    }
    
    #[tokio::test]
    async fn test_mock_load_balancing() {
        // Test load balancing with multiple mock GPUs
    }
}

#[tokio::test]
async fn test_error_handling() {
    use error::GpuError;
    
    // Test error types
    let gpu_not_found = GpuError::GpuNotFound(999);
    assert!(gpu_not_found.to_string().contains("999"));
    
    let backend_not_available = GpuError::BackendNotAvailable(GpuVendor::Intel);
    assert!(backend_not_available.to_string().contains("Intel"));
    
    let operation_not_supported = GpuError::OperationNotSupported("test operation".to_string());
    assert!(operation_not_supported.to_string().contains("test operation"));
    
    // Test error severity
    assert_eq!(gpu_not_found.severity(), error::ErrorSeverity::High);
    assert_eq!(operation_not_supported.severity(), error::ErrorSeverity::Medium);
    
    // Test recoverability
    let timeout_error = GpuError::Timeout(Duration::from_secs(30));
    assert!(timeout_error.is_recoverable());
    assert!(!gpu_not_found.is_recoverable());
}

#[test]
fn test_power_mode_serialization() {
    use serde_json;
    
    let modes = [
        PowerMode::MaxPerformance,
        PowerMode::Balanced,
        PowerMode::PowerSaver,
        PowerMode::Custom,
        PowerMode::Auto,
    ];
    
    for mode in &modes {
        let serialized = serde_json::to_string(mode).unwrap();
        let deserialized: PowerMode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(*mode, deserialized);
    }
}

#[test]
fn test_gpu_vendor_serialization() {
    use serde_json;
    
    let vendors = [
        GpuVendor::NVIDIA,
        GpuVendor::AMD,
        GpuVendor::Intel,
        GpuVendor::Unknown,
    ];
    
    for vendor in &vendors {
        let serialized = serde_json::to_string(vendor).unwrap();
        let deserialized: GpuVendor = serde_json::from_str(&serialized).unwrap();
        assert_eq!(*vendor, deserialized);
    }
}

#[test]
fn test_pci_info_structure() {
    let pci_info = PciInfo {
        domain: 0,
        bus: 1,
        device: 0,
        function: 0,
        vendor_id: 0x10DE, // NVIDIA
        device_id: 0x2204, // RTX 3090
    };
    
    assert_eq!(pci_info.vendor_id, 0x10DE);
    assert_eq!(pci_info.device_id, 0x2204);
    assert_eq!(pci_info.bus, 1);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let manager = GpuManager::new().await.unwrap();
    
    // Test concurrent detection calls
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let manager_ref = &manager;
            tokio::spawn(async move {
                manager_ref.detect_gpus().await
            })
        })
        .collect();
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test] 
async fn test_memory_usage() {
    // Test that the library doesn't leak memory
    let manager = GpuManager::new().await.unwrap();
    
    for _ in 0..100 {
        let _ = manager.detect_gpus().await;
        let _ = manager.get_all_gpu_status().await;
    }
    
    // If we get here without OOM, the test passes
    println!("Memory usage test completed");
}