#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/setup-llama-server.sh <model_path> [port]
# Example:
#   scripts/setup-llama-server.sh models/llama-3.1-8b-Q4.gguf 8080

MODEL_PATH="${1:-}"
PORT="${2:-8080}"

if [[ -z "$MODEL_PATH" ]]; then
  echo "usage: $0 <model_path.gguf> [port]" >&2
  exit 1
fi

if [[ ! -f "$MODEL_PATH" ]]; then
  echo "model file not found: $MODEL_PATH" >&2
  exit 1
fi

if [[ -x ./llama-server ]]; then
  BINARY=./llama-server
elif command -v llama-server >/dev/null 2>&1; then
  BINARY="$(command -v llama-server)"
else
  echo "llama-server not found. Build from llama.cpp or place binary at ./llama-server" >&2
  exit 1
fi

echo "starting llama-server on :$PORT using model $MODEL_PATH"
exec "$BINARY" -m "$MODEL_PATH" --port "$PORT"
