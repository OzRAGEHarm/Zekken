#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional

ROOT = Path(__file__).resolve().parent.parent
BENCH_DIR = ROOT / "benchmarks"
PY_BENCH_DIR = BENCH_DIR / "python"


@dataclass
class RunResult:
    ok: bool
    wall_s: float
    rss_kb: Optional[int]
    ops: Optional[float]
    checksum: Optional[str]
    stdout: str
    stderr: str


@dataclass
class BenchCase:
    name: str
    zk_file: Path
    py_file: Path


CASES = [
    BenchCase("cpu_numeric", BENCH_DIR / "cpu_numeric.zk", PY_BENCH_DIR / "cpu_numeric.py"),
    BenchCase("branch_calls", BENCH_DIR / "branch_calls.zk", PY_BENCH_DIR / "branch_calls.py"),
    BenchCase("queue_cleanup", BENCH_DIR / "queue_cleanup.zk", PY_BENCH_DIR / "queue_cleanup.py"),
    BenchCase("file_io_transform", BENCH_DIR / "file_io_transform.zk", PY_BENCH_DIR / "file_io_transform.py"),
    BenchCase("matrix_mixed", BENCH_DIR / "matrix_mixed.zk", PY_BENCH_DIR / "matrix_mixed.py"),
]

REAL_RE = re.compile(r"REAL=([0-9]*\.?[0-9]+)")
RSS_RE = re.compile(r"RSS_KB=(\d+)")
OPS_RE = re.compile(r"^Operations:\s*([0-9]+(?:\.[0-9]+)?)\s*$", re.MULTILINE)
CHECKSUM_RE = re.compile(r"^Checksum:\s*(.+?)\s*$", re.MULTILINE)


def run_command(cmd: List[str], cwd: Path, env_overrides: Optional[Dict[str, str]] = None) -> RunResult:
    time_bin = "/usr/bin/time" if Path("/usr/bin/time").exists() else None
    wrapped = cmd
    used_time = False
    if time_bin:
        wrapped = [time_bin, "-f", "REAL=%e RSS_KB=%M"] + cmd
        used_time = True

    start = time.perf_counter()
    proc = subprocess.run(
        wrapped,
        cwd=str(cwd),
        text=True,
        capture_output=True,
        env={**os.environ, **(env_overrides or {})},
    )
    end = time.perf_counter()

    stdout = proc.stdout
    stderr = proc.stderr

    wall_s = end - start
    rss_kb: Optional[int] = None

    if used_time:
        m_real = REAL_RE.search(stderr)
        m_rss = RSS_RE.search(stderr)
        if m_real:
            wall_s = float(m_real.group(1))
        if m_rss:
            rss_kb = int(m_rss.group(1))

    m_ops = OPS_RE.search(stdout)
    m_checksum = CHECKSUM_RE.search(stdout)
    ops = float(m_ops.group(1)) if m_ops else None
    checksum = m_checksum.group(1).strip() if m_checksum else None

    return RunResult(
        ok=proc.returncode == 0,
        wall_s=wall_s,
        rss_kb=rss_kb,
        ops=ops,
        checksum=checksum,
        stdout=stdout,
        stderr=stderr,
    )


def summarize(results: List[RunResult]) -> dict:
    wall = [r.wall_s for r in results]
    rss_vals = [r.rss_kb for r in results if r.rss_kb is not None]
    ops = results[0].ops if results and results[0].ops is not None else None
    avg_wall = statistics.mean(wall)
    return {
        "runs": len(results),
        "avg_s": avg_wall,
        "min_s": min(wall),
        "max_s": max(wall),
        "rss_kb": int(statistics.mean(rss_vals)) if rss_vals else None,
        "ops": ops,
        "ops_per_s": (ops / avg_wall) if ops and avg_wall > 0 else None,
        "checksum": results[0].checksum if results else None,
    }


def format_num(n: Optional[float], digits: int = 2) -> str:
    if n is None:
        return "n/a"
    return f"{n:.{digits}f}"


