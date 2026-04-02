set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    just --list

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

build:
    cargo build --workspace

run:
    cargo run -p cli -- run --p2p-listen /ip4/127.0.0.1/tcp/9000 --api-listen 127.0.0.1:8080 --llama-base-url http://127.0.0.1:8080
