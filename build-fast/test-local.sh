#!/bin/bash
#
# Test HecateOS ISO locally before pushing to CI
#

set -e

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
OUTPUT_DIR="$SCRIPT_DIR/../iso"

# Colors
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

# Find ISO to test
ISO_NAME="${1:-}"
if [ -z "$ISO_NAME" ]; then
    # Find latest ISO
    ISO_NAME=$(ls -t "$OUTPUT_DIR"/*.iso 2>/dev/null | head -1)
    if [ -z "$ISO_NAME" ]; then
        echo -e "${RED}No ISO found in $OUTPUT_DIR${RESET}"
        echo "Build one first with:"
        echo "  ./build-bootc.sh      (fastest)"
        echo "  ./build-mmdebstrap.sh (fast)"
        exit 1
    fi
else
    if [ ! -f "$OUTPUT_DIR/$ISO_NAME" ] && [ ! -f "$ISO_NAME" ]; then
        echo -e "${RED}ISO not found: $ISO_NAME${RESET}"
        exit 1
    fi
    [ -f "$ISO_NAME" ] && ISO_NAME=$(readlink -f "$ISO_NAME")
    [ -f "$OUTPUT_DIR/$ISO_NAME" ] && ISO_NAME="$OUTPUT_DIR/$ISO_NAME"
fi

echo -e "${PURPLE}╦ ╦┌─┐┌─┐┌─┐┌┬┐┌─┐╔═╗╔═╗${RESET}"
echo -e "${PURPLE}╠═╣├┤ │  ├─┤ │ ├┤ ║ ║╚═╗${RESET}"
echo -e "${PURPLE}╩ ╩└─┘└─┘┴ ┴ ┴ └─┘╚═╝╚═╝${RESET}"
echo -e "${CYAN}Local ISO Testing Suite${RESET}"
echo ""

ISO_SIZE=$(du -h "$ISO_NAME" | cut -f1)
echo -e "Testing ISO: ${CYAN}$(basename "$ISO_NAME")${RESET}"
echo -e "Size: ${CYAN}${ISO_SIZE}${RESET}"
echo ""

# Function to run tests
run_test() {
    local test_name="$1"
    local test_cmd="$2"
    
    echo -ne "${CYAN}Running: ${test_name}...${RESET} "
    if eval "$test_cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASSED${RESET}"
        return 0
    else
        echo -e "${RED}✗ FAILED${RESET}"
        return 1
    fi
}

# Test suite
echo -e "${CYAN}Running test suite...${RESET}"
echo ""

FAILED_TESTS=0

# Test 1: Check ISO structure
run_test "ISO structure validation" \
    "isoinfo -i '$ISO_NAME' -l | grep -q 'CASPER\\|BOOT\\|GRUB'" || ((FAILED_TESTS++))

# Test 2: Check bootloader
run_test "Bootloader presence" \
    "isoinfo -i '$ISO_NAME' -J -x /boot/grub/grub.cfg > /dev/null" || ((FAILED_TESTS++))

# Test 3: Check filesystem.squashfs
run_test "Squashfs filesystem" \
    "isoinfo -i '$ISO_NAME' -J -x /casper/filesystem.squashfs > /dev/null" || ((FAILED_TESTS++))

# Test 4: Check kernel
run_test "Kernel presence" \
    "isoinfo -i '$ISO_NAME' -J -x /casper/vmlinuz > /dev/null" || ((FAILED_TESTS++))

# Test 5: Check initrd
run_test "Initrd presence" \
    "isoinfo -i '$ISO_NAME' -J -x /casper/initrd > /dev/null" || ((FAILED_TESTS++))

# Test 6: Verify checksums if available
CHECKSUM_FILE="${ISO_NAME}.sha256"
if [ -f "$CHECKSUM_FILE" ]; then
    run_test "Checksum verification" \
        "cd $(dirname '$ISO_NAME') && sha256sum -c $(basename '$CHECKSUM_FILE')" || ((FAILED_TESTS++))
else
    echo -e "${YELLOW}⚠ Checksum file not found, skipping${RESET}"
fi

# Test 7: Check ISO size (should be reasonable)
ISO_SIZE_MB=$(du -m "$ISO_NAME" | cut -f1)
if [ "$ISO_SIZE_MB" -gt 100 ] && [ "$ISO_SIZE_MB" -lt 5000 ]; then
    echo -e "${CYAN}Running: ISO size check...${RESET} ${GREEN}✓ PASSED${RESET} (${ISO_SIZE_MB}MB)"
else
    echo -e "${CYAN}Running: ISO size check...${RESET} ${RED}✗ FAILED${RESET} (${ISO_SIZE_MB}MB seems unusual)"
    ((FAILED_TESTS++))
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

# Results
if [ "$FAILED_TESTS" -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${RESET}"
    echo -e "${GREEN}ISO is ready for testing${RESET}"
else
    echo -e "${RED}✗ $FAILED_TESTS test(s) failed${RESET}"
    echo -e "${YELLOW}Review the ISO build process${RESET}"
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""

# Offer to test in VM
echo -e "${CYAN}Test options:${RESET}"
echo "  1) Quick QEMU test (2GB RAM, no acceleration)"
echo "  2) Full QEMU test (4GB RAM, KVM acceleration)"
echo "  3) VirtualBox test (requires VirtualBox)"
echo "  4) Write to USB (destructive!)"
echo "  5) Skip VM test"
echo ""
read -p "Select option [1-5]: " choice

case "$choice" in
    1)
        echo -e "${CYAN}Starting quick QEMU test...${RESET}"
        qemu-system-x86_64 \
            -cdrom "$ISO_NAME" \
            -m 2048 \
            -display gtk \
            -vga virtio \
            -boot d
        ;;
    2)
        echo -e "${CYAN}Starting full QEMU test with KVM...${RESET}"
        if [ ! -e /dev/kvm ]; then
            echo -e "${YELLOW}KVM not available, falling back to TCG${RESET}"
            ACCEL=""
        else
            ACCEL="-enable-kvm -cpu host"
        fi
        
        # Check for UEFI firmware
        UEFI=""
        if [ -f /usr/share/ovmf/OVMF.fd ]; then
            UEFI="-bios /usr/share/ovmf/OVMF.fd"
            echo -e "${GREEN}Using UEFI boot${RESET}"
        fi
        
        qemu-system-x86_64 \
            -cdrom "$ISO_NAME" \
            -m 4096 \
            -smp 4 \
            $ACCEL \
            $UEFI \
            -display gtk \
            -vga virtio \
            -device e1000,netdev=net0 \
            -netdev user,id=net0 \
            -boot d
        ;;
    3)
        echo -e "${CYAN}Creating VirtualBox VM...${RESET}"
        if ! command -v VBoxManage &> /dev/null; then
            echo -e "${RED}VirtualBox not installed${RESET}"
            exit 1
        fi
        
        VM_NAME="HecateOS-Test-$(date +%s)"
        VBoxManage createvm --name "$VM_NAME" --ostype Ubuntu_64 --register
        VBoxManage modifyvm "$VM_NAME" --memory 4096 --cpus 2 --vram 128
        VBoxManage storagectl "$VM_NAME" --name "IDE Controller" --add ide
        VBoxManage storageattach "$VM_NAME" --storagectl "IDE Controller" \
            --port 0 --device 0 --type dvddrive --medium "$ISO_NAME"
        VBoxManage startvm "$VM_NAME"
        
        echo -e "${YELLOW}Remember to delete the VM when done:${RESET}"
        echo "  VBoxManage unregistervm $VM_NAME --delete"
        ;;
    4)
        echo -e "${RED}WARNING: This will destroy all data on the USB drive!${RESET}"
        echo "Available devices:"
        lsblk -d -o NAME,SIZE,MODEL | grep -E "^sd|^nvme"
        echo ""
        read -p "Enter device (e.g., /dev/sdb): " USB_DEVICE
        
        if [ ! -b "$USB_DEVICE" ]; then
            echo -e "${RED}Invalid device: $USB_DEVICE${RESET}"
            exit 1
        fi
        
        echo -e "${RED}Really write to $USB_DEVICE? Type YES to confirm: ${RESET}"
        read confirmation
        
        if [ "$confirmation" = "YES" ]; then
            echo -e "${CYAN}Writing ISO to $USB_DEVICE...${RESET}"
            sudo dd if="$ISO_NAME" of="$USB_DEVICE" bs=4M status=progress oflag=sync
            echo -e "${GREEN}✓ USB created successfully${RESET}"
        else
            echo "Cancelled"
        fi
        ;;
    5)
        echo "Skipping VM test"
        ;;
    *)
        echo "Invalid option"
        ;;
esac

echo ""
echo -e "${CYAN}Next steps:${RESET}"
echo "  1. If tests passed, run: ./pre-ci-check.sh"
echo "  2. Commit your changes"
echo "  3. Push to trigger CI"
echo ""