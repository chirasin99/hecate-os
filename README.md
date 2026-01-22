# HecateOS 🌙

<div align="center">
  <h3>⚡ High-Performance Linux Distribution for AI/ML Workstations ⚡</h3>
  <p><i>"Where raw power meets divine optimization"</i></p>
  
  [![Based on Ubuntu](https://img.shields.io/badge/Based%20on-Ubuntu%2024.04%20LTS-E95420?style=for-the-badge&logo=ubuntu)](https://ubuntu.com)
  [![NVIDIA Ready](https://img.shields.io/badge/NVIDIA-RTX%20Ready-76B900?style=for-the-badge&logo=nvidia)](https://nvidia.com)
  [![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
</div>

---

## 🔥 What is HecateOS?

HecateOS is an **adaptive** Linux distribution built on Ubuntu 24.04 LTS that automatically detects and optimizes for YOUR specific hardware, whatever it is.

### ✨ Key Features

- **🚀 Extreme Performance**: Pre-configured kernel optimizations for Intel 13th gen+ and NVIDIA RTX 40 series
- **🎮 Native NVIDIA Support**: RTX 4090 optimized with CUDA, persistence mode, and IRQ affinity tuning
- **💾 Smart Memory Management**: ZRAM compression for 128GB+ RAM systems
- **🔧 Zero-Config Dual Boot**: Seamless Windows 11 + HecateOS coexistence
- **🐳 Container-First**: Docker with native NVIDIA GPU runtime pre-configured
- **⚙️ Hardware Optimized**: Specific tuning for NVMe Gen5, DDR5-6400, and PCIe 5.0
- **🛡️ No Compromises**: Mitigations disabled, performance governor, all limiters removed

## 🎯 Adaptive Hardware Support

HecateOS **automatically detects and optimizes** for your hardware. No target needed:

| Component | What HecateOS Does | Optimization Level |
|-----------|-------------------|-------------------|
| **Any Intel CPU** | Detects generation, applies P-State/C-State tuning | Auto-scaled |
| **Any AMD CPU** | Detects Zen version, applies AMD-specific tuning | Auto-scaled |
| **8GB → 512GB RAM** | Adjusts ZRAM, swappiness, dirty ratios dynamically | Progressive |
| **Any NVIDIA GPU** | Selects optimal driver (550/535/525/470) | Tier-based |
| **Any AMD GPU** | Chooses AMDGPU or PRO driver automatically | Model-based |
| **Any Storage** | Detects NVMe gen, SATA SSD, or HDD and tunes I/O | Type-aware |

## 🔮 Philosophy

Unlike generic Linux distributions that use one-size-fits-all configs, HecateOS follows the principle of **"Adaptive Excellence"**:

1. **Hardware-Aware**: Detects your exact CPU, GPU, RAM, and storage
2. **Auto-Optimization**: Applies specific tuning for YOUR hardware
3. **No Manual Tuning**: The system figures out the best settings
4. **Progressive Scaling**: More RAM? Better GPU? It adapts accordingly

## 🚀 Quick Start

### Download ISO
```bash
wget https://github.com/Arakiss/hecate-os/releases/latest/download/hecate-os-24.04-amd64.iso
```

### Create Bootable USB
```bash
# Using dd (Linux/macOS)
sudo dd if=hecate-os-24.04-amd64.iso of=/dev/sdX bs=4M status=progress

# Or use Ventoy for multi-boot USB
```

### Installation
1. Boot from USB
2. Select "Install HecateOS"
3. Choose your NVMe drive (preserves Windows on other drives)
4. Reboot and enjoy maximum performance

## 💻 What's Included

### System Optimizations
- Intel P-State active governor
- IOMMU enabled for GPU passthrough
- C-States minimized for low latency
- PCIe ASPM disabled
- Spectre/Meltdown mitigations disabled (10-15% performance gain)

### Development Stack
- **Languages**: Python 3.12, Node.js 20 LTS, Rust, Go
- **Containers**: Docker CE with NVIDIA Container Toolkit
- **AI/ML**: CUDA 12.6, cuDNN, TensorRT, PyTorch, TensorFlow
- **Databases**: PostgreSQL 16, Redis, MongoDB
- **Tools**: Neovim, VS Code (optional), tmux, zsh with oh-my-zsh

### Performance Tools
- btop (better htop)
- nvtop (GPU monitoring)
- iostat, iotop (I/O monitoring)
- powertop (power optimization)
- turbostat (CPU frequency monitoring)

## 🛠️ Building from Source

### Prerequisites
```bash
# On Ubuntu 22.04+ or Debian 12+
sudo apt update
sudo apt install -y live-build debootstrap squashfs-tools xorriso
```

### Build Process
```bash
# Clone repository
git clone https://github.com/Arakiss/hecate-os.git
cd hecate-os

# Run build script
sudo ./build.sh

# ISO will be generated in iso/
```

### Customization
Edit configuration files in `config/` before building:
- `package-lists/`: Add/remove packages
- `includes.chroot/`: Add custom files
- `hooks/`: Modify build hooks

## 📊 Expected Performance Gains

*Note: These are theoretical improvements based on optimizations. Real benchmarks coming after first ISO build.*

| Optimization | Expected Impact | Reason |
|--------------|----------------|--------|
| Mitigations disabled | +5-15% | Spectre/Meltdown overhead removed |
| Performance governor | +3-8% | No frequency scaling delays |
| ZRAM vs disk swap | +10-20% | Memory compression faster than disk |
| Custom kernel params | +2-5% | Reduced latency, better scheduling |
| **Estimated Total** | **+15-30%** | *Actual results will vary by workload* |

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

### Areas of Interest
- Hardware-specific optimizations
- Performance tuning scripts
- Benchmark automation
- Documentation improvements

## 📜 License

HecateOS is released under the MIT License. See [LICENSE](LICENSE) for details.

### Attribution
- Based on Ubuntu 24.04 LTS by Canonical
- Inspired by Pop!_OS approach to desktop Linux
- NVIDIA drivers and CUDA are property of NVIDIA Corporation

## 🔗 Resources

- **Website**: [https://hecateos.dev](https://hecateos.dev) (coming soon)
- **Documentation**: [docs/](docs/)
- **Discord**: [Join our community](https://discord.gg/hecate-os)
- **Issues**: [GitHub Issues](https://github.com/Arakiss/hecate-os/issues)

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Arakiss/hecate-os&type=Date)](https://star-history.com/#Arakiss/hecate-os&Date)

---

<div align="center">
  <p><b>HecateOS</b> - Unleash the beast within your machine</p>
  <p>Made with 🖤 by the HecateOS Team</p>
</div>

---

<details>
<summary><b>Why "HecateOS"?</b></summary>

Named after my cat Hecate, who I named after the Greek goddess of crossroads and magic. She likes to sit at the crossroads between my keyboard and monitor while I code.

The name fits perfectly - a distro that lives at the crossroads between Windows and Linux, between raw hardware and optimized software. Plus, my cat approves. 🐱
</details>