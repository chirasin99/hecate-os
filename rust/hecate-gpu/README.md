# HecateOS GPU Management Library

A comprehensive GPU management library for HecateOS, providing advanced GPU monitoring, configuration, and optimization capabilities for NVIDIA and AMD GPUs.

## Features

### 🔧 Multi-Vendor GPU Support
- **NVIDIA GPUs**: Full support via NVML (NVIDIA Management Library)
- **AMD GPUs**: Comprehensive support via DRM and sysfs interfaces
- **Intel GPUs**: Basic support (planned)

### 🚀 Performance & Monitoring
- Real-time GPU status monitoring (temperature, power, utilization, memory)
- Historical metrics collection and analysis
- Performance trend analysis and anomaly detection
- Automated alerting system with configurable thresholds

### ⚖️ Multi-GPU Load Balancing
- Intelligent workload distribution across multiple GPUs
- Multiple balancing strategies (least utilized, thermal optimized, power efficient)
- Custom weighted assignment algorithms
- Automatic rebalancing based on system conditions

### 🎛️ Advanced Configuration Management
- Dynamic power management with multiple modes
- Custom fan curve configuration
- Clock frequency adjustment (overclocking/underclocking)
- Automatic GPU switching (integrated ↔ discrete)

### 🔄 Driver Management
- Automatic driver updates
- Version compatibility checking
- Driver rollback support

### 📊 Performance Profiling
- GPU efficiency scoring
- Workload-specific performance predictions
- Bottleneck identification
- Thermal and power optimization recommendations

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
hecate-gpu = "0.1.0"
```

### Basic Usage

```rust
use hecate_gpu::{GpuManager, GpuConfig, PowerMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GPU manager
    let manager = GpuManager::new().await?;
    
    // Detect all available GPUs
    let gpus = manager.detect_gpus().await?;
    println!("Found {} GPUs", gpus.len());
    
    for gpu in &gpus {
        println!("GPU {}: {} - {}°C, {}% utilization", 
                 gpu.index, gpu.name, gpu.temperature, gpu.utilization_gpu);
    }
    
    // Apply balanced configuration to first GPU
    if !gpus.is_empty() {
        let config = GpuConfig::balanced();
        manager.apply_config(0, config).await?;
        println!("Applied balanced configuration to GPU 0");
    }
    
    Ok(())
}
```

### Advanced Monitoring

```rust
use hecate_gpu::{GpuManager, gpu_summary};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = GpuManager::new().await?;
    let mut event_receiver = manager.subscribe_events();
    
    // Start real-time monitoring
    manager.start_monitoring().await?;
    
    // Listen for GPU events
    tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            match event {
                GpuEvent::TemperatureAlert { gpu_index, temperature, threshold } => {
                    println!("🌡️  GPU {} temperature alert: {}°C (threshold: {}°C)", 
                             gpu_index, temperature, threshold);
                }
                GpuEvent::VramAlert { gpu_index, used_percent, threshold } => {
                    println!("💾 GPU {} VRAM alert: {}% (threshold: {}%)", 
                             gpu_index, used_percent, threshold);
                }
                _ => {}
            }
        }
    });
    
    // Periodic status updates
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let statuses = manager.get_all_gpu_status().await?;
        
        for status in statuses {
            println!("{}", gpu_summary(&status));
        }
    }
}
```

### Load Balancing Setup

```rust
use hecate_gpu::{GpuManager, load_balancer::WorkloadType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = GpuManager::new().await?;
    let gpus = manager.detect_gpus().await?;
    
    if gpus.len() > 1 {
        // Enable automatic load balancing
        manager.enable_load_balancing().await?;
        
        // Get assignment for a new ML workload
        let assignment = manager.assign_workload().await?;
        println!("Assigned workload to GPU {} (confidence: {:.2}): {}", 
                 assignment.gpu_index, assignment.confidence, assignment.reason);
        
        // Predict performance for different workload types
        for &workload_type in &[WorkloadType::MachineLearning, WorkloadType::Graphics] {
            let prediction = manager.predict_performance(assignment.gpu_index, workload_type).await?;
            println!("Predicted {:?} performance: {:.2} (confidence: {:.2})", 
                     workload_type, prediction.predicted_score, prediction.confidence);
        }
    }
    
    Ok(())
}
```

### Custom GPU Configuration

```rust
use hecate_gpu::{GpuConfig, PowerMode, FanCurve};

// Create a custom high-performance configuration
let config = GpuConfig {
    power_mode: PowerMode::MaxPerformance,
    power_limit: Some(400), // 400W power limit
    temp_target: Some(85),  // Target 85°C
    fan_curve: Some(FanCurve::aggressive()),
    memory_clock_offset: Some(500), // +500 MHz memory
    gpu_clock_offset: Some(100),    // +100 MHz GPU
    auto_load_balance: true,
};

