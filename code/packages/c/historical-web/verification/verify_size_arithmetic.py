"""Bounded SAT check for the C HTTP framing-size guard.

This proves only the unsigned arithmetic implication ``length <= SIZE_MAX - 6
=> length + 6 does not wrap`` at 32 and 64 bits.  It does not model C source,
allocation, pointer validity, or the transport implementation.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def _clause_forbidden(assignments: list[tuple[int, bool]]) -> tuple[int, ...]:
    """Return a clause false only when every listed literal has its value."""
    return tuple(-variable if value else variable for variable, value in assignments)


def _cnf(width: int, include_guard: bool) -> str:
    """Encode ``x + 6`` with a ripple adder and ask whether it overflows."""
    constant = 6
    x = list(range(1, width + 1))
    summed = list(range(width + 1, 2 * width + 1))
    carries = list(range(2 * width + 1, 3 * width + 2))
    clauses: list[tuple[int, ...]] = [(-carries[0],), (carries[-1],)]

    for bit in range(width):
        a, out, carry_in, carry_out = x[bit], summed[bit], carries[bit], carries[bit + 1]
        fixed = bool(constant & (1 << bit))
        for a_value in (False, True):
            for carry_value in (False, True):
                total = int(a_value) + int(carry_value) + int(fixed)
                for out_value in (False, True):
                    for next_value in (False, True):
                        if (out_value, next_value) != (bool(total & 1), bool(total >> 1)):
                            clauses.append(_clause_forbidden([
                                (a, a_value), (carry_in, carry_value),
                                (out, out_value), (carry_out, next_value),
                            ]))

    if include_guard:
        # Only six values violate x <= SIZE_MAX - 6. Exclude each exact value;
        # this is a compact, exact encoding for this fixed subtraction.
        maximum = (1 << width) - 1
        for value in range(maximum - 5, maximum + 1):
            clauses.append(_clause_forbidden([
                (variable, bool(value & (1 << bit)))
                for bit, variable in enumerate(x)
            ]))

    body = [f"p cnf {3 * width + 1} {len(clauses)}"]
    body.extend(" ".join(map(str, clause)) + " 0" for clause in clauses)
    return "\n".join(body) + "\n"


def _solve(cnf: str) -> str:
    root = Path(__file__).resolve().parents[5]
    bridge = root / "code/packages/python/historical-web/verification/go_sat"
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."], cwd=bridge, input=cnf, text=True,
        capture_output=True, check=True, timeout=60,
    )
    return result.stdout.strip().upper()


def main() -> None:
    receipts: list[str] = []
    for width in (32, 64):
        guarded = _solve(_cnf(width, True))
        weakened = _solve(_cnf(width, False))
        if guarded != "UNSAT" or weakened != "SAT":
            raise SystemExit(f"width {width}: guarded={guarded}, weakened={weakened}")
        receipts.append(f"{width}-bit guarded=UNSAT weakened=SAT")
    for receipt in receipts:
        print(receipt)
    print("The claim is bounded arithmetic only; no C source, allocator, or pointer proof is made.")


if __name__ == "__main__":
    main()
