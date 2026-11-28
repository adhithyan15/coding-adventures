"""Bounded SAT miter for the HWEB00/HWEB08 request-line boundary."""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

# The HWEB08 target is '/' followed by zero or more bytes from this set.
_TARGET_TAIL = tuple(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/._-"
)
_LENGTHS = (1, 2, 3)  # Total target length, including the leading slash.


class CNF:
    """Small DIMACS builder for one-hot byte choices and Boolean predicates."""

    def __init__(self) -> None:
        self.variable_count = 0
        self.clauses: list[tuple[int, ...]] = []

    def variable(self) -> int:
        self.variable_count += 1
        return self.variable_count

    def clause(self, *literals: int) -> None:
        self.clauses.append(tuple(literals))

    def exactly_one(self, variables: tuple[int, ...]) -> None:
        self.clause(*variables)
        for left_index, left in enumerate(variables):
            for right in variables[left_index + 1 :]:
                self.clause(-left, -right)

    def dimacs(self) -> str:
        lines = [f"p cnf {self.variable_count} {len(self.clauses)}"]
        lines.extend(" ".join(map(str, clause)) + " 0" for clause in self.clauses)
        return "\n".join(lines) + "\n"


def _request_line_model(target_length: int, *, h00_mode: str) -> CNF:
    """Encode canonical H10 syntax and optionally require H00 acceptance.

    `strict` includes HWEB00's printable-nonspace address guard. `relaxed`
    removes only that guard's no-space lower bound for a negative control.
    """
    if target_length not in _LENGTHS:
        raise ValueError(f"unsupported bounded target length: {target_length}")
    if h00_mode not in {"absent", "strict", "relaxed"}:
        raise ValueError(f"unknown H00 miter mode: {h00_mode}")

    formula = CNF()
    target_choices: list[dict[str, int]] = []
    for position in range(target_length):
        alphabet = ("/",) if position == 0 else _TARGET_TAIL
        choices = {byte: formula.variable() for byte in alphabet}
        formula.exactly_one(tuple(choices.values()))
        target_choices.append(choices)

    # Every assignment constructs one legal H10 line from fixed `GET `, the
    # symbolic permitted target, fixed SP + `HTTP/1.0`, and canonical CRLF.

    if h00_mode != "absent":
        h00_accepted = formula.variable()
        separator_is_space = formula.variable()
        formula.clause(h00_accepted)
        formula.clause(separator_is_space)
        if h00_mode == "strict":
            # H00 sees the same prefix and final CRLF; target and version
            # characters are printable and nonspace. Its full address spans
            # this fixed H10 separator, violating the H00 no-space guard.
            formula.clause(-h00_accepted, -separator_is_space)

    return formula


def _solve(formula: CNF) -> str:
    bridge = Path(__file__).with_name("go_sat")
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."],
        cwd=bridge,
        input=formula.dimacs(),
        text=True,
        capture_output=True,
        check=True,
        timeout=30,
    )
    return result.stdout.strip().upper()


def _expect(label: str, formula: CNF, expected: str) -> None:
    actual = _solve(formula)
    if actual != expected:
        raise RuntimeError(f"{label}: solver returned {actual}, expected {expected}")


def run() -> None:
    for target_length in _LENGTHS:
        _expect(
            f"HWEB08 legal GET target length {target_length}",
            _request_line_model(target_length, h00_mode="absent"),
            "SAT",
        )
        _expect(
            f"HWEB08/HWEB00 shared-wire miter length {target_length}",
            _request_line_model(target_length, h00_mode="strict"),
            "UNSAT",
        )
        _expect(
            f"relaxed HWEB00 space-guard mutant length {target_length}",
            _request_line_model(target_length, h00_mode="relaxed"),
            "SAT",
        )

    print(
        "Go CDCL bounded request-line miter: common method GET; "
        "target lengths 1..3"
    )
    print(
        "Target domain: '/' followed by zero to two ASCII alnum, '/', '.', "
        "'_', or '-'"
    )
    print(
        "HWEB08 legal canonical CRLF line alone: SAT at each bounded target "
        "length"
    )
    print(
        "Same HWEB08 bytes also accepted by strict HWEB00: UNSAT at each "
        "target length"
    )
    print(
        "Negative control relaxing HWEB00's printable-nonspace guard: SAT "
        "at each length"
    )
    print(
        "Bounded language miter only; this does not prove scanner/source "
        "equivalence."
    )
    print("Newer HWEB08 requires CRLF; HWEB00 separately accepts LF or CRLF.")


if __name__ == "__main__":
    run()
