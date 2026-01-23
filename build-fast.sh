#!/bin/bash
#
# HecateOS Fast Build - Main entry point
#

set -e

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
BUILD_FAST_DIR="$SCRIPT_DIR/build-fast"

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
echo -e "${CYAN}Fast ISO Building System${RESET}"
echo ""

# Check if build-fast directory exists
if [ ! -d "$BUILD_FAST_DIR" ]; then
    echo -e "${RED}Error: build-fast directory not found${RESET}"
    exit 1
fi

# Parse command
case "${1:-help}" in
    install|deps)
        echo -e "${CYAN}Installing dependencies...${RESET}"
        exec "$BUILD_FAST_DIR/install-deps.sh"
        ;;
    
    build|bootc)
        echo -e "${CYAN}Building with bootc (fastest)...${RESET}"
        exec "$BUILD_FAST_DIR/build-bootc.sh"
        ;;
    
    mmdebstrap|fast)
        echo -e "${CYAN}Building with mmdebstrap...${RESET}"
        exec "$BUILD_FAST_DIR/build-mmdebstrap.sh"
        ;;
    
    test)
        echo -e "${CYAN}Testing ISO...${RESET}"
        shift
        exec "$BUILD_FAST_DIR/test-local.sh" "$@"
        ;;
    
    check|pre-ci)
        echo -e "${CYAN}Running pre-CI checks...${RESET}"
        exec "$BUILD_FAST_DIR/pre-ci-check.sh"
        ;;
    
    clean)
        echo -e "${CYAN}Cleaning build artifacts...${RESET}"
        rm -rf "$BUILD_FAST_DIR/work"* "$BUILD_FAST_DIR/cache"
        rm -rf "$SCRIPT_DIR/iso"/*.iso* 
        echo -e "${GREEN}✓ Cleaned${RESET}"
        ;;
    
    benchmark|bench)
        echo -e "${CYAN}Benchmarking build methods...${RESET}"
        echo ""
        
        # Time each method
        echo -e "${YELLOW}Testing mmdebstrap method...${RESET}"
        time -p "$BUILD_FAST_DIR/build-mmdebstrap.sh" 2>&1 | tail -3
        
        echo ""
        echo -e "${YELLOW}Testing bootc method...${RESET}"
        time -p "$BUILD_FAST_DIR/build-bootc.sh" 2>&1 | tail -3
        
        echo ""
        echo -e "${GREEN}Benchmark complete!${RESET}"
        ;;
    
    help|--help|-h|*)
        cat << EOF
${CYAN}Usage:${RESET} $0 [command] [options]

${CYAN}Commands:${RESET}
  install       Install build dependencies
  build         Build ISO using fastest method (bootc)
  mmdebstrap    Build ISO using mmdebstrap (2x faster than debootstrap)
  test [iso]    Test ISO in QEMU
  check         Run pre-CI checks locally
  clean         Remove all build artifacts
  benchmark     Compare build times of different methods
  help          Show this help

${CYAN}Quick Start:${RESET}
  1. $0 install     # Install dependencies
  2. $0 build       # Build ISO (5-10 minutes)
  3. $0 test        # Test in VM
  4. $0 check       # Validate before pushing

${CYAN}Build Methods Comparison:${RESET}
  ${GREEN}bootc${RESET}       - 5-10 min  - Container-based, excellent caching
  ${GREEN}mmdebstrap${RESET}  - 10-15 min - 2x faster than debootstrap
  ${YELLOW}docker${RESET}      - 8-12 min  - Docker export method
  ${RED}live-build${RESET}  - 30-60 min - Legacy method (slow)

${CYAN}Performance Tips:${RESET}
  • Run on SSD for best performance
  • Use local apt-cacher-ng for package caching
  • Container builds cache layers between runs
  • Test locally before pushing to CI

${CYAN}More Information:${RESET}
  See build-fast/README.md for detailed documentation
EOF
        ;;
esac