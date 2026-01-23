#!/bin/bash
#
# Pre-CI checks - Run this before pushing to avoid CI failures
#

set -e

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
cd "$SCRIPT_DIR/.."

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
echo -e "${CYAN}Pre-CI Validation Suite${RESET}"
echo ""
echo -e "${YELLOW}This will run the same checks as CI locally${RESET}"
echo ""

FAILED_CHECKS=0
TOTAL_CHECKS=0

# Function to run checks
check() {
    local name="$1"
    local cmd="$2"
    
    ((TOTAL_CHECKS++))
    echo -ne "${CYAN}Checking: ${name}...${RESET} "
    
    if eval "$cmd" > /tmp/pre-ci-check.log 2>&1; then
        echo -e "${GREEN}✓ PASSED${RESET}"
        return 0
    else
        echo -e "${RED}✗ FAILED${RESET}"
        echo -e "${YELLOW}  See /tmp/pre-ci-check.log for details${RESET}"
        ((FAILED_CHECKS++))
        return 1
    fi
}

# Check 1: Shell script linting
echo -e "${CYAN}━━━ Shell Script Quality ━━━${RESET}"
if command -v shellcheck &> /dev/null; then
    for script in $(find . -name "*.sh" -not -path "./chroot/*" -not -path "./.build/*" -not -path "./cache/*"); do
        check "ShellCheck: $(basename $script)" "shellcheck -S error '$script'"
    done
else
    echo -e "${YELLOW}⚠ ShellCheck not installed, skipping shell linting${RESET}"
    echo "  Install with: sudo apt-get install shellcheck"
fi

# Check 2: Dockerfile linting
echo ""
echo -e "${CYAN}━━━ Docker Configuration ━━━${RESET}"
if command -v hadolint &> /dev/null; then
    check "Dockerfile.build" "hadolint Dockerfile.build"
else
    echo -e "${YELLOW}⚠ Hadolint not installed, skipping Dockerfile linting${RESET}"
    echo "  Install with: wget -O /tmp/hadolint https://github.com/hadolint/hadolint/releases/latest/download/hadolint-Linux-x86_64 && sudo mv /tmp/hadolint /usr/local/bin/ && sudo chmod +x /usr/local/bin/hadolint"
fi

# Check 3: YAML validation
echo ""
echo -e "${CYAN}━━━ GitHub Actions Workflows ━━━${RESET}"
for workflow in .github/workflows/*.yml; do
    if [ -f "$workflow" ]; then
        check "YAML syntax: $(basename $workflow)" "python3 -c 'import yaml; yaml.safe_load(open(\"$workflow\"))'"
    fi
done

# Check 4: Rust checks (if applicable)
echo ""
echo -e "${CYAN}━━━ Rust Components ━━━${RESET}"
if [ -d "rust" ] && [ -f "rust/Cargo.toml" ]; then
    cd rust
    check "Cargo fmt" "cargo fmt -- --check"
    check "Cargo clippy" "cargo clippy -- -D warnings"
    check "Cargo test" "cargo test --quiet"
    check "Cargo build" "cargo build --release --quiet"
    cd ..
else
    echo -e "${YELLOW}No Rust components found${RESET}"
fi

# Check 5: Required files
echo ""
echo -e "${CYAN}━━━ Required Files ━━━${RESET}"
REQUIRED_FILES=(
    "VERSION"
    "LICENSE"
    "README.md"
    "build.sh"
    ".github/workflows/build-iso.yml"
)

for file in "${REQUIRED_FILES[@]}"; do
    check "File exists: $file" "[ -f '$file' ]"
done

# Check 6: Version consistency
echo ""
echo -e "${CYAN}━━━ Version Consistency ━━━${RESET}"
if [ -f "VERSION" ]; then
    VERSION=$(cat VERSION | tr -d '[:space:]')
    
    # Check version format
    check "Version format (semver)" "echo '$VERSION' | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$'"
    
    # Check if version is tagged
    if git tag | grep -q "^v$VERSION$"; then
        echo -e "${CYAN}Checking: Git tag v$VERSION...${RESET} ${GREEN}✓ EXISTS${RESET}"
    else
        echo -e "${CYAN}Checking: Git tag v$VERSION...${RESET} ${YELLOW}⚠ NOT TAGGED${RESET}"
        echo -e "  ${YELLOW}Consider tagging: git tag v$VERSION${RESET}"
    fi
fi

# Check 7: ISO build dependencies
echo ""
echo -e "${CYAN}━━━ Build Dependencies ━━━${RESET}"
DEPS=(
    "docker"
    "mmdebstrap"
    "xorriso"
    "squashfs-tools"
)

for dep in "${DEPS[@]}"; do
    check "Command available: $dep" "command -v $dep"
done

# Check 8: Git status
echo ""
echo -e "${CYAN}━━━ Git Repository ━━━${RESET}"
check "Git repository clean" "[ -z \"\$(git status --porcelain)\" ]" || \
    echo -e "  ${YELLOW}Uncommitted changes present${RESET}"

check "On main/master branch" "git branch --show-current | grep -qE '^(main|master)$'" || \
    echo -e "  ${YELLOW}Not on main branch${RESET}"

# Check 9: Docker build test
echo ""
echo -e "${CYAN}━━━ Docker Build Test ━━━${RESET}"
check "Docker build (dry-run)" "docker build -f Dockerfile.build --no-cache --target=base . 2>/dev/null || docker build -f Dockerfile.build ."

# Check 10: ISO size estimate
echo ""
echo -e "${CYAN}━━━ ISO Size Estimate ━━━${RESET}"
if [ -d "config/includes.chroot" ]; then
    SIZE_MB=$(du -sm config/includes.chroot 2>/dev/null | cut -f1)
    echo -e "${CYAN}Checking: Custom files size...${RESET} ${GREEN}${SIZE_MB}MB${RESET}"
    
    if [ "$SIZE_MB" -gt 500 ]; then
        echo -e "  ${YELLOW}⚠ Large custom files may slow ISO build${RESET}"
    fi
fi

# Results summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

if [ "$FAILED_CHECKS" -eq 0 ]; then
    echo -e "${GREEN}✓ All $TOTAL_CHECKS checks passed!${RESET}"
    echo -e "${GREEN}Ready to push to CI${RESET}"
    echo ""
    echo -e "${CYAN}Recommended next steps:${RESET}"
    echo "  1. git add -A"
    echo "  2. git commit -m 'Your commit message'"
    echo "  3. git push origin main"
    exit 0
else
    echo -e "${RED}✗ $FAILED_CHECKS/$TOTAL_CHECKS checks failed${RESET}"
    echo -e "${YELLOW}Fix these issues before pushing to CI${RESET}"
    echo ""
    echo -e "${CYAN}To see detailed errors:${RESET}"
    echo "  cat /tmp/pre-ci-check.log"
    exit 1
fi