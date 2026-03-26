#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_TRIPLE="${1:-}"

cd "${ROOT_DIR}"

if [[ "${TARGET_TRIPLE}" == "host" ]]; then
  TARGET_TRIPLE=""
fi

if [[ -n "${TARGET_TRIPLE}" ]]; then
  if ! rustup target list --installed | grep -q "^${TARGET_TRIPLE}$"; then
    echo "[CLI] Installing Rust target: ${TARGET_TRIPLE}"
    rustup target add "${TARGET_TRIPLE}"
  fi

  echo "[CLI] Building release binary for ${TARGET_TRIPLE}..."
  cargo build --release --bin zekken --target "${TARGET_TRIPLE}"
  echo "[CLI] Done."
  echo "Binary: ${ROOT_DIR}/target/${TARGET_TRIPLE}/release/zekken"
  if [[ "${TARGET_TRIPLE}" == *windows* ]]; then
    echo "Binary (windows): ${ROOT_DIR}/target/${TARGET_TRIPLE}/release/zekken.exe"
  fi
else
  echo "[CLI] Building release binary for host..."
  cargo build --release --bin zekken
  echo "[CLI] Done."
  echo "Binary: ${ROOT_DIR}/target/release/zekken"
fi
