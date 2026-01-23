#!/bin/bash
#
# Install dependencies for fast ISO building
#

set -e

PURPLE='\033[0;35m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

echo -e "${PURPLE}HecateOS Fast Build - Dependency Installer${RESET}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${YELLOW}This script needs sudo. Re-running with sudo...${RESET}"
    exec sudo "$0" "$@"
fi

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VERSION=$VERSION_ID
else
    echo -e "${RED}Cannot detect OS version${RESET}"
    exit 1
fi

echo -e "${CYAN}Detected OS: $OS $VERSION${RESET}"
echo ""

# Core dependencies for all methods
echo -e "${CYAN}Installing core dependencies...${RESET}"
apt-get update
apt-get install -y \
    curl \
    wget \
    git \
    gnupg \
    ca-certificates \
    lsb-release \
    software-properties-common \
    build-essential \
    xorriso \
    isolinux \
    syslinux-utils \
    squashfs-tools \
    genisoimage \
    dosfstools \
    mtools \
    grub-pc-bin \
    grub-efi-amd64-bin \
    grub-efi-amd64-signed \
    shim-signed \
    ovmf \
    qemu-system-x86 \
    qemu-utils \
    apt-cacher-ng

# Install mmdebstrap (faster than debootstrap)
echo -e "${CYAN}Installing mmdebstrap...${RESET}"
apt-get install -y mmdebstrap arch-test qemu-user-static

# Install Docker if not present
if ! command -v docker &> /dev/null; then
    echo -e "${CYAN}Installing Docker...${RESET}"
    curl -fsSL https://get.docker.com | sh
    systemctl enable --now docker
else
    echo -e "${GREEN}✓ Docker already installed${RESET}"
fi

# Install Podman (for bootc)
if ! command -v podman &> /dev/null; then
    echo -e "${CYAN}Installing Podman...${RESET}"
    apt-get install -y podman
else
    echo -e "${GREEN}✓ Podman already installed${RESET}"
fi

# Install bootc-image-builder
echo -e "${CYAN}Installing bootc-image-builder...${RESET}"
if ! command -v bootc-image-builder &> /dev/null; then
    # Install via container alias
    cat > /usr/local/bin/bootc-image-builder << 'EOF'
#!/bin/bash
exec podman run --rm -it --privileged \
    --pull newer \
    -v /var/lib/containers/storage:/var/lib/containers/storage \
    -v .:/workspace \
    -v ./output:/output \
    quay.io/centos-bootc/bootc-image-builder:latest "$@"
EOF
    chmod +x /usr/local/bin/bootc-image-builder
    echo -e "${GREEN}✓ bootc-image-builder installed as container alias${RESET}"
else
    echo -e "${GREEN}✓ bootc-image-builder already available${RESET}"
fi

# Install mkosi if available (for Ubuntu 24.04+)
if [ "$OS" = "ubuntu" ] && [ "${VERSION%%.*}" -ge 24 ]; then
    echo -e "${CYAN}Installing mkosi...${RESET}"
    apt-get install -y mkosi systemd-container systemd-boot
else
    echo -e "${YELLOW}mkosi requires Ubuntu 24.04+, skipping${RESET}"
fi

# Configure apt-cacher-ng for faster package downloads
echo -e "${CYAN}Configuring apt-cacher-ng...${RESET}"
systemctl enable --now apt-cacher-ng
echo 'PassThroughPattern: .*' >> /etc/apt-cacher-ng/acng.conf
systemctl restart apt-cacher-ng

# Create cache directories
echo -e "${CYAN}Creating cache directories...${RESET}"
mkdir -p cache/{packages,images,squashfs,containers}
chmod 777 cache

# Install additional performance tools
echo -e "${CYAN}Installing performance monitoring tools...${RESET}"
apt-get install -y \
    htop \
    iotop \
    sysstat \
    time \
    pv

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo -e "${GREEN}✓ All dependencies installed successfully!${RESET}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""
echo -e "${CYAN}Available build methods:${RESET}"
echo "  ./build-bootc.sh      - Fastest (container-based)"
echo "  ./build-mmdebstrap.sh - Fast (2x faster than debootstrap)"
echo "  ./build-docker.sh     - Docker export method"
echo ""
echo -e "${CYAN}Next steps:${RESET}"
echo "  1. Run ./build-bootc.sh to build ISO"
echo "  2. Run ./test-local.sh to test before pushing"
echo ""