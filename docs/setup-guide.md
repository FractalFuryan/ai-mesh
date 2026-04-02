# Setup Guide

## Prerequisites

- Rust stable toolchain
- Optional: llama.cpp and llama-server for local inference

## Build

```bash
cargo build --workspace
```

## Run a node

```bash
cargo run -p cli -- run --p2p-listen /ip4/127.0.0.1/tcp/9000 --api-listen 127.0.0.1:8080 --llama-base-url http://127.0.0.1:8080
```
