import math

SIZE = 20
ROUNDS = 5


def build_matrix(n: int, seed: float):
    m = []
    i = 0
    while i < n:
        row = []
        j = 0
        while j < n:
            x = (i * 0.137) + (j * 0.271) + seed
            v = math.sin(x) + math.cos(x / 2.0)
            row.append(v)
            j += 1
        m.append(row)
        i += 1
    return m


def matmul_square(a, b, n: int):
    out = []
    i = 0
    while i < n:
        row = []
        j = 0
        while j < n:
            s = 0.0
            k = 0
            while k < n:
                s += a[i][k] * b[k][j]
                k += 1
            row.append(s)
            j += 1
        out.append(row)
        i += 1
    return out


def checksum(m, n: int) -> float:
    total = 0.0
    i = 0
    while i < n:
        j = 0
        while j < n:
            total += m[i][j]
            j += 1
        i += 1
    return total


a = build_matrix(SIZE, 0.35)
b = build_matrix(SIZE, 1.75)

round_i = 0
while round_i < ROUNDS:
    a = matmul_square(a, b, SIZE)
    b = matmul_square(b, a, SIZE)
    round_i += 1

final_sum = checksum(a, SIZE)
print("Benchmark: matrix_mixed")
print(f"Operations: {SIZE * SIZE * SIZE * ROUNDS * 2}")
print(f"Checksum: {final_sum}")