def resolve_zekken_binary(user_bin: Optional[str]) -> str:
    if user_bin:
        return user_bin
    local_release = ROOT / "target" / "release" / "zekken"
    if local_release.exists():
        return str(local_release)
    return "zekken"


def select_cases(selected: List[str]) -> List[BenchCase]:
    if not selected:
        return CASES
    names = {n.strip() for n in selected}
    chosen = [c for c in CASES if c.name in names]
    missing = sorted(names - {c.name for c in chosen})
    if missing:
        raise SystemExit(f"Unknown benchmark name(s): {', '.join(missing)}")
    return chosen


def print_summary_row(engine: str, name: str, s: dict) -> None:
    print(
        f"{engine:7} {name:18} "
        f"avg={format_num(s['avg_s'], 4)}s "
        f"min={format_num(s['min_s'], 4)}s "
        f"max={format_num(s['max_s'], 4)}s "
        f"ops/s={format_num(s['ops_per_s'], 2)} "
        f"rss_kb={s['rss_kb'] if s['rss_kb'] is not None else 'n/a'}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Zekken/Python benchmark suite")
    parser.add_argument("--vm", action="store_true", help="Uses VM instead of tree walk evaluator for tests")
    parser.add_argument("--engine", choices=["zekken", "python", "both"], default="both")
    parser.add_argument("--runs", type=int, default=3, help="Runs per benchmark")
    parser.add_argument("--bench", action="append", default=[], help="Benchmark name to run (repeatable)")
    parser.add_argument("--zekken-bin", default=None, help="Path to zekken binary")
    parser.add_argument("--python-bin", default=sys.executable, help="Path to python binary")
    parser.add_argument("--show-output", action="store_true", help="Print benchmark stdout/stderr")
    parser.add_argument(
        "--suppress-zekken-print",
        action="store_true",
        help="Set ZEKKEN_DISABLE_PRINT=1 for Zekken benchmark runs",
    )
    args = parser.parse_args()

    if args.runs < 1:
        raise SystemExit("--runs must be >= 1")

    cases = select_cases(args.bench)
    zekken_bin = resolve_zekken_binary(args.zekken_bin)

    print(f"Suite root: {ROOT}")
    print(f"Runs per benchmark: {args.runs}")
    print(f"Selected: {', '.join(c.name for c in cases)}")
    if args.engine in ("zekken", "both"):
        print(f"Zekken binary: {zekken_bin}")
    if args.engine in ("python", "both"):
        print(f"Python binary: {args.python_bin}")
    print()

    had_failure = False

    for case in cases:
        if args.engine in ("zekken", "both"):
            runs: List[RunResult] = []
            z_env = {"ZEKKEN_DISABLE_PRINT": "1"} if args.suppress_zekken_print else None
            for _ in range(args.runs):
                run_args = ["run"]
                if args.vm:
                    run_args.append("--vm")
                run_args.append(str(case.zk_file))
                result = run_command([zekken_bin] + run_args, ROOT, env_overrides=z_env)
                if args.show_output:
                    print(result.stdout, end="")
                    if result.stderr:
                        print(result.stderr, end="", file=sys.stderr)
                if not result.ok:
                    had_failure = True
                    print(f"[ERROR] zekken {case.name} failed", file=sys.stderr)
                    print(result.stderr, file=sys.stderr)
                    break
                runs.append(result)
            if runs:
                summary = summarize(runs)
                print_summary_row("zekken", case.name, summary)

        if args.engine in ("python", "both"):
            runs = []
            for _ in range(args.runs):
                result = run_command([args.python_bin, str(case.py_file)], ROOT)
                if args.show_output:
                    print(result.stdout, end="")
                    if result.stderr:
                        print(result.stderr, end="", file=sys.stderr)
                if not result.ok:
                    had_failure = True
                    print(f"[ERROR] python {case.name} failed", file=sys.stderr)
                    print(result.stderr, file=sys.stderr)
                    break
                runs.append(result)
            if runs:
                summary = summarize(runs)
                print_summary_row("python", case.name, summary)

    return 1 if had_failure else 0


if __name__ == "__main__":
    raise SystemExit(main())
