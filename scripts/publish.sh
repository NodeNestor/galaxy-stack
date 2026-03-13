#!/usr/bin/env bash
set -euo pipefail

# Rebuild and republish the WASM module (after changing server/src/lib.rs)
# Usage: bash scripts/publish.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"
DB_NAME="${PUBLIC_STDB_DB:-app}"

cd "$ROOT"

echo "==> Building WASM module..."
cd server && cargo build --target wasm32-unknown-unknown --release && cd ..

echo "==> Publishing module as '$DB_NAME'..."
WASM_PATH="server/target/wasm32-unknown-unknown/release/app_server.wasm"
docker cp "$WASM_PATH" "$(docker compose ps -q spacetimedb):/tmp/module.wasm"
MSYS_NO_PATHCONV=1 docker exec "$(docker compose ps -q spacetimedb)" \
  spacetime publish --server http://localhost:3000 --bin-path /tmp/module.wasm "$DB_NAME" -y

echo "==> Module updated."
