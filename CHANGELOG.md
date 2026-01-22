# Changelog

All notable changes to HecateOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of HecateOS
- Adaptive hardware detection system
- Automatic optimization based on detected hardware
- Support for Intel 10th-13th gen CPUs
- Support for NVIDIA RTX 20/30/40 series GPUs
- Intelligent driver selection system
- ZRAM configuration with dynamic sizing
- Multiple ISO editions (Ultimate, Workstation, Gaming, Developer, Lite, Server)
- Welcome wizard for first boot
- Performance benchmarking suite
- Post-installation setup script
- GitHub Actions CI/CD pipeline
- Web installer (curl hecate.sh | bash)

### Security
- CPU mitigations disabled by default for performance (can be re-enabled)
- Security policy and hardening guide included

### Known Issues
- AMD CPU support is theoretical (needs testing)
- AMD GPU support is basic (needs testing)
- Laptop battery optimizations not implemented
- Secure Boot not supported with NVIDIA drivers

## [1.0.0] - TBD

### Initial Release
- First public release
- Based on Ubuntu 24.04 LTS
- Optimized for high-end workstations
- Focus on Intel + NVIDIA hardware

---

## Release Types

- **Major (X.0.0)**: Breaking changes, major features
- **Minor (0.X.0)**: New features, hardware support
- **Patch (0.0.X)**: Bug fixes, performance improvements

## Versioning Strategy

- **1.x**: Ubuntu 24.04 base
- **2.x**: Future Ubuntu 26.04 base (planned)
- **Point releases**: Monthly security and bug fixes