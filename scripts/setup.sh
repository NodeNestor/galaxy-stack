#!/usr/bin/env bash
set -euo pipefail

# One-command setup: docker compose + build WASM + publish module
# Usage: bash scripts/setup.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"
DB_NAME="${PUBLIC_STDB_DB:-app}"

cd "$ROOT"

echo "==> Starting containers..."
docker compose up -d

echo "==> Waiting for SpacetimeDB to be healthy..."
for i in $(seq 1 60); do
  curl -sf http://localhost:3000/v1/ping > /dev/null 2>&1 && break
  if [ "$i" -eq 60 ]; then
    echo "    ERROR: SpacetimeDB did not start within 60 seconds."
    exit 1
  fi
  sleep 1
done
echo "    SpacetimeDB is ready."

echo "==> Building WASM module..."
cd server && cargo build --target wasm32-unknown-unknown --release && cd ..

echo "==> Publishing module as '$DB_NAME'..."
WASM_PATH="server/target/wasm32-unknown-unknown/release/app_server.wasm"
docker cp "$WASM_PATH" "$(docker compose ps -q spacetimedb):/tmp/module.wasm"
MSYS_NO_PATHCONV=1 docker exec "$(docker compose ps -q spacetimedb)" \
  spacetime publish --server http://localhost:3000 --bin-path /tmp/module.wasm "$DB_NAME" -y

echo ""
echo "==> Done! Open http://localhost:4321"
