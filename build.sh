#!/bin/bash
#
# HecateOS ISO Build Script
# Builds the custom HecateOS ISO image
#

set -e

# Colors
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
WHITE='\033[1;37m'
RESET='\033[0m'

# Variables
BUILD_DIR=$(dirname "$(readlink -f "$0")")
BUILD_DATE=$(date +%Y%m%d)

# Read version from VERSION file (single source of truth)
if [ -f "$BUILD_DIR/VERSION" ]; then
    VERSION=$(cat "$BUILD_DIR/VERSION" | tr -d '[:space:]')
else
    VERSION="0.1.0"
    echo "Warning: VERSION file not found, using default $VERSION"
fi

ISO_NAME="hecate-os-${VERSION}-amd64"
CODENAME="crossroads"

# ASCII Art
show_banner() {
    echo -e "${PURPLE}"
    cat << 'EOF'
╦ ╦┌─┐┌─┐┌─┐┌┬┐┌─┐╔═╗╔═╗
╠═╣├┤ │  ├─┤ │ ├┤ ║ ║╚═╗
╩ ╩└─┘└─┘┴ ┴ ┴ └─┘╚═╝╚═╝

ISO Build System
EOF
    echo -e "${RESET}"
    echo -e "${WHITE}Version: $VERSION ($CODENAME)${RESET}"
    echo -e "${WHITE}Build Date: $BUILD_DATE${RESET}"
    echo -e "${WHITE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}\n"
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        echo -e "${RED}This script must be run as root${RESET}"
        echo "Usage: sudo $0 [options]"
        exit 1
    fi
}

# Check dependencies
check_dependencies() {
    echo -e "${CYAN}Checking build dependencies...${RESET}"
    
    MISSING_DEPS=""
    for dep in lb debootstrap squashfs-tools xorriso; do
        if ! command -v $dep &> /dev/null; then
            MISSING_DEPS="$MISSING_DEPS $dep"
        fi
    done
    
    if [ ! -z "$MISSING_DEPS" ]; then
        echo -e "${YELLOW}Missing dependencies:$MISSING_DEPS${RESET}"
        echo -e "${CYAN}Installing dependencies...${RESET}"
        apt update
        apt install -y live-build debootstrap squashfs-tools xorriso isolinux
    fi
    
    echo -e "${GREEN}✓ All dependencies satisfied${RESET}\n"
}

