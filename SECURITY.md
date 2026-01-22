# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Important Security Notice

HecateOS makes performance-first decisions that may impact security:

### ⚠️ Security Trade-offs for Performance

1. **CPU Mitigations Disabled by Default**
   - Spectre/Meltdown mitigations are OFF
   - Gives +5-15% performance boost
   - Increases vulnerability to side-channel attacks
   - **Recommendation**: Only use on trusted networks

2. **Permissive Kernel Parameters**
   - `kernel.kptr_restrict=0` (kernel pointers visible)
   - `kernel.perf_event_paranoid=-1` (full perf access)
   - Better debugging and profiling
   - **Recommendation**: Harden for production servers

3. **Development Tools Pre-installed**
   - Compilers, debuggers included
   - Larger attack surface
   - **Recommendation**: Use Server edition for production

### Re-enabling Security Features

If you need better security over performance:

```bash
# Re-enable CPU mitigations
sudo sed -i 's/mitigations=off/mitigations=auto/' /etc/default/grub
sudo update-grub
sudo reboot

# Harden kernel parameters
echo "kernel.kptr_restrict=2" | sudo tee -a /etc/sysctl.d/99-security.conf
echo "kernel.perf_event_paranoid=3" | sudo tee -a /etc/sysctl.d/99-security.conf
sudo sysctl -p
```

## Reporting a Vulnerability

**DO NOT** report security vulnerabilities through public GitHub issues.

Instead, please report them via:
1. GitHub Security Advisories (preferred)
2. Email to: (add your security email)
3. Discord DM to maintainers

Please include:
- Type of issue (buffer overflow, SQL injection, cross-site scripting, etc.)
- Full paths of source file(s) related to the issue
- Location of affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce
- Proof-of-concept or exploit code (if possible)
- Impact of the issue

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Patch Development**: Depends on severity
  - Critical: Within 1 week
  - High: Within 2 weeks
  - Medium: Within 1 month
  - Low: Next release

## Security Updates

Security updates will be released as:
- Hot patches for critical vulnerabilities
- Included in next point release for others

Subscribe to security notifications:
- Watch this repo with "Security alerts" enabled
- Join Discord #security channel

## Known Security Considerations

### NVIDIA Proprietary Drivers
- Closed source, can't audit
- Required for GPU functionality
- Keep updated via `apt upgrade`

### Third-party Repositories
HecateOS may add:
- NVIDIA PPA
- Docker CE repository
- Only official sources used

### Network Services
Default enabled services:
- SSH (key-auth recommended)
- Docker daemon (local only)

## Hardening Guide

For production use, see [docs/HARDENING.md](docs/HARDENING.md) for:
- Re-enabling mitigations
- Firewall configuration
- SELinux/AppArmor setup
- Secure boot configuration

## Security Hall of Fame

We thank the following researchers for responsible disclosure:
- (Your name here!)

## License

Security fixes are released under the same MIT license as HecateOS.