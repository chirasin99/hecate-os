#!/bin/bash
#
# HecateOS Fast Build - mmdebstrap method
# 2x faster than traditional debootstrap
#

set -e

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
cd "$SCRIPT_DIR/.."

# Source common functions
source "$SCRIPT_DIR/common.sh" 2>/dev/null || true

# Variables
BUILD_DATE=$(date +%Y%m%d-%H%M%S)
VERSION=$(cat VERSION 2>/dev/null || echo "0.1.0")
ARCH="amd64"
SUITE="noble"  # Ubuntu 24.04
VARIANT="minbase"
OUTPUT_DIR="$SCRIPT_DIR/../iso"
WORK_DIR="$SCRIPT_DIR/work"
CACHE_DIR="$SCRIPT_DIR/cache"
ROOTFS_DIR="$WORK_DIR/rootfs"
ISO_DIR="$WORK_DIR/iso"
ISO_NAME="hecate-os-${VERSION}-mmdebstrap-${BUILD_DATE}.iso"

# Use local apt-cacher-ng if available
APT_PROXY=""
if systemctl is-active --quiet apt-cacher-ng; then
    APT_PROXY="http://127.0.0.1:3142"
    echo -e "${GREEN}Using local apt-cacher-ng proxy${RESET}"
fi

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
echo -e "${CYAN}Fast Build with mmdebstrap (2x faster)${RESET}"
echo ""

# Timer function
start_timer() {
    START_TIME=$(date +%s)
}

end_timer() {
    END_TIME=$(date +%s)
    ELAPSED=$((END_TIME - START_TIME))
    echo -e "${GREEN}Time elapsed: $((ELAPSED / 60))m $((ELAPSED % 60))s${RESET}"
}

# Check root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${YELLOW}Re-running with sudo...${RESET}"
    exec sudo APT_PROXY="$APT_PROXY" "$0" "$@"
fi

# Clean work directory
echo -e "${CYAN}Preparing work directory...${RESET}"
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR" "$ROOTFS_DIR" "$ISO_DIR" "$OUTPUT_DIR" "$CACHE_DIR/packages"

# Start timer
start_timer

# Create base system with mmdebstrap (FAST!)
echo -e "${CYAN}Creating base system with mmdebstrap...${RESET}"
echo -e "${YELLOW}This is 2x faster than debootstrap!${RESET}"

# Build package list
PACKAGES=(
    # Core system
    linux-image-generic-hwe-24.04
    linux-headers-generic-hwe-24.04
    ubuntu-minimal
    ubuntu-standard
    
    # Boot and init
    systemd
    systemd-sysv
    systemd-timesyncd
    casper
    lupin-casper
    
    # Hardware support
    firmware-linux
    linux-firmware
    intel-microcode
    amd64-microcode
    
    # Filesystem
    btrfs-progs
    xfsprogs
    e2fsprogs
    dosfstools
    ntfs-3g
    
    # Network
    network-manager
    iproute2
    iputils-ping
    dhcpcd5
    wireless-tools
    wpasupplicant
    
    # Tools
    vim-tiny
    nano
    htop
    curl
    wget
    git
    sudo
    bash-completion
    
    # HecateOS specific
    build-essential
    cargo
    rustc
    python3-minimal
)

PACKAGE_LIST=$(IFS=,; echo "${PACKAGES[*]}")

# Use mmdebstrap with caching
MMDEBSTRAP_OPTS=(
    --variant="$VARIANT"
    --include="$PACKAGE_LIST"
    --architectures="$ARCH"
    --aptopt="Dir::Cache::Archives \"$CACHE_DIR/packages\""
)

if [ -n "$APT_PROXY" ]; then
    MMDEBSTRAP_OPTS+=(--aptopt="Acquire::http::Proxy \"$APT_PROXY\"")
fi

mmdebstrap "${MMDEBSTRAP_OPTS[@]}" \
    "$SUITE" \
    "$ROOTFS_DIR" \
    "http://archive.ubuntu.com/ubuntu"

echo -e "${GREEN}✓ Base system created${RESET}"

# Configure the system
echo -e "${CYAN}Configuring system...${RESET}"

