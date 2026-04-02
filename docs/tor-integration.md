# Tor Integration

Current plan:

- Run ai-mesh nodes with optional SOCKS5 proxy settings.
- Route outbound peer dials through Tor when privacy mode is enabled.
- Isolate control and data channels where feasible.

Security notes:

- Do not expose raw model endpoints on public interfaces.
- Keep auth material out of logs.
