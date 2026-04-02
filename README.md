# ai-mesh

**A decentralized P2P Tor-compatible AI Mesh**
Local-first intelligence network. Every device is an AI node. No central servers. Cryptographic trust. Optional anonymity.

## Features

- Local inference only (llama.cpp backend)
- OpenAI-compatible local API (`/v1/chat/completions`)
- libp2p-powered P2P mesh with signed job routing
- BLAKE2b-style receipt model and signature-ready envelopes
- Optional Tor transport (onion services)
- Local-first memory and content-addressed knowledge layers

## Quick Start

1. Run a llama.cpp server (default port 8080):

```bash
./llama-server -m models/llama-3.1-8b-Q4.gguf --port 8080
```

2. Build and run ai-mesh:

```bash
cargo run --bin ai-mesh -- run --p2p-listen /ip4/127.0.0.1/tcp/9000 --api-listen 127.0.0.1:8080 --llama-base-url http://127.0.0.1:8080
```

3. Test locally:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
	-H "Content-Type: application/json" \
	-d '{"model": "local-model", "messages": [{"role": "user", "content": "Hello from the mesh!"}]}'
```

## Architecture

```mermaid
graph TD
    A[User App] -->|OpenAI API| B[Axum API Layer]
    B --> C[Model Runtime (llama.cpp)]
    B --> D[Coordination and Routing]
    D <--> E[libp2p Mesh (P2P)]
    E --> F[Tor Transport (optional)]
    C --> G[Local Memory and Vector Store]
    E --> H[Signed Jobs and Receipts]
```

Full blueprint: `docs/architecture.md`

## MVP Roadmap

- [x] Local node with OpenAI-compatible API surface
- [x] Cryptographic identity and signed envelope types
- [ ] Basic P2P job routing
- [ ] Capability advertisement
- [ ] Persistent identity and config
- [ ] Tor onion support
- [ ] Trust and verification layer

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

See `Justfile` for common tasks.

## Contributing

Contributions are welcome. See `CONTRIBUTING.md`.

## License

This project is dual-licensed under MIT OR Apache-2.0. See `LICENSE`.
