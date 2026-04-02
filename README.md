# ai-mesh

**A decentralized P2P Tor-compatible AI Mesh**
Local-first intelligence network. Every device is an AI node. No central servers. Cryptographic trust. Optional anonymity.

## Features

- Local inference only (powered by llama.cpp)
- OpenAI-compatible local API (`/v1/chat/completions`)
- libp2p-powered peer-to-peer mesh with signed job routing
- BLAKE2b receipts + ed25519 signatures for verifiable results
- Optional Tor transport (onion services)
- Local-first memory and knowledge (embeddings + content-addressed storage)

## Quick Start

1. Start llama.cpp server (required for inference):

```bash
./llama-server -m models/llama-3.1-8b-Q4_K_M.gguf --port 8181 --host 127.0.0.1
```

2. Build and run ai-mesh:

```bash
cargo run -p cli
```

3. Test the local API:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
        "messages": [
            {"role": "user", "content": "Say hello from the ai-mesh!"}
        ]
    }'
```

## Architecture

- **Inference Layer**: llama.cpp (local only)
- **API Layer**: Axum OpenAI-compatible facade
- **Core Layer**: Cryptographic identity, signed jobs, receipts
- **Network Layer**: libp2p request/response + identify + ping
- **Trust Layer**: ed25519 signatures + BLAKE2b hash-based receipts

Everything works offline-first. Tor is optional.

## MVP Roadmap

- [x] Local node with OpenAI-compatible API
- [x] Real ed25519 identity + signed JobEnvelope / JobResultEnvelope
- [x] Basic P2P request/response skeleton
- [x] Persistent identity storage
- [x] Configuration system (TOML)
- [ ] Outbound job sending + two-node testing
- [ ] Capability advertisement
- [ ] Tor onion transport option
- [ ] Trust and verification layer (receipt chaining + spot-checking)

## Development

```bash
# Format + lint
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test
cargo test --workspace

# Run the node
cargo run -p cli
```

See `Justfile` for common tasks.

## Project Structure

```text
ai-mesh/
|- crates/
|  |- node-core/      # Identity, signing, JobEnvelope, receipts
|  |- model-runtime/  # llama.cpp HTTP wrapper
|  |- net-libp2p/     # libp2p mesh
|  |- api/            # Axum OpenAI-compatible endpoints
|  |- cli/            # Main daemon binary
|  `- config/         # TOML configuration crate (mesh-config)
|- config/
|- docs/
`- scripts/
```

## Next Steps

- Outbound `send-job` support for two-node handoff testing
- Capability advertisement + peer dialing strategy
- Tor integration
- Local vector memory

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

**License**: MIT OR Apache-2.0
