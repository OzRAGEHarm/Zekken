from pathlib import Path

PATH = Path("bench_io_temp.txt")
ROUNDS = 450


def build_blob(round_idx: int) -> str:
    lines = []
    i = 0
    while i < 40:
        lines.append(f"line-{round_idx}-{i}\n")
        i += 1
    return "".join(lines)


def run(path: Path, rounds: int) -> int:
    i = 0
    checksum = 0

    while i < rounds:
        payload = build_blob(i)
        path.write_text(payload)
        data = path.read_text()
        checksum += len(data)
        i += 1

    if path.exists():
        path.unlink()
    return checksum


checksum = run(PATH, ROUNDS)
print("Benchmark: file_io_transform")
print(f"Operations: {ROUNDS * 3}")
print(f"Checksum: {checksum}")
