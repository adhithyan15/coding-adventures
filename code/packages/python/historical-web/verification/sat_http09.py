"""Bit-level bounded SAT evidence, delegated to the repository Go solver."""

from __future__ import annotations

import os
import subprocess
from itertools import product
from pathlib import Path

from historical_web import parse_http09_request


def _cnf(length: int, mutant: bool = False) -> str:
    """Encode byte bits, safe ASCII admission, and an unsafe-byte miter."""
    bits = length * 8
    bad = [bits + position + 1 for position in range(length)]
    clauses: list[tuple[int, ...]] = []

    def byte_clause(position: int, value: int) -> tuple[int, ...]:
        return tuple(
            -(position * 8 + bit + 1) if value & (1 << bit) else position * 8 + bit + 1
            for bit in range(8)
        )

    if not mutant:
        for position in range(length):
            for value in range(256):
                if not 0x21 <= value <= 0x7E:
                    clauses.append(byte_clause(position, value))
    if length == 0:
        clauses.append(())
    else:
        clauses.append(tuple(bad))
        if not mutant:
            for position, selector in enumerate(bad):
                for value in range(0x21, 0x7F):
                    clauses.append((-selector, *byte_clause(position, value)))
    variable_count = bits + length
    body = [f"p cnf {variable_count} {len(clauses)}"]
    body.extend(" ".join(map(str, clause)) + " 0" for clause in clauses)
    return "\n".join(body) + "\n"


def _solve(cnf: str) -> str:
    go = os.environ.get("HWEB_GO", "/workspace/.coding-adventures-tools/go/bin/go")
    bridge = Path(__file__).with_name("go_sat")
    result = subprocess.run(
        [go, "run", "."], cwd=bridge, input=cnf, text=True,
        capture_output=True, check=True,
    )
    return result.stdout.strip()


def _replay() -> int:
    checked = 0
    for length in range(3):
        for payload in product(range(256), repeat=length):
            for terminator in (b"\n", b"\r\n"):
                # With a bare LF terminator, a final CR is consumed as the
                # optional CR of CRLF.  This is the historical parser's exact
                # framing semantics, not an admission-model claim.
                expected = length > 0 and all(0x21 <= item <= 0x7E for item in payload)
                if terminator == b"\n" and payload and payload[-1] == 0x0D:
                    expected = bool(payload[:-1]) and all(
                        0x21 <= item <= 0x7E for item in payload[:-1]
                    )
                try:
                    parse_http09_request(b"GET " + bytes(payload) + terminator)
                except ValueError:
                    actual = False
                else:
                    actual = True
                assert actual is expected, (payload, terminator)
                checked += 1
    return checked


def run() -> None:
    for length in range(3):
        assert _solve(_cnf(length)) == "UNSAT"
    assert _solve(_cnf(2, mutant=True)) == "SAT"
    checked = _replay()
    print("Go CDCL SAT model: 3 UNSAT bit-level framing queries (lengths 0..2)")
    print("Go CDCL negative control: SAT (admission clauses removed)")
    print(f"public-parser octet conformance: {checked} cases")


if __name__ == "__main__":
    run()
