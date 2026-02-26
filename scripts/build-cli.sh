#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

echo "[CLI] Building release binary..."
cargo build --release --bin zekken

echo "[CLI] Done."
echo "Binary: ${ROOT_DIR}/target/release/zekken"
