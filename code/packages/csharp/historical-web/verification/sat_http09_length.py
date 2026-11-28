"""SAT check for the checked HTTP/0.9 request length addition in Http09."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def _forbid(assignments: list[tuple[int, bool]]) -> tuple[int, ...]:
    return tuple(-var if value else var for var, value in assignments)


def _cnf(*, guard_max_length: bool, require_negative_length: bool) -> str:
    """Encode the source's checked ``4 + length`` then ``addressEnd + 2``."""
    width = 32
    length = list(range(1, width + 1))
    address_end = list(range(width + 1, 2 * width + 1))
    request_length = list(range(2 * width + 1, 3 * width + 1))
    carry_first = list(range(3 * width + 1, 4 * width + 2))
    carry_second = list(range(4 * width + 2, 5 * width + 3))
    clauses: list[tuple[int, ...]] = [(-carry_first[0],), (-carry_second[0],)]

    # The encoder rejects empty addresses, and Span.Length is a nonnegative Int32.
    clauses.append((-length[-1],))
    clauses.append(tuple(length[:-1]))

    def add_constant(
        source: list[int], output: list[int], carry: list[int], value: int
    ) -> None:
        for bit in range(width):
            a, out, cin, cout = source[bit], output[bit], carry[bit], carry[bit + 1]
            addend = bool(value & (1 << bit))
            for a_value in (False, True):
                for carry_value in (False, True):
                    total = int(a_value) + int(carry_value) + int(addend)
                    for out_value in (False, True):
                        for next_value in (False, True):
                            if (out_value, next_value) != (
                                bool(total & 1),
                                bool(total >> 1),
                            ):
                                clauses.append(
                                    _forbid(
                                        [
                                            (a, a_value),
                                            (cin, carry_value),
                                            (out, out_value),
                                            (cout, next_value),
                                        ]
                                    )
                                )

    add_constant(length, address_end, carry_first, 4)
    add_constant(address_end, request_length, carry_second, 2)

    # A successful checked Int32 addition requires Length <= Int32.MaxValue - 6.
    # The only excluded nonnegative values are the final six Int32 values.
    if guard_max_length:
        maximum = (1 << (width - 1)) - 1
        for value in range(maximum - 5, maximum + 1):
            clauses.append(
                _forbid(
                    [
                        (variable, bool(value & (1 << bit)))
                        for bit, variable in enumerate(length)
                    ]
                )
            )
    if require_negative_length:
        clauses.append((address_end[-1], request_length[-1]))

    count = 5 * width + 2
    lines = [f"p cnf {count} {len(clauses)}"]
    lines.extend(" ".join(map(str, clause)) + " 0" for clause in clauses)
    return "\n".join(lines) + "\n"


def _solve(cnf: str) -> str:
    root = Path(__file__).resolve().parents[5]
    bridge = root / "code/packages/python/historical-web/verification/go_sat"
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."],
        cwd=bridge,
        input=cnf,
        text=True,
        capture_output=True,
        check=True,
        timeout=60,
    )
    return result.stdout.strip().upper()


def main() -> None:
    legal = _solve(_cnf(guard_max_length=True, require_negative_length=False))
    guarded_overflow = _solve(_cnf(guard_max_length=True, require_negative_length=True))
    mutant_overflow = _solve(_cnf(guard_max_length=False, require_negative_length=True))
    expected = ("SAT", "UNSAT", "SAT")
    actual = (legal, guarded_overflow, mutant_overflow)
    if actual != expected:
        raise SystemExit(f"expected {expected}, got {actual}")
    print("checked `4 + length`, then `addressEnd + 2`: legal completion SAT")
    print("max-length-guarded additions with negative intermediate/final: UNSAT")
    print("mutant without max-length guard and negative intermediate/final: SAT")


if __name__ == "__main__":
    main()
