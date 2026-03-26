# Benchmark Suite

This suite stresses different runtime behaviors beyond pure CPU math loops.

## Benchmarks

- `cpu_numeric.zk` / `python/cpu_numeric.py`
  - Scalar numeric + trig heavy loop.
- `branch_calls.zk` / `python/branch_calls.py`
  - Branch-heavy and function-call-heavy loop.
- `queue_cleanup.zk` / `python/queue_cleanup.py`
  - Long-lived queue workload (native deque in Zekken via `queue_new`).
- `file_io_transform.zk` / `python/file_io_transform.py`
  - Repeated file write/read + string transform.
- `matrix_mixed.zk` / `python/matrix_mixed.py`
  - Dense nested loops and matrix-style computation.

Each benchmark prints:
- `Benchmark: <name>`
- `Operations: <n>`
- `Checksum: <value>`

The runner uses `Operations` + average wall time to report ops/sec.

## Runner

Use:

```bash
python3 scripts/run-benchmarks.py --engine both --runs 3
```

Examples:

```bash
# Zekken only
python3 scripts/run-benchmarks.py --engine zekken --runs 5

# Python only
python3 scripts/run-benchmarks.py --engine python --runs 5

# Single benchmark
python3 scripts/run-benchmarks.py --engine both --bench cpu_numeric --runs 3

# Explicit Zekken binary
python3 scripts/run-benchmarks.py --engine zekken --zekken-bin ./target/release/zekken

# Suppress benchmark prints inside Zekken runtime
python3 scripts/run-benchmarks.py --engine zekken --suppress-zekken-print
```

## Notes

- On Linux, the runner uses `/usr/bin/time` to capture peak RSS (KB).
- If `/usr/bin/time` is unavailable, wall time still works and RSS shows `n/a`.
- Run in release mode for meaningful Zekken numbers.
