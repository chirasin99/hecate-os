# HecateOS

A Linux distribution that detects your hardware and applies specific optimizations automatically. Based on Ubuntu 24.04 LTS.

> **Status: Alpha (v0.1.0)** — Framework complete, tested on one machine. No ISO releases yet.

## The Problem

Most Linux distros ship with generic configs. You install Ubuntu, then spend hours tweaking sysctl, GRUB parameters, GPU drivers, and kernel settings. Or you don't, and leave performance on the table.

## What HecateOS Does

On first boot, HecateOS runs `hardware-detector.sh` which:

1. **Detects your hardware** — CPU model/generation, GPU vendor/model/VRAM, RAM amount/speed, storage type
2. **Creates a profile** — Classifies your system as Ultimate, Gaming, Developer, Server, or Minimal
3. **Applies optimizations** — Sets kernel parameters, sysctl values, GPU settings, I/O schedulers specific to YOUR hardware

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Hardware Detect │ ──▶ │ Profile Create  │ ──▶ │ Apply Tuning    │
│                 │     │                 │     │                 │
│ • CPU gen       │     │ • Ultimate      │     │ • sysctl.conf   │
│ • GPU tier      │     │ • Gaming        │     │ • GRUB params   │
│ • RAM amount    │     │ • Developer     │     │ • GPU settings  │
│ • Storage type  │     │ • Server        │     │ • I/O scheduler │
└─────────────────┘     │ • Minimal       │     └─────────────────┘
                        └─────────────────┘
```

## What Gets Tuned

**CPU** — Intel P-State or AMD P-State governor, C-State limits, turbo boost settings

**Memory** — Swappiness (10 for high RAM, 60 for low), dirty ratios, ZRAM compression ratio, transparent hugepages

**GPU (NVIDIA)** — Driver version by generation (570 for RTX 40, 535 for RTX 30, etc.), persistence mode, power limits, compute mode

**Storage** — I/O scheduler (none for NVMe Gen4+, mq-deadline for older), read-ahead values

**Kernel** — `mitigations=off` for ~10% perf gain (configurable), `intel_pstate=active`, IOMMU, PCIe ASPM off

## Tested Hardware

Actually tested on:
- Intel Core i9-13900K
- NVIDIA RTX 4090
- 128GB DDR5-6400
- Samsung 990 PRO NVMe

Should work on (detection logic exists but untested):
- Intel 10th gen+
- AMD Ryzen (Zen 2+)
- NVIDIA GTX 10 series+
- AMD GPUs (basic support)
- 8GB-512GB RAM
- Any NVMe/SATA SSD/HDD

## Project Structure

```
hecate-os/
├── scripts/
│   ├── hardware-detector.sh    # Detects and profiles hardware
│   ├── apply-optimizations.sh  # Applies profile-specific tuning
│   ├── hecate-driver-installer.sh  # GPU driver selection
│   └── hecate-benchmark.sh     # Performance testing
├── config/
│   ├── package-lists/          # Packages to install
│   ├── includes.chroot/        # System configs (sysctl, GRUB, docker)
│   └── hooks/                  # Build-time scripts
└── build.sh                    # Main build script
```

## Building

```bash
# Install dependencies (Ubuntu 22.04+)
sudo apt install live-build debootstrap squashfs-tools xorriso

# Clone and build
git clone https://github.com/Arakiss/hecate-os.git
cd hecate-os
sudo ./build.sh
```

ISO output: `iso/hecate-os-0.1.0-amd64.iso`

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
- Lower-end hardware (validate Lite edition)

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

## Why "HecateOS"?

Named after my cat, who sits at the crossroads between my keyboard and monitor. The Greek goddess Hecate ruled crossroads and magic. This distro lives at the crossroads between Windows dual-boot and Linux, between generic configs and hardware-specific tuning.

## License

MIT. Based on Ubuntu 24.04 LTS by Canonical.

---

**Links:** [Roadmap](docs/ROADMAP.md) · [Security](SECURITY.md) · [Issues](https://github.com/Arakiss/hecate-os/issues)
