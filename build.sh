#!/bin/bash
# Unified build script for pico-racer host application
# Builds RP2350 firmware, PC debug host, runs tests

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Directories
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${REPO_ROOT}/build"

# Default build targets
BUILD_FIRMWARE=true
BUILD_PC=true
BUILD_TEST=true
BUILD_FMT=true
BUILD_CLIPPY=true
RELEASE_MODE=false
FLASH_FIRMWARE=false

# Parse command-line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --firmware-only)
            BUILD_PC=false
            shift
            ;;
        --pc-only)
            BUILD_FIRMWARE=false
            shift
            ;;
        --test-only)
            BUILD_FIRMWARE=false
            BUILD_PC=false
            BUILD_FMT=false
            BUILD_CLIPPY=false
            shift
            ;;
        --no-test)
            BUILD_TEST=false
            shift
            ;;
        --no-lint)
            BUILD_FMT=false
            BUILD_CLIPPY=false
            shift
            ;;
        --release)
            RELEASE_MODE=true
            shift
            ;;
        --flash-firmware)
            FLASH_FIRMWARE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --firmware-only     Build only RP2350 firmware (skips PC host)"
            echo "  --pc-only           Build only PC debug host (skips firmware)"
            echo "  --test-only         Run tests only (skip all builds and lints)"
            echo "  --no-test           Skip tests (build only)"
            echo "  --no-lint           Skip formatting check and clippy"
            echo "  --release           Build in release mode (optimized)"
            echo "  --flash-firmware    Flash firmware to RP2350 after build"
            echo "  --help              Show this help message"
            echo ""
            echo "Default: Lint, test, and build everything"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}=== pico-racer Build System ===${NC}"
echo ""

# Step 1: Format check
if [ "$BUILD_FMT" = true ]; then
    echo -e "${YELLOW}[1/5] Checking formatting...${NC}"
    cd "${REPO_ROOT}"
    cargo fmt --check
    echo -e "${GREEN}✓ Formatting OK${NC}"
    echo ""
fi

# Step 2: Clippy
# Run separately for host crates and the RP2350 firmware crate, since the
# firmware crate is no_std and cannot compile for the host target (rp235x-hal
# gates `multicore`/`entry` behind `target_arch = "arm"`, and the ARMv8-M MPU
# fields used in main.rs only exist when targeting cortex-m33).
if [ "$BUILD_CLIPPY" = true ]; then
    echo -e "${YELLOW}[2/5] Running clippy...${NC}"
    cd "${REPO_ROOT}"
    cargo clippy \
        -p pico-racer-pc -p pico-racer-core -p pico-racer-hal -p asset-prep \
        -- -D warnings
    cargo clippy \
        -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf \
        -- -D warnings
    echo -e "${GREEN}✓ Clippy passed${NC}"
    echo ""
fi

# Step 3: Rust tests
if [ "$BUILD_TEST" = true ]; then
    echo -e "${YELLOW}[3/5] Running Rust tests...${NC}"
    cd "${REPO_ROOT}"
    cargo test -p pico-racer-core
    echo -e "${GREEN}✓ Rust tests passed${NC}"
    echo ""
fi

# Step 4: Build RP2350 firmware (asset conversion happens automatically via build.rs)
if [ "$BUILD_FIRMWARE" = true ]; then
    echo -e "${YELLOW}[4/5] Building RP2350 firmware (includes asset conversion)...${NC}"
    cd "${REPO_ROOT}"
    if [ "$RELEASE_MODE" = true ]; then
        cargo build --release -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf
        FIRMWARE_ELF="${REPO_ROOT}/build/cargo/thumbv8m.main-none-eabihf/release/pico-racer-rp2350"
    else
        cargo build -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf
        FIRMWARE_ELF="${REPO_ROOT}/build/cargo/thumbv8m.main-none-eabihf/debug/pico-racer-rp2350"
    fi
    echo -e "${GREEN}✓ Firmware built: ${FIRMWARE_ELF}${NC}"
    echo ""
fi

# Step 5: Build PC debug host
if [ "$BUILD_PC" = true ]; then
    echo -e "${YELLOW}[5/5] Building PC debug host...${NC}"
    cd "${REPO_ROOT}"
    if [ "$RELEASE_MODE" = true ]; then
        cargo build --release -p pico-racer-pc
        PC_BINARY="${REPO_ROOT}/build/cargo/release/pico-racer-pc"
    else
        cargo build -p pico-racer-pc
        PC_BINARY="${REPO_ROOT}/build/cargo/debug/pico-racer-pc"
    fi
    echo -e "${GREEN}✓ PC debug host built${NC}"
    echo ""
fi

# Collect build outputs
echo -e "${YELLOW}Collecting build outputs...${NC}"
mkdir -p "${OUTPUT_DIR}/firmware" "${OUTPUT_DIR}/pc"

if [ "$BUILD_FIRMWARE" = true ] && [ -n "${FIRMWARE_ELF:-}" ] && [ -f "$FIRMWARE_ELF" ]; then
    cp "$FIRMWARE_ELF" "${OUTPUT_DIR}/firmware/pico-racer-rp2350.elf"
    echo "  Firmware: ${OUTPUT_DIR}/firmware/pico-racer-rp2350.elf"
fi

if [ "$BUILD_PC" = true ] && [ -n "${PC_BINARY:-}" ] && [ -f "$PC_BINARY" ]; then
    cp "$PC_BINARY" "${OUTPUT_DIR}/pc/pico-racer-pc"
    echo "  PC Debug Host: ${OUTPUT_DIR}/pc/pico-racer-pc"
fi

echo -e "${GREEN}✓ Build outputs collected in ${OUTPUT_DIR}/${NC}"
echo ""

# Optional: Flash firmware
if [ "$FLASH_FIRMWARE" = true ]; then
    echo -e "${YELLOW}Flashing firmware to RP2350...${NC}"
    cd "${REPO_ROOT}"
    cargo run --release -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf
    echo -e "${GREEN}✓ Firmware flashed${NC}"
fi

echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
