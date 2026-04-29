#FROM rust:1.94-slim AS builder
FROM rust:1.84-slim

RUN apt-get update && apt-get install -y \
    pkg-config \
    curl \
    perl \
    libasound2-dev \
    libssl-dev \
    cmake \
    clang \
    usbutils \
    git \
    nano \
    && rm -rf /var/lib/apt/lists/*

ENV CMAKE_POLICY_VERSION_MINIMUM=3.5

WORKDIR /app
COPY . .

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs/ | sh -s -- -y
RUN cargo +stable install espup --locked
RUN espup install
RUN echo "source /root/export-esp.sh" >> /root/.bashrc

RUN cargo +stable install esp-generate
RUN cargo +stable install esp-config

RUN cargo install cargo-binstall --locked
RUN cargo binstall espflash

ENV LIBCLANG_PATH="/usr/local/rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
ENV PATH="/usr/local/rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin:$PATH"

CMD ["/bin/bash"]

