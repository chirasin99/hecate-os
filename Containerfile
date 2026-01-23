# HecateOS Bootable Container Image
FROM ubuntu:24.04

# Metadata
LABEL org.opencontainers.image.title="HecateOS"
LABEL org.opencontainers.image.description="Performance-optimized Linux with automatic hardware detection"
LABEL org.opencontainers.image.vendor="HecateOS Team"
LABEL org.opencontainers.image.version="0.1.0"

# Environment
ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV TZ=UTC

# Base system packages
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        # Core system
        systemd \
        systemd-sysv \
        ubuntu-minimal \
        # Kernel and boot
        linux-image-generic-hwe-24.04 \
        linux-headers-generic-hwe-24.04 \
        initramfs-tools \
        grub-pc-bin \
        grub-efi-amd64-bin \
        # Network
        network-manager \
        iproute2 \
        iputils-ping \
        # Essential tools
        sudo \
        vim-tiny \
        nano \
        curl \
        wget \
        git \
        htop \
        # Hardware support
        pciutils \
        usbutils \
        dmidecode \
        lshw \
        # Build tools for HecateOS components
        build-essential \
        pkg-config \
        libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal && \
    /root/.cargo/bin/rustup component add rustfmt clippy

ENV PATH="/root/.cargo/bin:${PATH}"

# Copy and build Rust components if available
COPY rust/ /tmp/hecate-rust/
WORKDIR /tmp/hecate-rust
RUN if [ -f Cargo.toml ]; then \
        cargo build --release && \
        find target/release -maxdepth 1 -type f -executable -name "hecate-*" \
            -exec cp {} /usr/local/bin/ \; && \
        rm -rf /tmp/hecate-rust; \
    fi

# Copy system configuration
COPY config/includes.chroot/ / 2>/dev/null || true

# System configuration
RUN echo "hecate-os" > /etc/hostname && \
    echo "127.0.1.1 hecate-os" >> /etc/hosts && \
    useradd -m -s /bin/bash -G sudo,adm,cdrom,dip,plugdev hecate && \
    echo "hecate:hecate" | chpasswd && \
    echo "hecate ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/hecate && \
    chmod 0440 /etc/sudoers.d/hecate

# Enable essential services
RUN systemctl enable NetworkManager.service || true && \
    systemctl enable systemd-timesyncd.service || true && \
    systemctl set-default multi-user.target

# Cleanup
RUN apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    rm -rf /usr/share/doc/* /usr/share/man/* && \
    find /var/log -type f -exec truncate -s 0 {} \;

# Set working directory
WORKDIR /

# Default init
CMD ["/sbin/init"]