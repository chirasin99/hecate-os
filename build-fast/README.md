# HecateOS Fast Build System

## Overview
Modern, fast ISO building system for HecateOS using container-native tools and intelligent caching.

## Build Methods (Fastest to Slowest)

### 1. Bootc Method (5-10 min) - RECOMMENDED
Uses container images to build ISOs with excellent caching.
```bash
./build-bootc.sh
```

### 2. Mmdebstrap Method (10-15 min)
2x faster than debootstrap with better dependency resolution.
```bash
./build-mmdebstrap.sh
```

### 3. Docker Export Method (8-12 min)
Builds system in Docker, exports to ISO.
```bash
./build-docker.sh
```

### 4. Legacy Live-build (30-60 min)
Original method for compatibility.
```bash
../build.sh
```

## Quick Start

```bash
# Install dependencies
sudo ./install-deps.sh

# Build ISO (fastest method)
./build-bootc.sh

# Test locally before CI
./test-local.sh

# Run CI checks locally
./pre-ci-check.sh
```

## Build Cache
All methods use a shared cache in `cache/` directory:
- Base images
- Package cache
- Squashfs layers
- Container layers

## Performance Benchmarks

| Method | Clean Build | Incremental | Cache Size |
|--------|------------|-------------|------------|
| Bootc | 8 min | 2 min | 2GB |
| Mmdebstrap | 12 min | 5 min | 1.5GB |
| Docker | 10 min | 3 min | 3GB |
| Live-build | 45 min | 30 min | 2GB |