# Contributing to HecateOS

First off, thanks for taking the time to contribute! 🎉

HecateOS is a community-driven project and we welcome contributions of all kinds: code, documentation, bug reports, feature requests, and more.

## Code of Conduct

Be cool. Don't be an ass. We're all here to make Linux better.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

**Great Bug Reports** include:
- Your hardware specs (run `hecate-info --all`)
- Steps to reproduce
- Expected behavior
- Actual behavior
- Logs if applicable (`journalctl -b`)

### Suggesting Features

Feature requests are welcome! Please provide:
- Use case - why is this needed?
- Expected behavior
- Would you be willing to implement it?

### Your First Code Contribution

Unsure where to begin? Look for issues labeled:
- `good first issue` - Simple fixes
- `help wanted` - We need your expertise
- `hardware:amd` - AMD-specific issues (we need AMD testers!)

### Pull Requests

1. Fork the repo
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Development Setup

```bash
# Clone your fork
git clone git@github.com:your-username/hecate-os.git
cd hecate-os

# Install build dependencies (Ubuntu 22.04+)
sudo apt install live-build debootstrap squashfs-tools xorriso

# Build ISO (takes 30-60 min)
sudo ./build.sh build

# Test in VM
./build.sh test
```

## Commit Message Convention

We use conventional commits:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only
- `style:` Formatting, no code change
- `refactor:` Code change that neither fixes a bug nor adds a feature
- `perf:` Performance improvement
- `test:` Adding tests
- `chore:` Updating build tasks, package manager configs, etc

Examples:
```
feat: Add support for AMD Ryzen 9000 series
fix: NVIDIA driver installation on GTX 1660
docs: Update hardware compatibility matrix
perf: Optimize ZRAM configuration for 16GB systems
```

## Testing

### Hardware We Need Testing On

**Most Needed:**
- AMD Ryzen (any generation)
- AMD Radeon GPUs
- Intel Arc GPUs
- Laptops (battery optimization)
- Older hardware (pre-2020)

**How to Test:**
1. Build or download ISO
2. Install on test machine (or VM)
3. Run hardware detection: `sudo hecate-hardware-detect`
4. Run benchmarks: `sudo hecate-benchmark`
5. Share results in an issue

## Project Structure

```
hecate-os/
├── config/           # Live-build configuration
│   ├── hooks/       # Build hooks
│   ├── includes.chroot/  # Files to include in ISO
│   └── package-lists/    # Package definitions
├── scripts/         # HecateOS tools and utilities
├── editions/        # Edition-specific builds
└── docs/           # Documentation
```

## Adding Hardware Support

To add support for new hardware:

1. Update `scripts/hardware-detector.sh`:
```bash
# Add detection logic for your hardware
if [[ "$CPU_MODEL" =~ "Your CPU" ]]; then
    CPU_GENERATION="your-gen"
    # Add specific optimizations
fi
```

2. Update `scripts/apply-optimizations.sh`:
```bash
# Add optimization logic
if [[ "$CPU_GENERATION" == "your-gen" ]]; then
    # Apply your optimizations
fi
```

3. Test thoroughly and submit PR

## Documentation

Documentation improvements are always welcome! Areas that need work:
- Hardware compatibility list
- Performance tuning guides
- Troubleshooting guides
- Translations

## Financial Support

If you want to support the project financially:
- Sponsor via GitHub Sponsors (coming soon)
- Donate hardware for testing
- Provide cloud resources for CI/CD

## Recognition

Contributors will be added to:
- CONTRIBUTORS.md file
- GitHub contributors graph
- Special thanks in release notes

## Questions?

- Discord: https://discord.gg/hecate-os
- GitHub Discussions: Enable in repo settings
- Email: (add your email if you want)

## License

By contributing to HecateOS, you agree that your contributions will be licensed under the MIT License.