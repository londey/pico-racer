# pico-racer Development Container
FROM ubuntu:24.04

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install base dependencies
RUN apt-get update && apt-get install -y \
    # Build essentials
    build-essential \
    git \
    curl \
    wget \
    ca-certificates \
    gnupg \
    # For USB/FPGA programming
    libftdi1-2 \
    libftdi1-dev \
    libudev-dev \
    usbutils \
    # C++ build tools (cmake needed for native Rust deps)
    cmake \
    pkg-config \
    # Python (used by PeakRDL tools)
    python3 \
    python3-pip \
    python3-venv \
    # PDF rendering (for datasheet reading)
    poppler-utils \
    # Utilities
    make \
    sudo \
    locales \
    && rm -rf /var/lib/apt/lists/*

# Set up locale
RUN locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# Install Node.js 20.x LTS (required for Claude Code CLI)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup (required for sysdoc + RP2350 firmware)
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="${CARGO_HOME}/bin:${PATH}"

# Add RP2350 (Cortex-M33) cross-compilation target for firmware builds
RUN rustup target add thumbv8m.main-none-eabihf

# Install Rust security and dependency auditing tools
RUN cargo install cargo-deny cargo-audit

# Install sysdoc from git
RUN cargo install --git https://github.com/sysdoc-rs/sysdoc.git

# Install PeakRDL for register interface code generation (SystemRDL → SV + Rust)
RUN pip3 install --break-system-packages peakrdl peakrdl-regblock peakrdl-rust

# Install Claude Code CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Create non-root user for development (handle existing UID/GID 1000)
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=${USER_UID}

RUN if getent group ${USER_GID} > /dev/null 2>&1; then \
    existing_group=$(getent group ${USER_GID} | cut -d: -f1); \
    groupmod -n ${USERNAME} ${existing_group}; \
    else \
    groupadd --gid ${USER_GID} ${USERNAME}; \
    fi \
    && if getent passwd ${USER_UID} > /dev/null 2>&1; then \
    existing_user=$(getent passwd ${USER_UID} | cut -d: -f1); \
    usermod -l ${USERNAME} -d /home/${USERNAME} -m ${existing_user}; \
    else \
    useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USERNAME}; \
    fi \
    && echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/${USERNAME} \
    && chmod 0440 /etc/sudoers.d/${USERNAME}

# Add user to dialout and plugdev groups for USB access
RUN usermod -aG dialout,plugdev ${USERNAME}

# Create .claude directory for user
RUN mkdir -p /home/${USERNAME}/.claude \
    && chown -R ${USERNAME}:${USERNAME} /home/${USERNAME}/.claude

# Add Rust/Cargo to path in bashrc
# Use user-specific CARGO_HOME so vscode user can download crates and install tools
RUN echo '# Rust environment' >> /home/${USERNAME}/.bashrc \
    && echo 'export CARGO_HOME="${HOME}/.cargo"' >> /home/${USERNAME}/.bashrc \
    && echo 'export PATH="/opt/cargo/bin:${HOME}/.cargo/bin:${PATH}"' >> /home/${USERNAME}/.bashrc

# Store Claude Code config in the workspace so plans and memory persist across
# devcontainer rebuilds
ENV CLAUDE_CONFIG_DIR=/workspaces/pico-racer/.claude

# Final user switch for workdir
USER ${USERNAME}
WORKDIR /workspaces
