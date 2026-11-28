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
_H10_SUFFIX = b" HTTP/1.0\r\n"


class CNF:
    """Small DIMACS builder for bit-level request bytes and predicates."""

    def __init__(self) -> None:
        self.variable_count = 0
        self.clauses: list[tuple[int, ...]] = []

    def variable(self) -> int:
        self.variable_count += 1
        return self.variable_count

    def clause(self, *literals: int) -> None:
        self.clauses.append(tuple(literals))

    def dimacs(self) -> str:
        lines = [f"p cnf {self.variable_count} {len(self.clauses)}"]
        lines.extend(" ".join(map(str, clause)) + " 0" for clause in self.clauses)
        return "\n".join(lines) + "\n"


def _fixed_byte(formula: CNF, bits: tuple[int, ...], value: int) -> None:
    for bit, variable in enumerate(bits):
        formula.clause(variable if value & (1 << bit) else -variable)


def _byte_not_value(bits: tuple[int, ...], value: int) -> tuple[int, ...]:
    """Return a clause false only when `bits` encode exactly `value`."""
    return tuple(
        -variable if value & (1 << bit) else variable
        for bit, variable in enumerate(bits)
    )


def _constrain_byte_range(
    formula: CNF, bits: tuple[int, ...], lower: int, upper: int
) -> None:
    # The high bit clears the entire 0x80..0xff half in one clause. Only
    # exclude the small forbidden low and DEL ranges byte by byte.
    formula.clause(-bits[7])
    for value in range(lower):
        formula.clause(*_byte_not_value(bits, value))
    for value in range(upper + 1, 0x80):
        formula.clause(*_byte_not_value(bits, value))


def _request_line_model(target_length: int, *, h00_mode: str) -> CNF:
    """Encode one shared byte vector and both protocol interpretations.

    `strict` constrains HWEB00's projected address to 0x21..0x7e. `relaxed`
    changes only the lower bound to 0x20 for the negative control.
    """
    if target_length not in _LENGTHS:
        raise ValueError(f"unsupported bounded target length: {target_length}")
    if h00_mode not in {"absent", "strict", "relaxed"}:
        raise ValueError(f"unknown H00 miter mode: {h00_mode}")

    formula = CNF()
    prefix = b"GET "
    wire_length = len(prefix) + target_length + len(_H10_SUFFIX)
    wire = tuple(
        tuple(formula.variable() for _ in range(8)) for _ in range(wire_length)
    )

    # A shared vector constructs exact `GET SP target SP HTTP/1.0 CRLF` bytes.
    # The target starts with '/', then uses only HWEB08's closed tail alphabet.
    for position, byte in enumerate(prefix):
        _fixed_byte(formula, wire[position], byte)
    target_start = len(prefix)
    _fixed_byte(formula, wire[target_start], ord("/"))
    target_tail = {ord(byte) for byte in _TARGET_TAIL}
    for offset in range(1, target_length):
        bits = wire[target_start + offset]
        for byte in range(256):
            if byte not in target_tail:
                formula.clause(*_byte_not_value(bits, byte))
    suffix_start = target_start + target_length
    for offset, byte in enumerate(_H10_SUFFIX):
        _fixed_byte(formula, wire[suffix_start + offset], byte)

    if h00_mode != "absent":
        # H00's address is the same wire slice after `GET SP` and before the
        # final CRLF. This includes H10's separator and version token.
        lower = 0x21 if h00_mode == "strict" else 0x20
        for bits in wire[len(prefix) : wire_length - 2]:
            _constrain_byte_range(formula, bits, lower, 0x7e)

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
