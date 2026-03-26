ITERATIONS = 450_000


def mix_state(state: int, code: str, flip: bool) -> int:
    if code == "A":
        if flip:
            return (state * 3 + 11) % 2147483647
        return (state * 5 + 7) % 2147483647
    if code == "B":
        if flip:
            return (state * 7 + 13) % 2147483647
        return (state * 11 + 17) % 2147483647
    if code == "C":
        if flip:
            return (state * 13 + 19) % 2147483647
        return (state * 17 + 23) % 2147483647
    if flip:
        return (state * 19 + 29) % 2147483647
    return (state * 23 + 31) % 2147483647


def run(n: int) -> int:
    i = 0
    state = 123456789
    flip = True
    code = "A"

    while i < n:
        m = state % 4
        if m == 0:
            code = "A"
        elif m == 1:
            code = "B"
        elif m == 2:
            code = "C"
        else:
            code = "D"

        state = mix_state(state, code, flip)
        flip = not flip
        i += 1

    return state


checksum = run(ITERATIONS)
print("Benchmark: branch_calls")
print(f"Operations: {ITERATIONS * 8}")
print(f"Checksum: {checksum}")
