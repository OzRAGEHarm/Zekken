from collections import deque

ENQUEUE_TOTAL = 90_000


def run(total: int) -> int:
    q = deque()
    i = 0
    checksum = 0

    while i < total:
        q.append(((i * 31) + 7) % 1_000_003)

        if i % 2 == 0 and q:
            checksum += q.popleft()

        i += 1

    while q:
        checksum += q.popleft()

    return checksum


checksum = run(ENQUEUE_TOTAL)
print("Benchmark: queue_cleanup")
print(f"Operations: {ENQUEUE_TOTAL * 6}")
print(f"Checksum: {checksum}")
