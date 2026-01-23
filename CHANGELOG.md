# Changelog

All notable changes to HecateOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-24

### Added

#### Rust System Components (NEW)
- **hecate-core**: Hardware detection and system profiling library
  - Automatic detection of AI/Gaming/Workstation profiles
  - Hardware enumeration and capabilities detection
  - System optimization profiles based on detected hardware

- **hecate-daemon**: System optimization daemon
  - Auto-tuning based on hardware profile
  - Real-time system health monitoring
  - Thermal, memory, and GPU health management
  - Performance optimization on boot

- **hecate-gpu**: Comprehensive GPU management module
  - NVIDIA GPU control via NVML wrapper
  - Power mode profiles (MaxPerformance, Balanced, PowerSaver)
  - Real-time GPU monitoring
  - Multi-GPU support with individual control

- **hecate-monitor**: Real-time monitoring server
  - WebSocket server for streaming metrics (port 3000)
  - Comprehensive system metrics collection
  - Process tracking with top CPU/memory consumers
  - Automatic client reconnection support

- **hecate-cli**: Advanced system management CLI
  - System information display with multiple formats
  - Real-time monitoring mode
  - GPU power management commands
  - System benchmark integration
  - Network diagnostics and health checks

- **hecate-bench**: Comprehensive benchmark suite
  - CPU, Memory, Disk I/O benchmarks
  - GPU and AI/ML workload benchmarks
  - System stress testing
  - Result comparison and export (CSV/JSON)

- **hecate-pkg**: Modern package manager
  - Full dependency resolution with semver
  - Repository management with mirrors
  - Package integrity verification (SHA256, BLAKE3)
  - Transaction-based installation/removal

- **hecate-dashboard**: Web monitoring dashboard
  - Next.js with Shadcn UI base components
  - Real-time WebSocket connection
  - Responsive design with visualizations
  - Bun runtime support

#### Hardware Support Updates
- 2026 hardware support
  - NVIDIA RTX 50 series (5090, 5080, 5070)
  - AMD RX 8000/9000 series
  - Intel Arc B-series GPUs
  - NVIDIA driver 590 series

#### Infrastructure
- GitHub Actions CI/CD pipeline for Rust components
- Repository management scripts
- Custom HecateOS package repository structure

#### Original Features (from previous work)
- Initial project structure with live-build configuration
- Hardware detection system (`hardware-detector.sh`)
- Automatic optimization application (`apply-optimizations.sh`)
- NVIDIA driver installer with GPU tier detection
- Benchmark suite for performance testing
- Single ISO with automatic hardware profiling
- Comprehensive package lists for development, AI/ML, and performance tools
- GRUB theme customization
- Docker daemon pre-configuration
- Systemd services for NVIDIA persistence and IRQ affinity
- **CLI tools** (`bin/hecate*`)
  - `hecate` - Main command dispatcher
  - `hecate info` - System information display
  - `hecate update` - System updates with migration support
  - `hecate optimize` - Re-detect and apply optimizations
  - `hecate driver` - GPU driver management
  - `hecate migrate` - Run pending migrations
- **Migration system** (`migrations/`)
  - Timestamped migration scripts
  - Automatic tracking of applied migrations
  - Runs during `hecate update`
- **Docker build environment**
  - `Dockerfile.build` for reproducible builds
  - `docker-compose.yml` for local development
- **Release scripts**
  - ISO upload to Cloudflare R2/S3 (`hecate-iso-upload.sh`)
  - GPG signing (`hecate-iso-sign.sh`)
  - Full release workflow (`hecate-release.sh`)

### Changed
- Migrated core components from shell scripts to Rust
- Updated driver installer for 2026 hardware
- Enhanced system optimization strategies
- Improved hardware detection algorithms
- GitHub Actions updated to v4 (cache, upload-artifact, download-artifact)
- CI workflow now supports Docker-based builds
- ShellCheck now scans entire repository

### Fixed
- Package lists compatibility with Ubuntu 24.04
- Removed unavailable packages from AI/ML list
- Updated NVIDIA driver package references
- CI pipeline now works (deprecated actions v3 → v4)
- Build now works on Ubuntu 24.04 (removed unsupported live-build options)

## [0.1.0] - Unreleased

First alpha release. Hardware detection and optimization framework complete.

### Tested Hardware
- Intel Core i9-13900K
- NVIDIA RTX 4090
- 128GB DDR5-6400

### Known Limitations
- AMD support is theoretical (untested)
- Laptop optimizations not implemented
- Only high-end Intel/NVIDIA hardware tested

[Unreleased]: https://github.com/Arakiss/hecate-os/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Arakiss/hecate-os/releases/tag/v0.1.0
