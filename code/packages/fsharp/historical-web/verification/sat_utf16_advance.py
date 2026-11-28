"""SAT check for the HWEB11 UTF-16 high-surrogate scanner step."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def _forbid(assignments: list[tuple[int, bool]]) -> tuple[int, ...]:
    return tuple(-var if value else var for var, value in assignments)


def _cnf(*, check_next_unit: bool, require_negative_remaining: bool) -> str:
    """Model a valid current code unit and the transition consuming two units."""
    width = 32
    remaining = list(range(1, width + 1))
    after_advance = list(range(width + 1, 2 * width + 1))
    carry = list(range(2 * width + 1, 3 * width + 2))
    clauses: list[tuple[int, ...]] = [(-carry[0],)]

    # remaining = source.Length - index; a scanner loop iteration has remaining >= 1.
    clauses.append((-remaining[-1],))
    clauses.append(tuple(remaining[:-1]))

    # The source tests `index + 1 >= source.Length` before entering the branch,
    # equivalent to remaining == 1 for a valid current index.
    if check_next_unit:
        clauses.append(
            _forbid(
                [
                    (variable, bool(1 & (1 << bit)))
                    for bit, variable in enumerate(remaining)
                ]
            )
        )

    # Encode 32-bit subtraction by two as addition of the two's-complement constant.
    constant = (1 << width) - 2
    for bit in range(width):
        a, out, carry_in, carry_out = (
            remaining[bit],
            after_advance[bit],
            carry[bit],
            carry[bit + 1],
        )
        addend = bool(constant & (1 << bit))
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
                                        (carry_in, carry_value),
                                        (out, out_value),
                                        (carry_out, next_value),
                                    ]
                                )
                            )

    if require_negative_remaining:
        clauses.append((after_advance[-1],))

    count = 3 * width + 1
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
    legal = _solve(_cnf(check_next_unit=True, require_negative_remaining=False))
    guarded_bad_step = _solve(
        _cnf(check_next_unit=True, require_negative_remaining=True)
    )
    mutant_bad_step = _solve(
        _cnf(check_next_unit=False, require_negative_remaining=True)
    )
    expected = ("SAT", "UNSAT", "SAT")
    actual = (legal, guarded_bad_step, mutant_bad_step)
    if actual != expected:
        raise SystemExit(f"expected {expected}, got {actual}")
    print("valid UTF-16 high-surrogate pair advance: SAT")
    print("guarded advance past source end: UNSAT")
    print("mutant without next-code-unit bounds check: SAT")


if __name__ == "__main__":
    main()
