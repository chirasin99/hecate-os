#!/bin/bash
#
# HecateOS Ultra-Fast Build - bootc method
# Container-native OS building with excellent caching
#

set -e

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
cd "$SCRIPT_DIR/.."

# Variables
BUILD_DATE=$(date +%Y%m%d-%H%M%S)
VERSION=$(cat VERSION 2>/dev/null || echo "0.1.0")
OUTPUT_DIR="$SCRIPT_DIR/../iso"
CONTAINER_NAME="hecate-os:${VERSION}"
REGISTRY="localhost"
ISO_NAME="hecate-os-${VERSION}-bootc-${BUILD_DATE}.iso"

# Colors
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

echo -e "${PURPLE}╦ ╦┌─┐┌─┐┌─┐┌┬┐┌─┐╔═╗╔═╗${RESET}"
echo -e "${PURPLE}╠═╣├┤ │  ├─┤ │ ├┤ ║ ║╚═╗${RESET}"
echo -e "${PURPLE}╩ ╩└─┘└─┘┴ ┴ ┴ └─┘╚═╝╚═╝${RESET}"
echo -e "${CYAN}Ultra-Fast Container-Based Build${RESET}"
echo ""

# Timer
start_timer() {
    START_TIME=$(date +%s)
}

end_timer() {
    END_TIME=$(date +%s)
    ELAPSED=$((END_TIME - START_TIME))
    echo -e "${GREEN}Time elapsed: $((ELAPSED / 60))m $((ELAPSED % 60))s${RESET}"
}

# Check if podman is available
if ! command -v podman &> /dev/null; then
    echo -e "${RED}Podman is required for bootc builds${RESET}"
    echo "Run: sudo ./install-deps.sh"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
start_timer

# Create Containerfile for HecateOS
echo -e "${CYAN}Creating HecateOS container image...${RESET}"

cat > "$SCRIPT_DIR/Containerfile" << 'EOF'
# HecateOS Bootable Container Image
FROM ubuntu:24.04

# Avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8

# Install bootc for container-native OS
RUN apt-get update && \
    apt-get install -y \
        curl \
        gnupg \
        software-properties-common && \
    # Note: In production, we'd install actual bootc here
    # For now, we'll prepare the system
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install kernel and boot requirements
RUN apt-get update && \
    apt-get install -y \
        linux-image-generic-hwe-24.04 \
        linux-headers-generic-hwe-24.04 \
        systemd \
        systemd-sysv \
        systemd-boot \
        dracut \
        grub-pc-bin \
        grub-efi-amd64-bin \
        grub-efi-amd64-signed \
        shim-signed && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install base system
RUN apt-get update && \
    apt-get install -y \
        ubuntu-minimal \
        ubuntu-standard \
        network-manager \
        sudo \
        vim \
        nano \
        htop \
        curl \
        wget \
        git \
        build-essential \
        python3-minimal && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Hardware support
RUN apt-get update && \
    apt-get install -y \
        firmware-linux \
        linux-firmware \
        intel-microcode \
        amd64-microcode \
        mesa-utils \
        vulkan-tools && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install Rust for HecateOS components
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    rustup default stable

# Copy HecateOS components
COPY rust/target/release/hecate-* /usr/local/bin/ 2>/dev/null || true
COPY config/includes.chroot/ / 2>/dev/null || true

# Configure system
RUN echo "hecate-os" > /etc/hostname && \
    useradd -m -s /bin/bash -G sudo hecate && \
    echo "hecate:hecate" | chpasswd && \
    echo "hecate ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Enable services
RUN systemctl enable NetworkManager || true && \
    systemctl enable systemd-timesyncd || true

