# Architecture

ai-mesh uses a modular Rust workspace:

- node-core: identity, signing, receipts, and shared domain errors.
- model-runtime: model process orchestration and health checks.
- net-libp2p: peer discovery and mesh networking primitives.
- api: HTTP facade and OpenAI-style endpoint compatibility.
- cli: executable entry point for local node operation.

Design principles:

- Local-first model execution.
- Optional privacy routing through Tor.
- Composable crates with clear boundaries.
