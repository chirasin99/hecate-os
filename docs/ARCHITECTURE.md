# HecateOS Architecture

## Overview

HecateOS automatically detects hardware and applies performance optimizations on first boot.

## How It Works

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│ Hardware Detect  │────▶│  Auto Profile    │────▶│  Apply Tuning    │
├──────────────────┤     ├──────────────────┤     ├──────────────────┤
│ • CPU generation │     │ Based on:        │     │ • sysctl.conf    │
│ • GPU tier       │     │ • GPU VRAM       │     │ • GRUB params    │
│ • RAM amount     │     │ • RAM size       │     │ • GPU settings   │
│ • Storage type   │     │ • CPU cores      │     │ • I/O scheduler  │
└──────────────────┘     └──────────────────┘     └──────────────────┘
```

## System Components

### Shell Scripts (Legacy)
- `hardware-detector.sh` - Detects and profiles hardware
- `apply-optimizations.sh` - Applies profile-specific tuning
- `hecate-driver-installer.sh` - GPU driver selection

### Rust Components (v0.2.0+)
- **hecate-core** - Hardware detection & profiling library
- **hecate-daemon** - System optimization daemon
- **hecate-gpu** - GPU management module (NVIDIA)
- **hecate-monitor** - Real-time monitoring server (WebSocket)
- **hecate-cli** - Advanced CLI tool
- **hecate-bench** - Comprehensive benchmark suite
- **hecate-pkg** - Package manager core

### Web Dashboard
- Next.js application with Shadcn UI
- Real-time WebSocket connection to monitoring server
- Accessible at http://localhost:9313

## Default Ports

HecateOS uses these default ports (all configurable):
- **9313** - System monitoring (hecate-monitor)
- **9314** - Package manager API (hecate-pkg) 
- **9315** - Remote management (future)

> Why 931x? It's our easter egg - 93 (IX in Roman = 9), 13 (mystical), together forming "Hecate's numbers"

## What Gets Tuned

### CPU
- Intel P-State or AMD P-State governor
- C-State limits
- Turbo boost settings
- IRQ affinity

### Memory
- Swappiness (10 for high RAM, 60 for low)
- Dirty ratios
- ZRAM compression ratio
- Transparent hugepages

### GPU (NVIDIA)
- Driver version by generation
- Persistence mode
- Power limits
- Compute mode

### Storage
- I/O scheduler (none for NVMe Gen4+, mq-deadline for older)
- Read-ahead values
- Queue depth

### Kernel
- `mitigations=off` for ~10% performance gain (configurable)
- `intel_pstate=active`
- IOMMU settings
- PCIe ASPM disabled

## System Profiles

HecateOS automatically detects and applies one of these profiles:

- **AI Flagship** - Multi-GPU, high VRAM, optimized for ML/AI workloads
- **Pro Workstation** - High-end single GPU, balanced for professional work
- **Gaming Enthusiast** - Gaming-focused optimizations
- **Content Creator** - Optimized for video/3D rendering
- **Developer** - Balanced for compilation and development
- **Standard** - Conservative optimizations for general use

## CLI Tools

### Rust CLI (hecate-cli)
```bash
hecate info          # System information with JSON/YAML export
hecate monitor       # Real-time monitoring in terminal
hecate gpu power     # GPU power management
hecate benchmark     # Run comprehensive benchmarks
hecate health        # System health check
```

### Benchmark Suite (hecate-bench)
```bash
hecate-bench all     # Run all benchmarks
hecate-bench cpu     # CPU performance tests
hecate-bench gpu     # GPU compute tests
hecate-bench ai      # AI/ML workload tests
hecate-bench stress  # Stress testing
```

### Package Manager (hecate-pkg)
```bash
hecate-pkg install <package>   # Install with dependency resolution
hecate-pkg search <query>      # Search packages
hecate-pkg update              # Update all packages
hecate-pkg sync                # Sync repositories
```

### Legacy Shell Commands
```bash
hecate info          # Show system info and applied optimizations
hecate update        # Update system packages and run migrations
hecate optimize      # Re-detect hardware and apply optimizations
hecate driver        # Manage GPU drivers
hecate migrate       # Run pending migrations
```