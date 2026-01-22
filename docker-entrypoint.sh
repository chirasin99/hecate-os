#!/bin/bash
#
# HecateOS Docker Build Entrypoint
# Handles ISO building inside Docker container
#

set -e

# Colors
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'

echo -e "${PURPLE}"
cat << 'EOF'
╦ ╦┌─┐┌─┐┌─┐┌┬┐┌─┐╔═╗╔═╗
╠═╣├┤ │  ├─┤ │ ├┤ ║ ║╚═╗
╩ ╩└─┘└─┘┴ ┴ ┴ └─┘╚═╝╚═╝

Docker Build Environment
EOF
echo -e "${RESET}"

# Verify we're running with necessary privileges
if [ ! -w /dev ]; then
    echo -e "${RED}Error: Container must run with --privileged flag${RESET}"
    echo "Usage: docker run --rm --privileged -v \$(pwd):/build hecate-builder"
    exit 1
fi

# Verify build directory has required files
if [ ! -f "/build/build.sh" ]; then
    echo -e "${RED}Error: build.sh not found in /build${RESET}"
    echo "Mount the HecateOS repository: -v \$(pwd):/build"
    exit 1
fi

# Create iso output directory if not exists
mkdir -p /build/iso

# Parse command
case "${1:-build}" in
    build)
        echo -e "${CYAN}Starting full build...${RESET}"
        cd /build
        ./build.sh build
        ;;
    rebuild)
        echo -e "${CYAN}Starting rebuild (no clean)...${RESET}"
        cd /build
        ./build.sh rebuild
        ;;
    clean)
        echo -e "${CYAN}Cleaning build artifacts...${RESET}"
        cd /build
        ./build.sh clean
        ;;
    shell)
        echo -e "${CYAN}Starting interactive shell...${RESET}"
        exec /bin/bash
        ;;
    *)
        echo "Usage: docker run ... hecate-builder [build|rebuild|clean|shell]"
        echo ""
        echo "Commands:"
        echo "  build   - Full clean build (default)"
        echo "  rebuild - Rebuild without cleaning"
        echo "  clean   - Clean build artifacts"
        echo "  shell   - Interactive shell for debugging"
        exit 1
        ;;
esac

# Show output location
if ls /build/iso/*.iso 1>/dev/null 2>&1; then
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${GREEN}Build complete! ISO available in ./iso/${RESET}"
    ls -lh /build/iso/*.iso 2>/dev/null || true
fi
