FROM rust:1.87-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY crates ./crates

RUN cargo build -p cli --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ai-mesh /usr/local/bin/ai-mesh
COPY config/default.toml /app/config/default.toml

EXPOSE 7001 8080
CMD ["ai-mesh", "run", "--p2p-listen", "/ip4/0.0.0.0/tcp/9000", "--api-listen", "0.0.0.0:8080", "--llama-base-url", "http://127.0.0.1:8080"]
