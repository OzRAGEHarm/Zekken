#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-}"
PLATFORM="${2:-}"
HOST_TRIPLE="$(rustc -vV | awk '/host:/ { print $2 }')"

host_platform() {
  case "${HOST_TRIPLE}" in
    *windows*) echo "windows" ;;
    *) echo "linux" ;;
  esac
}

default_triple_for() {
  local platform="$1"
  local host_platform_name
  host_platform_name="$(host_platform)"

  case "${platform}" in
    linux)
      if [[ "${host_platform_name}" == "linux" ]]; then
        # Native build: let Cargo pick the host target so output lands in `target/release/`.
        echo "host"
      else
        echo "x86_64-unknown-linux-gnu"
      fi
      ;;
    windows)
      if [[ "${host_platform_name}" == "windows" ]]; then
        # Native build: let Cargo pick the host target so output lands in `target/release/`.
        echo "host"
      else
        echo "x86_64-pc-windows-gnu"
      fi
      ;;
    *)
      echo ""
      ;;
  esac
}

run_cli_for_platforms() {
  local selected_platform="$1"
  local targets=()
  local t

  case "${selected_platform}" in
    "" )
      # Native build for the current host.
      targets=("host")
      ;;
    linux|windows)
      t="$(default_triple_for "${selected_platform}")"
      [[ -n "${t}" ]] && targets+=("${t}")
      ;;
    all)
      targets+=("$(default_triple_for linux)")
      targets+=("$(default_triple_for windows)")
      ;;
    *)
      echo "Invalid platform: ${selected_platform}"
      echo "Valid platforms: linux|windows|all"
      exit 1
      ;;
  esac

  # De-duplicate targets while preserving order.
  local unique=()
  local seen=""
  for t in "${targets[@]}"; do
    if [[ -z "${t}" ]]; then
      continue
    fi
    if [[ " ${seen} " != *" ${t} "* ]]; then
      unique+=("${t}")
      seen="${seen} ${t}"
    fi
  done

  for t in "${unique[@]}"; do
    "${SCRIPT_DIR}/build-cli.sh" "${t}"
  done
}

run_mode() {
  local selected_mode="$1"
  local selected_platform="$2"

  if [[ "${selected_mode}" == "wasm" && -n "${selected_platform}" ]]; then
    echo "[build.sh] Note: platform argument is ignored for wasm; wasm target is web/wasm32."
  fi

  if [[ "${selected_mode}" == "all" || "${selected_mode}" == "both" ]]; then
    run_cli_for_platforms "${selected_platform}"
    "${SCRIPT_DIR}/build-wasm.sh"
    return
  fi

  case "${selected_mode}" in
    cli)
      run_cli_for_platforms "${selected_platform}"
      ;;
    wasm)
      "${SCRIPT_DIR}/build-wasm.sh"
      ;;
    *)
      echo "Usage: $0 <cli|wasm|all> [linux|windows|all]"
      echo "  arg1: build scope"
      echo "  arg2: optional CLI target platform(s), defaults to host platform"
      exit 1
      ;;
  esac
}

if [[ -z "${MODE}" ]]; then
  echo "Usage: $0 <cli|wasm|all> [linux|windows|all]"
  echo "Examples:"
  echo "  $0 cli"
  echo "  $0 cli windows"
  echo "  $0 cli all"
  echo "  $0 wasm"
  echo "  $0 all all"
  exit 1
fi

run_mode "${MODE}" "${PLATFORM}"
