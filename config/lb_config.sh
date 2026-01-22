#!/bin/bash
# HecateOS Live-Build Configuration Script

set -e

echo "Initializing HecateOS live-build configuration..."

lb config \
    --distribution noble \
    --mode ubuntu \
    --architectures amd64 \
    --linux-flavours generic-hwe-24.04 \
    --binary-images iso-hybrid \
    --archive-areas "main restricted universe multiverse" \
    --apt-indices true \
    --apt-recommends true \
    --cache true \
    --debian-installer live \
    --debian-installer-gui false \
    --win32-loader false \
    --iso-application "HecateOS" \
    --iso-volume "HecateOS-24.04" \
    --iso-publisher "HecateOS Team" \
    --iso-preparer "live-build" \
    --memtest none \
    --bootappend-live "boot=casper quiet splash intel_pstate=active intel_iommu=on iommu=pt nvme_core.default_ps_max_latency_us=0 pcie_aspm=off processor.max_cstate=1 intel_idle.max_cstate=0 mitigations=off" \
    --security true \
    --updates true \
    --backports false \
    --parent-mirror-bootstrap "http://archive.ubuntu.com/ubuntu/" \
    --parent-mirror-chroot "http://archive.ubuntu.com/ubuntu/" \
    --parent-mirror-chroot-security "http://security.ubuntu.com/ubuntu/" \
    --parent-mirror-binary "http://archive.ubuntu.com/ubuntu/" \
    --parent-mirror-binary-security "http://security.ubuntu.com/ubuntu/" \
    --mirror-bootstrap "http://archive.ubuntu.com/ubuntu/" \
    --mirror-chroot "http://archive.ubuntu.com/ubuntu/" \
    --mirror-chroot-security "http://security.ubuntu.com/ubuntu/" \
    --mirror-binary "http://archive.ubuntu.com/ubuntu/" \
    --mirror-binary-security "http://security.ubuntu.com/ubuntu/"

echo "Live-build configuration complete!"