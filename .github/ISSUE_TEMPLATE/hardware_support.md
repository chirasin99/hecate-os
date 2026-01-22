---
name: Hardware Support Request
about: Request support for specific hardware
title: '[HARDWARE] '
labels: hardware-support
assignees: ''
---

## Hardware Information

### Component to Support
- [ ] CPU
- [ ] GPU
- [ ] Motherboard
- [ ] Network Card
- [ ] Storage Controller
- [ ] Other: ___________

### Specific Model
Manufacturer and exact model:
```
(e.g., AMD Ryzen 9 7950X, Intel Arc A770, etc.)
```

### Current Status
What happens when you try HecateOS on this hardware?
- [ ] Not detected at all
- [ ] Detected but not optimized
- [ ] Partially working
- [ ] Errors/crashes
- [ ] Haven't tried yet

### Hardware Detection Output
Run `sudo hecate-hardware-detect` and paste output:
```
(paste here)
```

### System Information
Run these commands and paste output:
```bash
lscpu | head -20
```
```
(paste here)
```

```bash
lspci | grep -E "VGA|3D|Display"
```
```
(paste here)
```

```bash
lsusb
```
```
(paste here)
```

## Testing Availability
- [ ] I have this hardware and can test
- [ ] I can provide remote access for testing
- [ ] I can run test commands and report back
- [ ] I just want support added

## Additional Information
Any other details, special features, or links to hardware documentation.