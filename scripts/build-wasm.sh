#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEMO_WASM_DIR="${ROOT_DIR}/website/Demo/js/WASM"

cd "${ROOT_DIR}"

echo "[WASM] Building website demo assets..."
mkdir -p "${DEMO_WASM_DIR}"

ensure_demo_assets_exist_or_copy_from_pkg() {
  # wasm-pack usually writes into the provided --out-dir, but we keep a fallback copy
  # from ./pkg to avoid "built but not in demo dir" surprises.
  local js="${DEMO_WASM_DIR}/zekken_wasm.js"
  local wasm="${DEMO_WASM_DIR}/zekken_wasm_bg.wasm"
  local dts="${DEMO_WASM_DIR}/zekken_wasm.d.ts"
  local wasm_dts="${DEMO_WASM_DIR}/zekken_wasm_bg.wasm.d.ts"

  if [[ -f "${js}" && -f "${wasm}" ]]; then
    return 0
  fi

  if [[ -d "${ROOT_DIR}/pkg" ]]; then
    echo "[WASM] Demo assets missing in ${DEMO_WASM_DIR}; copying from ${ROOT_DIR}/pkg..."
    cp -f "${ROOT_DIR}/pkg/zekken_wasm.js" "${js}" 2>/dev/null || true
    cp -f "${ROOT_DIR}/pkg/zekken_wasm_bg.wasm" "${wasm}" 2>/dev/null || true
    cp -f "${ROOT_DIR}/pkg/zekken_wasm.d.ts" "${dts}" 2>/dev/null || true
    cp -f "${ROOT_DIR}/pkg/zekken_wasm_bg.wasm.d.ts" "${wasm_dts}" 2>/dev/null || true
  fi

  if [[ ! -f "${js}" || ! -f "${wasm}" ]]; then
    echo "ERROR: WASM assets were not produced in ${DEMO_WASM_DIR}."
    echo "Expected:"
    echo "  ${js}"
    echo "  ${wasm}"
    echo "If you used wasm-pack, check whether it wrote to ./pkg instead."
    exit 1
  fi
}

if command -v wasm-pack >/dev/null 2>&1; then
  echo "[WASM] Using wasm-pack..."
  wasm-pack build \
    --release \
    --target web \
    --out-dir "${DEMO_WASM_DIR}" \
    --out-name zekken_wasm
  ensure_demo_assets_exist_or_copy_from_pkg
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

  # Fallback mode should always write directly to DEMO_WASM_DIR, but verify anyway.
  ensure_demo_assets_exist_or_copy_from_pkg
fi

printf "[WASM] Done.\nAssets:\n"
printf "        %s\n" "${DEMO_WASM_DIR}/zekken_wasm.js"
printf "        %s\n" "${DEMO_WASM_DIR}/zekken_wasm_bg.wasm"
