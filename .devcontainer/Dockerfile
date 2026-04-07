FROM rust:1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2-dev \
    pkg-config \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN cargo install trunk --locked --version 0.21.14

WORKDIR /workspace
