import math

ITERATIONS = 350_000


def run(n: int) -> float:
    i = 0
    x = 0.713521
    y = 1.381966
    z = 0.577215

    while i < n:
        a = math.sin(x)
        b = math.cos(y)
        c = math.tan(z * 0.1)

        x = (a + (b * 1.0000003)) + ((i % 13) * 0.000001)
        y = (b + (c * 0.9999997)) + ((i % 17) * 0.000001)
        z = (c + (a * 1.0000001)) + ((i % 19) * 0.000001)

        if x > 10000.0:
            x = x / 10000.0
        elif x < -10000.0:
            x = x / -10000.0

        i += 1

    return x + y + z


checksum = run(ITERATIONS)
print("Benchmark: cpu_numeric")
print(f"Operations: {ITERATIONS * 10}")
print(f"Checksum: {checksum}")