# Clean previous builds
clean_build() {
    echo -e "${CYAN}Cleaning previous builds...${RESET}"
    cd "$BUILD_DIR"
    
    # Clean live-build
    lb clean --purge 2>/dev/null || true
    
    # Remove old files
    rm -rf .build auto cache chroot* binary* *.iso *.img *.list *.packages *.files *.contents *.zsync *.log
    
    # Clean ISO directory
    rm -rf iso/*
    
    echo -e "${GREEN}✓ Clean complete${RESET}\n"
}

# Initialize live-build configuration
init_config() {
    echo -e "${CYAN}Initializing live-build configuration...${RESET}"
    cd "$BUILD_DIR"
    
    # Run configuration script
    if [ -f "config/lb_config.sh" ]; then
        bash config/lb_config.sh
    else
        # Fallback configuration
        lb config \
            --distribution noble \
            --mode ubuntu \
            --architectures amd64 \
            --linux-flavours generic-hwe-24.04 \
            --binary-images iso-hybrid \
            --archive-areas "main restricted universe multiverse" \
            --debian-installer live \
            --debian-installer-gui false \
            --memtest none \
            --iso-application "HecateOS" \
            --iso-volume "HecateOS-$VERSION" \
            --iso-publisher "HecateOS Team" \
            --bootappend-live "boot=casper quiet splash intel_pstate=active mitigations=off"
    fi
    
    echo -e "${GREEN}✓ Configuration initialized${RESET}\n"
}

# Build the ISO
build_iso() {
    echo -e "${CYAN}Building HecateOS ISO...${RESET}"
    echo -e "${YELLOW}This will take 30-60 minutes depending on your system and internet speed${RESET}\n"
    
    cd "$BUILD_DIR"
    
    # Start build with logging
    lb build 2>&1 | tee build-${BUILD_DATE}.log
    
    # Check if build succeeded
    if [ -f "live-image-amd64.hybrid.iso" ]; then
        # Move and rename ISO
        mv live-image-amd64.hybrid.iso "iso/${ISO_NAME}-${BUILD_DATE}.iso"
        
        # Create symlink to latest
        cd iso/
        ln -sf "${ISO_NAME}-${BUILD_DATE}.iso" "${ISO_NAME}-latest.iso"
        cd ..
        
        # Generate checksums
        cd iso/
        sha256sum "${ISO_NAME}-${BUILD_DATE}.iso" > "${ISO_NAME}-${BUILD_DATE}.iso.sha256"
        md5sum "${ISO_NAME}-${BUILD_DATE}.iso" > "${ISO_NAME}-${BUILD_DATE}.iso.md5"
        cd ..
        
        echo -e "\n${GREEN}✓ Build successful!${RESET}"
        echo -e "${WHITE}ISO Location: ${CYAN}iso/${ISO_NAME}-${BUILD_DATE}.iso${RESET}"
        echo -e "${WHITE}ISO Size: ${CYAN}$(du -h iso/${ISO_NAME}-${BUILD_DATE}.iso | cut -f1)${RESET}"
        echo -e "${WHITE}SHA256: ${CYAN}$(cat iso/${ISO_NAME}-${BUILD_DATE}.iso.sha256 | cut -d' ' -f1)${RESET}"
    else
        echo -e "\n${RED}✗ Build failed! Check build-${BUILD_DATE}.log for details${RESET}"
        exit 1
    fi
}

# Create USB instructions
show_usb_instructions() {
    echo -e "\n${WHITE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${WHITE}Creating Bootable USB:${RESET}"
    echo -e "${WHITE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
    echo -e "${CYAN}Method 1: Using dd (Linux/macOS)${RESET}"
    echo -e "  ${WHITE}sudo dd if=iso/${ISO_NAME}-latest.iso of=/dev/sdX bs=4M status=progress sync${RESET}"
    echo ""
    echo -e "${CYAN}Method 2: Using Ventoy (Recommended for multi-boot)${RESET}"
    echo -e "  1. Install Ventoy on USB drive"
    echo -e "  2. Copy ISO to USB drive"
    echo -e "  3. Boot and select HecateOS from menu"
    echo ""
    echo -e "${CYAN}Method 3: Using Rufus (Windows)${RESET}"
    echo -e "  1. Download Rufus from https://rufus.ie"
    echo -e "  2. Select ISO and USB drive"
    echo -e "  3. Use DD mode when prompted"
    echo ""
    echo -e "${YELLOW}⚠ WARNING: This will erase all data on the USB drive!${RESET}"
}

# Test ISO in VM
test_iso() {
    echo -e "\n${CYAN}Testing ISO in QEMU...${RESET}"
    
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        echo -e "${YELLOW}QEMU not installed. Installing...${RESET}"
        apt install -y qemu-system-x86
    fi
    
    ISO_PATH="iso/${ISO_NAME}-latest.iso"
    
    if [ -f "$ISO_PATH" ]; then
        echo -e "${WHITE}Starting VM with:${RESET}"
        echo -e "  RAM: 4GB"
        echo -e "  CPU: 4 cores"
        echo -e "  UEFI: Enabled"
        echo ""
        echo -e "${YELLOW}Press Ctrl+C to stop the VM${RESET}"
        
        qemu-system-x86_64 \
            -cdrom "$ISO_PATH" \
            -m 4G \
            -smp 4 \
            -enable-kvm \
            -cpu host \
            -bios /usr/share/ovmf/OVMF.fd \
            -vga virtio \
            -display gtk
    else
        echo -e "${RED}ISO not found. Build it first with: $0 build${RESET}"
    fi
}

# Main function
main() {
    case "$1" in
        clean)
            show_banner
            check_root
            clean_build
            ;;
        build)
            show_banner
            check_root
            check_dependencies
            clean_build
            init_config
            build_iso
            show_usb_instructions
            ;;
        rebuild)
            show_banner
            check_root
            check_dependencies
            init_config
            build_iso
            show_usb_instructions
            ;;
        test)
            show_banner
            test_iso
            ;;
        *)
            show_banner
            echo "Usage: $0 {build|rebuild|clean|test}"
            echo ""
            echo "Commands:"
            echo "  build    - Clean and build new ISO"
            echo "  rebuild  - Build ISO without cleaning"
            echo "  clean    - Remove all build artifacts"
            echo "  test     - Test ISO in QEMU/KVM"
            echo ""
            echo "Examples:"
            echo "  sudo $0 build    # Full build from scratch"
            echo "  sudo $0 rebuild  # Quick rebuild"
            echo "  $0 test          # Test in VM"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"