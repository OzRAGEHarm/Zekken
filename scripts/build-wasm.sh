#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEMO_WASM_DIR="${ROOT_DIR}/website/Demo/js/WASM"

cd "${ROOT_DIR}"

echo "[WASM] Building website demo assets..."
mkdir -p "${DEMO_WASM_DIR}"

if command -v wasm-pack >/dev/null 2>&1; then
  echo "[WASM] Using wasm-pack..."
  wasm-pack build \
    --release \
    --target web \
    --out-dir "${DEMO_WASM_DIR}" \
    --out-name zekken_wasm
else
  echo "[WASM] wasm-pack not found, using cargo + wasm-bindgen fallback..."

  if ! rustup target list --installed | grep -q '^wasm32-unknown-unknown$'; then
    echo "[WASM] Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
  fi

  cargo build --release --target wasm32-unknown-unknown

  if ! command -v wasm-bindgen >/dev/null 2>&1; then
    echo "ERROR: wasm-bindgen CLI is required for fallback mode."
    echo "Install it with: cargo install wasm-bindgen-cli"
    exit 1
  fi

  wasm-bindgen \
    --target web \
    --out-dir "${DEMO_WASM_DIR}" \
    "${ROOT_DIR}/target/wasm32-unknown-unknown/release/zekken_wasm.wasm"
fi

echo "[WASM] Done.\nAssests:"
echo "        ${DEMO_WASM_DIR}/zekken_wasm.js"
echo "        ${DEMO_WASM_DIR}/zekken_wasm_bg.wasm"
