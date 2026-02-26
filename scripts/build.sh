#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-}"

run_mode() {
  local selected="$1"
  case "${selected}" in
    cli)
      "${SCRIPT_DIR}/build-cli.sh"
      ;;
    wasm)
      "${SCRIPT_DIR}/build-wasm.sh"
      ;;
    both|all)
      "${SCRIPT_DIR}/build-cli.sh"
      "${SCRIPT_DIR}/build-wasm.sh"
      ;;
    *)
      echo "Usage: $0 [cli|wasm|both]"
      exit 1
      ;;
  esac
}

if [[ -z "${MODE}" ]]; then
  echo "Select build target:"
  echo "  1) cli"
  echo "  2) wasm (only used for the website demo)"
  echo "  3) both"
  read -r -p "Enter choice [1-3]: " choice
  case "${choice}" in
    1) MODE="cli" ;;
    2) MODE="wasm" ;;
    3) MODE="both" ;;
    *) echo "Invalid choice."; exit 1 ;;
  esac
fi

run_mode "${MODE}"
