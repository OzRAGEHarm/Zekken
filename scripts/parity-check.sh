#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BIN="${BIN:-target/debug/zekken}"
WORK_DIR="${WORK_DIR:-/tmp/zekken-parity}"

mkdir -p "$WORK_DIR"
rm -f "$WORK_DIR"/*

echo "[parity] building debug binary..."
cargo build >/dev/null

total=0
ok=0
skip=0
fail=0

while IFS= read -r file; do
  total=$((total + 1))
  if grep -q '@input' "$file"; then
    echo "SKIP_INTERACTIVE $file"
    skip=$((skip + 1))
    continue
  fi

  out_file="$WORK_DIR/$(echo "$file" | tr '/' '_').raw"
  if "$BIN" run "$file" >"$out_file" 2>&1; then
    echo "OK $file"
    ok=$((ok + 1))
  else
    echo "FAIL $file"
    cat "$out_file" || true
    fail=$((fail + 1))
  fi
done < <(find tests examples -type f -name '*.zk' | sort)

echo
echo "[parity] total=$total ok=$ok skip=$skip fail=$fail"
if [[ $fail -gt 0 ]]; then
  exit 1
fi
