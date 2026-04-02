# ai-mesh

**A decentralized P2P Tor-compatible AI Mesh**  
Local-first intelligence network. Every device is an AI node. No central servers. Cryptographic trust. Optional anonymity.

## Features

- Local inference only (powered by llama.cpp)
- OpenAI-compatible local API (`/v1/chat/completions`)
- libp2p-powered peer-to-peer mesh with signed jobs and receipts
- ed25519 signatures + BLAKE2b receipts for verifiable results
- Capability advertisement and basic peer discovery
- Persistent identity and TOML configuration
- Optional Tor transport (planned)

## Quick Start

1. Start llama.cpp server (on port 8181 by default):

```bash
./llama-server -m models/llama-3.1-8b-Q4_K_M.gguf --port 8181 --host 127.0.0.1
```

2. Run the node:

```bash
cargo run -p cli run
```

3. Test the local API:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -d '{
        "messages": [{"role": "user", "content": "Hello from ai-mesh!"}]
    }'
```

## Two-Node Test

1. **Node A** (default ports):
     ```bash
     cargo run -p cli run
     ```
     Copy its Peer ID from the startup log.

2. **Node B** - edit `~/.config/ai-mesh/config.toml`:
     ```toml
     p2p_listen = "/ip4/0.0.0.0/tcp/9001"
     bootstrap_peers = ["/ip4/127.0.0.1/tcp/9000/p2p/<NodeA-Peer-ID>"]
     ```

3. Start Node B:
     ```bash
     cargo run -p cli run
     ```

4. From Node B, send a test job to Node A:
     ```bash
     cargo run -p cli send-job \
         --to <NodeA-Peer-ID> \
         --prompt "Hello from Node B via the mesh!"
     ```

You should see the job received and a signed result returned on Node A.

## Capability Advertisement

Nodes now automatically publish their capabilities on startup via gossipsub (`ai-mesh/capabilities` topic).

Current capability includes:
- Available models
- Max context length
- Quantization level
- Rough speed estimate

Future steps will use these announcements for intelligent job routing.

## Project Structure

```text
ai-mesh/
├── crates/
│   ├── node-core/      # Identity, signing, JobEnvelope, NodeCapability
│   ├── model-runtime/  # llama.cpp wrapper
│   ├── net-libp2p/     # libp2p mesh + gossipsub
│   ├── api/            # Axum OpenAI façade
│   ├── config/         # TOML config + persistence
│   └── cli/            # Daemon + send-job subcommand
├── config/
└── docs/
```

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
cargo run -p cli run
```

See `Justfile` for common tasks.

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

---

**License**: MIT OR Apache-2.0