manager.apply_config(0, config).await?;
```

## Configuration

### Power Management Modes

- **MaxPerformance**: Removes power limits for maximum performance
- **Balanced**: Balances performance and power consumption (default)
- **PowerSaver**: Minimizes power consumption
- **Custom**: Allows manual configuration of all parameters
- **Auto**: Automatically adjusts based on workload

### Fan Curve Presets

- **Aggressive**: High fan speeds for maximum cooling
- **Quiet**: Lower fan speeds for reduced noise
- **Custom**: Define your own temperature → fan speed curve

### Load Balancing Strategies

- **LeastUtilized**: Assigns to GPU with lowest current utilization
- **ThermalOptimized**: Assigns to coolest GPU
- **PowerEfficient**: Assigns to most power-efficient GPU
- **MemoryOptimized**: Assigns to GPU with most available VRAM
- **PerformanceOptimized**: Assigns to GPU with best performance potential
- **Custom**: Uses custom weights for multiple criteria

## Monitoring & Alerting

The library provides comprehensive monitoring with configurable alerts:

### Alert Types
- **Temperature Alerts**: Triggered when GPU exceeds thermal thresholds
- **VRAM Alerts**: Triggered when memory usage is high
- **Power Alerts**: Triggered when power draw approaches limits
- **Performance Alerts**: Triggered for performance degradation

### Historical Analysis
- Performance trend analysis
- Anomaly detection (temperature spikes, power drops, etc.)
- Efficiency scoring over time
- Bottleneck identification

## Hardware Requirements

### NVIDIA GPUs
- **Driver**: NVIDIA proprietary drivers (470.x or newer recommended)
- **API**: NVML (included with drivers)
- **Permissions**: User must have access to NVIDIA devices
- **Supported Cards**: All modern NVIDIA GPUs (GTX 10 series and newer)

### AMD GPUs
- **Driver**: Mesa drivers or AMDGPU-PRO
- **API**: DRM/sysfs interfaces
- **Permissions**: User must have access to `/sys/class/drm/` and hwmon
- **Supported Cards**: All modern AMD GPUs (RX 400 series and newer)

### System Requirements
- **OS**: Linux (tested on Ubuntu 20.04+)
- **Architecture**: x86_64, aarch64
- **Memory**: Minimal overhead (<50MB per GPU)
- **CPU**: Negligible impact (<1% CPU usage)

## Performance

The library is designed for minimal overhead:

- **Startup time**: <100ms
- **Memory usage**: <50MB per daemon
- **CPU usage**: <1% idle
- **Response time**: <10ms for status queries
- **Monitoring frequency**: Configurable (default: 1Hz)

## Building

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libdrm-dev

# NVIDIA Support (optional)
sudo apt install -y nvidia-driver-525 nvidia-utils-525

# AMD Support (optional) 
sudo apt install -y mesa-vulkan-drivers libdrm-amdgpu1
```

### Build Commands

```bash
# Standard build
cargo build --release

# Build with specific features
cargo build --release --features "nvidia"
cargo build --release --features "amd"
cargo build --release --features "nvidia,amd"

# Run tests
cargo test

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --no-deps --open
```

### Cross Compilation

```bash
# For ARM64 (Raspberry Pi 4, etc.)
cargo build --target aarch64-unknown-linux-gnu --release

# For static linking
cargo build --target x86_64-unknown-linux-musl --release
```

## Examples

See the [`examples/`](examples/) directory for more comprehensive examples:

- [`basic_monitoring.rs`](examples/basic_monitoring.rs) - Simple GPU monitoring
- [`load_balancing.rs`](examples/load_balancing.rs) - Multi-GPU load balancing
- [`power_management.rs`](examples/power_management.rs) - Advanced power configuration
- [`anomaly_detection.rs`](examples/anomaly_detection.rs) - Performance anomaly detection
- [`driver_management.rs`](examples/driver_management.rs) - Automatic driver updates

## API Documentation

Full API documentation is available at [docs.rs](https://docs.rs/hecate-gpu) or can be generated locally:

```bash
cargo doc --no-deps --open
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/hecateos/hecate-os
cd hecate-os/rust/hecate-gpu

# Install development dependencies
cargo install cargo-watch cargo-audit

# Run development server with auto-reload
cargo watch -x check -x test
```

### Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run with mock hardware (for CI)
cargo test --features testing

# Run benchmarks
cargo bench
```

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## Acknowledgments

- NVIDIA for the NVML API
- AMD for the open-source AMDGPU drivers
- The Rust GPU compute ecosystem
- All contributors to this project

---

*Part of the HecateOS project - making Linux performance optimization automatic.*