# Copy HecateOS files
if [ -d "config/includes.chroot" ]; then
    cp -r config/includes.chroot/* "$ROOTFS_DIR/"
fi

# Copy Rust binaries if built
if [ -d "rust/target/release" ]; then
    mkdir -p "$ROOTFS_DIR/usr/local/bin"
    find rust/target/release -maxdepth 1 -type f -executable \
        -name "hecate-*" -exec cp {} "$ROOTFS_DIR/usr/local/bin/" \;
fi

# Set hostname
echo "hecate-os" > "$ROOTFS_DIR/etc/hostname"

# Configure networking
cat > "$ROOTFS_DIR/etc/hosts" << EOF
127.0.0.1   localhost
127.0.1.1   hecate-os

# IPv6
::1     ip6-localhost ip6-loopback
fe00::0 ip6-localnet
ff00::0 ip6-mcastprefix
ff02::1 ip6-allnodes
ff02::2 ip6-allrouters
EOF

# Create user
chroot "$ROOTFS_DIR" useradd -m -s /bin/bash -G sudo hecate || true
echo "hecate:hecate" | chroot "$ROOTFS_DIR" chpasswd

# Enable services
chroot "$ROOTFS_DIR" systemctl enable NetworkManager || true
chroot "$ROOTFS_DIR" systemctl enable systemd-timesyncd || true

# Clean apt cache in rootfs
chroot "$ROOTFS_DIR" apt-get clean
rm -rf "$ROOTFS_DIR/var/lib/apt/lists/"*

# Create squashfs
echo -e "${CYAN}Creating squashfs filesystem...${RESET}"
mkdir -p "$ISO_DIR/casper"

# Use parallel compression for speed
mksquashfs "$ROOTFS_DIR" "$ISO_DIR/casper/filesystem.squashfs" \
    -comp xz \
    -processors $(nproc) \
    -b 1M \
    -Xbcj x86 \
    -Xdict-size 100%

# Calculate size
printf $(du -sx --block-size=1 "$ROOTFS_DIR" | cut -f1) > "$ISO_DIR/casper/filesystem.size"

# Copy kernel and initrd
echo -e "${CYAN}Copying kernel and initrd...${RESET}"
cp "$ROOTFS_DIR"/boot/vmlinuz-* "$ISO_DIR/casper/vmlinuz"
cp "$ROOTFS_DIR"/boot/initrd.img-* "$ISO_DIR/casper/initrd"

# Create GRUB configuration
echo -e "${CYAN}Creating bootloader...${RESET}"
mkdir -p "$ISO_DIR/boot/grub"
cat > "$ISO_DIR/boot/grub/grub.cfg" << 'EOF'
set timeout=10
set default=0

menuentry "HecateOS Live" {
    linux /casper/vmlinuz boot=casper quiet splash ---
    initrd /casper/initrd
}

menuentry "HecateOS Live (safe graphics)" {
    linux /casper/vmlinuz boot=casper nomodeset quiet splash ---
    initrd /casper/initrd
}

menuentry "Memory Test" {
    linux16 /casper/memtest
}
EOF

# Create ISO
echo -e "${CYAN}Creating ISO image...${RESET}"
xorriso -as mkisofs \
    -iso-level 3 \
    -full-iso9660-filenames \
    -rational-rock \
    -volid "HECATE_OS" \
    -appid "HecateOS Live ISO" \
    -publisher "HecateOS Team" \
    -preparer "mmdebstrap build system" \
    -eltorito-boot boot/grub/i386-pc/eltorito.img \
    -no-emul-boot \
    -boot-load-size 4 \
    -boot-info-table \
    -eltorito-catalog boot/grub/boot.cat \
    -eltorito-alt-boot \
    -e EFI/efiboot.img \
    -no-emul-boot \
    -isohybrid-mbr /usr/lib/ISOLINUX/isohdpfx.bin \
    -isohybrid-gpt-basdat \
    -output "$OUTPUT_DIR/$ISO_NAME" \
    "$ISO_DIR"

# End timer
end_timer

# Generate checksums
echo -e "${CYAN}Generating checksums...${RESET}"
cd "$OUTPUT_DIR"
sha256sum "$ISO_NAME" > "$ISO_NAME.sha256"
md5sum "$ISO_NAME" > "$ISO_NAME.md5"

# Create latest symlink
ln -sf "$ISO_NAME" "hecate-os-latest-mmdebstrap.iso"

# Success
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${GREEN}✓ ISO build successful!${RESET}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "ISO: ${CYAN}$OUTPUT_DIR/$ISO_NAME${RESET}"
echo -e "Size: ${CYAN}$(du -h "$OUTPUT_DIR/$ISO_NAME" | cut -f1)${RESET}"
echo ""
echo -e "${CYAN}Test with:${RESET} $SCRIPT_DIR/test-local.sh $ISO_NAME"

# Clean up work directory
echo -e "${CYAN}Cleaning up...${RESET}"
rm -rf "$WORK_DIR"