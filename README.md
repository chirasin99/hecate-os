# HecateOS

A Linux distribution that detects your hardware and applies specific optimizations automatically. Based on Ubuntu 24.04 LTS.

> **Status: Alpha (v0.2.0)** — Major Rust components added, real-time monitoring, web dashboard. Framework complete, tested on limited hardware.

## The Problem

Most Linux distros ship with generic configs. You install Ubuntu, then spend hours tweaking sysctl, GRUB parameters, GPU drivers, and kernel settings. Or you don't, and leave performance on the table.

## What HecateOS Does

On first boot, HecateOS runs `hardware-detector.sh` which:

1. **Detects your hardware** — CPU model/generation, GPU vendor/model/VRAM, RAM amount/speed, storage type
2. **Creates a profile** — Automatically classifies your system based on capabilities
3. **Applies optimizations** — Sets kernel parameters, sysctl values, GPU settings, I/O schedulers specific to YOUR hardware

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Hardware Detect │ ──▶ │ Auto Profile    │ ──▶ │ Apply Tuning    │
│                 │     │                 │     │                 │
│ • CPU gen       │     │ Based on:       │     │ • sysctl.conf   │
│ • GPU tier      │     │ • GPU VRAM      │     │ • GRUB params   │
│ • RAM amount    │     │ • RAM amount    │     │ • GPU settings  │
│ • Storage type  │     │ • CPU cores     │     │ • I/O scheduler │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## What Gets Tuned

**CPU** — Intel P-State or AMD P-State governor, C-State limits, turbo boost settings

**Memory** — Swappiness (10 for high RAM, 60 for low), dirty ratios, ZRAM compression ratio, transparent hugepages

**GPU (NVIDIA)** — Driver version by generation (570 for RTX 40, 535 for RTX 30, etc.), persistence mode, power limits, compute mode

**Storage** — I/O scheduler (none for NVMe Gen4+, mq-deadline for older), read-ahead values

**Kernel** — `mitigations=off` for ~10% perf gain (configurable), `intel_pstate=active`, IOMMU, PCIe ASPM off

## Hardware Support

### Tested on:
- Intel Core i9-13900K
- NVIDIA RTX 4090
- 128GB DDR5-6400
- Samsung 990 PRO NVMe

### Supported (2026 drivers included):
- **NVIDIA**: RTX 50 series (5090, 5080, 5070), RTX 40/30/20 series, GTX 16/10 series
- **AMD**: RX 8000/9000 series (upcoming), RX 7000/6000 series
- **Intel**: Arc B-series, Arc A-series
- **CPUs**: Intel 10th gen+, AMD Zen 2+
- **Memory**: 8GB-512GB RAM
- **Storage**: NVMe Gen5/4/3, SATA SSD/HDD

## New in v0.2.0: Rust System Components

HecateOS now includes high-performance Rust components for critical system functions:

### Core Components

- **hecate-monitor** - Real-time system monitoring server
  - WebSocket server on port 3000
  - Browser-based dashboard at http://localhost:3000
  - Streams CPU, memory, GPU, disk, network metrics
  
- **hecate-cli** - Advanced system management CLI
  ```bash
  hecate info          # System information with JSON/YAML export
  hecate monitor       # Real-time monitoring in terminal
  hecate gpu power     # GPU power management
  hecate benchmark     # Run comprehensive benchmarks
  hecate health        # System health check
  ```

- **hecate-bench** - Comprehensive benchmark suite
  ```bash
  hecate-bench all           # Run all benchmarks
  hecate-bench cpu           # CPU performance tests
  hecate-bench gpu           # GPU compute tests
  hecate-bench ai            # AI/ML workload tests
  hecate-bench stress        # Stress testing
  ```

- **hecate-pkg** - Modern package manager
  ```bash
  hecate-pkg install <package>   # Install with dependency resolution
  hecate-pkg search <query>      # Search packages
  hecate-pkg update              # Update all packages
  hecate-pkg sync                # Sync repositories
  ```

### Web Dashboard

Access the monitoring dashboard at http://localhost:3000 after starting `hecate-monitor`:
- Real-time system metrics
- GPU power management
- Process monitoring
- Network activity tracking
- Built with Next.js and Shadcn UI

## Shell CLI Commands

Original shell-based commands still available:

```bash
hecate info          # Show system info and applied optimizations
hecate update        # Update system packages and run migrations
hecate optimize      # Re-detect hardware and apply optimizations
hecate driver        # Manage GPU drivers (status/install/remove)
hecate migrate       # Run pending migrations
```

## Building

### Option 1: Docker (Recommended)

```bash
git clone https://github.com/Arakiss/hecate-os.git
cd hecate-os

# Build the Docker image and ISO
docker compose run --rm build

# Or manually:
docker build -f Dockerfile.build -t hecate-builder .
docker run --rm --privileged -v $(pwd):/build hecate-builder
```

### Option 2: Native Build

Requires Ubuntu 24.04:

```bash
# Install dependencies
sudo apt install live-build debootstrap squashfs-tools xorriso isolinux

# Clone and build
git clone https://github.com/Arakiss/hecate-os.git
cd hecate-os
sudo ./build.sh build
```

ISO output: `iso/hecate-os-0.1.0-amd64-YYYYMMDD.iso`

## Performance Claims

These are estimates based on the optimizations applied. No benchmarks yet.

| Change | Expected Gain | Why |
|--------|---------------|-----|
| `mitigations=off` | 5-15% | Removes Spectre/Meltdown overhead |
| Performance governor | 3-8% | No frequency scaling latency |
| ZRAM vs disk swap | 10-20% | Compression faster than disk I/O |
| Tuned sysctl | 2-5% | Better memory/network/scheduler settings |

Real benchmarks will come after community testing on varied hardware.

## Security Trade-offs

HecateOS prioritizes performance over security hardening:

- Spectre/Meltdown mitigations disabled by default
- SSH enabled by default
- Firewall installed but not enabled

See [SECURITY.md](SECURITY.md) for details and how to re-enable protections.

## Contributing

Need testers with:
- AMD Ryzen CPUs (Zen 2, 3, 4)
- AMD GPUs (RX 6000/7000)
- Laptops (battery/thermal management)
- Lower-end hardware (8GB RAM, older GPUs)

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

## Why "HecateOS"?

Named after my cat, who sits at the crossroads between my keyboard and monitor. The Greek goddess Hecate ruled crossroads and magic. This distro lives at the crossroads between Windows dual-boot and Linux, between generic configs and hardware-specific tuning.

## License

MIT. Based on Ubuntu 24.04 LTS by Canonical.

---

**Links:** [Roadmap](docs/ROADMAP.md) · [Security](SECURITY.md) · [Issues](https://github.com/Arakiss/hecate-os/issues)
