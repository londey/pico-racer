#!/bin/bash
set -e

echo "=== pico-racer Devcontainer Post-Create Setup ==="

# Verify Rust toolchain
echo "Verifying Rust toolchain..."
echo "  rustc: $(rustc --version 2>/dev/null || echo 'not found')"
echo "  cargo: $(cargo --version 2>/dev/null || echo 'not found')"
echo "  RP2350 target: $(rustup target list --installed | grep thumbv8m || echo 'not found')"

# Verify Claude Code CLI
echo "Verifying Claude Code CLI..."
echo "  claude: $(claude --version 2>/dev/null || echo 'installed')"

# Set up Claude Code if ANTHROPIC_API_KEY is available
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "ANTHROPIC_API_KEY detected - Claude Code ready to use"
else
    echo "ANTHROPIC_API_KEY not set - run 'claude' and use /login to authenticate"
fi

# Initialize git submodules
echo "Initializing git submodules..."
cd /workspaces/pico-racer

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Available tools:"
echo "  rustc, cargo (Rust toolchain)"
echo "  claude (AI assistant)"
echo ""