# Clean up
RUN apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    rm -rf /var/cache/apt/archives/*

# Set up bootc labels
LABEL org.opencontainers.image.title="HecateOS"
LABEL org.opencontainers.image.description="Performance-optimized Linux with automatic hardware detection"
LABEL org.opencontainers.image.version="${VERSION}"
LABEL ostree.bootable="true"

# Default command
CMD ["/sbin/init"]
EOF

# Build container image (uses cache layers!)
echo -e "${CYAN}Building container image (cached layers = fast!)...${RESET}"
podman build \
    --tag "${REGISTRY}/${CONTAINER_NAME}" \
    --file "$SCRIPT_DIR/Containerfile" \
    --layers \
    --cache-from "${REGISTRY}/${CONTAINER_NAME}" \
    .

echo -e "${GREEN}✓ Container image built${RESET}"

# Alternative 1: Create ISO using bootc-image-builder (if available)
if command -v bootc-image-builder &> /dev/null; then
    echo -e "${CYAN}Creating ISO with bootc-image-builder...${RESET}"
    
    # Create config for bootc-image-builder
    cat > "$SCRIPT_DIR/bootc-config.toml" << EOF
[[customizations.user]]
name = "hecate"
password = "hecate"
groups = ["wheel", "sudo"]
EOF

    bootc-image-builder \
        --type iso \
        --config "$SCRIPT_DIR/bootc-config.toml" \
        --output "$OUTPUT_DIR" \
        "${REGISTRY}/${CONTAINER_NAME}"
        
    mv "$OUTPUT_DIR/bootiso.iso" "$OUTPUT_DIR/$ISO_NAME" 2>/dev/null || true
    
else
    # Alternative 2: Manual ISO creation from container
    echo -e "${YELLOW}bootc-image-builder not found, using manual method${RESET}"
    echo -e "${CYAN}Exporting container to filesystem...${RESET}"
    
    WORK_DIR="$SCRIPT_DIR/work-bootc"
    rm -rf "$WORK_DIR"
    mkdir -p "$WORK_DIR/rootfs" "$WORK_DIR/iso/casper"
    
    # Create container and export filesystem
    CONTAINER_ID=$(podman create "${REGISTRY}/${CONTAINER_NAME}")
    podman export "$CONTAINER_ID" | tar -C "$WORK_DIR/rootfs" -xf -
    podman rm "$CONTAINER_ID"
    
    # Create squashfs
    echo -e "${CYAN}Creating squashfs (parallel compression)...${RESET}"
    mksquashfs "$WORK_DIR/rootfs" "$WORK_DIR/iso/casper/filesystem.squashfs" \
        -comp xz \
        -processors $(nproc) \
        -b 1M
    
    # Copy kernel and initrd
    cp "$WORK_DIR/rootfs"/boot/vmlinuz-* "$WORK_DIR/iso/casper/vmlinuz" 2>/dev/null || \
        echo -e "${YELLOW}Warning: Kernel not found in container${RESET}"
    cp "$WORK_DIR/rootfs"/boot/initrd.img-* "$WORK_DIR/iso/casper/initrd" 2>/dev/null || \
        echo -e "${YELLOW}Warning: Initrd not found in container${RESET}"
    
    # Create basic GRUB config
    mkdir -p "$WORK_DIR/iso/boot/grub"
    cat > "$WORK_DIR/iso/boot/grub/grub.cfg" << 'EOF'
set timeout=10
set default=0

menuentry "HecateOS Live (Container Build)" {
    linux /casper/vmlinuz boot=casper quiet splash ---
    initrd /casper/initrd
}
EOF
    
    # Create ISO
    echo -e "${CYAN}Creating ISO image...${RESET}"
    genisoimage -r -V "HECATE_BOOTC" \
        -cache-inodes -J -l \
        -b isolinux/isolinux.bin \
        -c isolinux/boot.cat \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        -o "$OUTPUT_DIR/$ISO_NAME" \
        "$WORK_DIR/iso" 2>/dev/null || \
    xorriso -as mkisofs \
        -rational-rock \
        -volid "HECATE_BOOTC" \
        -output "$OUTPUT_DIR/$ISO_NAME" \
        "$WORK_DIR/iso"
    
    # Cleanup
    rm -rf "$WORK_DIR"
fi

end_timer

# Generate checksums
echo -e "${CYAN}Generating checksums...${RESET}"
cd "$OUTPUT_DIR"
sha256sum "$ISO_NAME" > "$ISO_NAME.sha256"

# Create latest symlink
ln -sf "$ISO_NAME" "hecate-os-latest-bootc.iso"

# Show results
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${GREEN}✓ ISO build successful!${RESET}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "ISO: ${CYAN}$OUTPUT_DIR/$ISO_NAME${RESET}"
echo -e "Size: ${CYAN}$(du -h "$OUTPUT_DIR/$ISO_NAME" | cut -f1)${RESET}"
echo ""
echo -e "${CYAN}Container image cached for next build!${RESET}"
echo -e "${CYAN}Next build will be even faster!${RESET}"
echo ""
echo -e "Test with: ${CYAN}$SCRIPT_DIR/test-local.sh $ISO_NAME${RESET}"