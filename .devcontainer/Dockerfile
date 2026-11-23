FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install system dependencies for Soroban WASM builds
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        curl \
        git \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Add WASM target for Soroban contract builds
RUN rustup target add wasm32-unknown-unknown

# Install just command runner
RUN cargo install just

# Install Node.js for tooling scripts (TypeScript)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npx

# Set working directory
WORKDIR /workspaces/ApexChainx-Contracts

# Pre-fetch Cargo dependencies for faster builds
COPY Cargo.toml ./
RUN touch Cargo.lock
COPY apexchainx_calculator/Cargo.toml apexchainx_calculator/
COPY apexchainx_calculator/fuzz/Cargo.toml apexchainx_calculator/fuzz/
RUN mkdir -p apexchainx_calculator/src && echo 'fn main() {}' > apexchainx_calculator/src/lib.rs
RUN cargo fetch || true
RUN rm -rf apexchainx_calculator/src

CMD ["/bin/bash"]
