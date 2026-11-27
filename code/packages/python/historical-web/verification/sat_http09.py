"""Bounded SAT evidence for the HTTP/0.9 request framing contract.

This is deliberately a small, dependency-free SAT runner.  The repository's
CDCL solvers are implemented in other language lanes; importing one here would
change this Python package's runtime dependencies.  The DPLL kernel below is
only the proof harness, not a replacement for VER09's solver or a production
proof checker.

The model has one variable for each possible byte class at each bounded
address position: ``safe`` (``!`` through ``~`` except space) or ``forbidden``.
It asks whether a request can be both accepted and contain a forbidden byte.
That query is UNSAT for lengths 0, 1, and 2.  A negative control drops the
acceptance restriction and is SAT, yielding a concrete forbidden-byte witness.
Finally, all class combinations are replayed through the public Python
exports.  The replay is finite conformance evidence, distinct from the model
result and from a whole-program proof.
"""

from __future__ import annotations

from itertools import product

from historical_web import encode_http09_request, parse_http09_request

Clause = tuple[int, ...]
Assignment = dict[int, bool]


def _unit_propagate(clauses: list[Clause], assignment: Assignment) -> bool:
    """Propagate units; return false when a clause is falsified."""
    changed = True
    while changed:
        changed = False
        for clause in clauses:
            values = [assignment.get(abs(literal)) for literal in clause]
            if any(
                value is not None and value == (literal > 0)
                for value, literal in zip(values, clause, strict=True)
            ):
                continue
            undecided = [
                literal
                for literal, value in zip(clause, values, strict=True)
                if value is None
            ]
            if not undecided:
                return False
            if len(undecided) == 1:
                literal = undecided[0]
                variable = abs(literal)
                wanted = literal > 0
                previous = assignment.get(variable)
                if previous is not None and previous != wanted:
                    return False
                if previous is None:
                    assignment[variable] = wanted
                    changed = True
    return True


def _solve(
    clauses: list[Clause], assignment: Assignment | None = None
) -> Assignment | None:
    """Return one satisfying assignment, or ``None`` (a tiny DPLL solver)."""
    current = {} if assignment is None else dict(assignment)
    if not _unit_propagate(clauses, current):
        return None
    variables = sorted({abs(literal) for clause in clauses for literal in clause})
    variable = next((item for item in variables if item not in current), None)
    if variable is None:
        return current
    for value in (True, False):
        branch = dict(current)
        branch[variable] = value
        result = _solve(clauses, branch)
        if result is not None:
            return result
    return None


def _model(
    length: int, require_acceptance: bool
) -> tuple[list[Clause], dict[tuple[int, str], int]]:
    """Build the bounded one-hot byte-class model and its variable map."""
    variables: dict[tuple[int, str], int] = {}
    next_variable = 1
    for position in range(length):
        for category in ("safe", "forbidden"):
            variables[position, category] = next_variable
            next_variable += 1

    clauses: list[Clause] = []
    for position in range(length):
        safe = variables[position, "safe"]
        forbidden = variables[position, "forbidden"]
        clauses.extend(((safe, forbidden), (-safe, -forbidden)))
    if require_acceptance:
        if length == 0:
            clauses.append(())  # the address is nonempty
        for position in range(length):
            clauses.append((variables[position, "safe"],))
        clauses.append(
            tuple(variables[position, "forbidden"] for position in range(length))
        )
    else:
        # The mutant control permits one forbidden byte in an accepted address.
        clauses.append(
            tuple(variables[position, "forbidden"] for position in range(length))
            if length
            else (),
        )
    return clauses, variables


def _classify(byte: int) -> str:
    return "safe" if 0x21 <= byte <= 0x7E else "forbidden"


def _actual_accepts(payload: bytes, terminator: bytes) -> bool:
    wire = b"GET " + payload + terminator
    try:
        parse_http09_request(wire)
    except ValueError:
        return False
    return True


def _exhaustive_conformance() -> int:
    """Replay every two-position class combination against public exports."""
    checked = 0
    # A space is an unambiguous forbidden address byte.  CR/LF are checked
    # below with a following byte, since a final CR can be the CR of CRLF.
    representatives = {"safe": b"A", "forbidden": b" "}
    for length in range(3):
        for classes in product(("safe", "forbidden"), repeat=length):
            payload = b"".join(representatives[item] for item in classes)
            expected = length > 0 and all(item == "safe" for item in classes)
            if expected:
                assert encode_http09_request(payload.decode("ascii")) == (
                    b"GET " + payload + b"\r\n"
                )
            else:
                try:
                    encode_http09_request(payload.decode("ascii"))
                except ValueError:
                    pass
                else:
                    raise AssertionError(f"encoder accepted {payload!r}")
            for terminator in (b"\n", b"\r\n"):
                assert _actual_accepts(payload, terminator) is expected, (
                    payload,
                    terminator,
                )
                checked += 1
    for wire in (b"GET A\rB\n", b"GET A\nB\n", b"GET A\r\nB\n"):
        assert not _actual_accepts(wire[4:-1], b"\n")
        checked += 1
    for address in ("A", "AA", "/index.html"):
        assert parse_http09_request(encode_http09_request(address)) == address
        checked += 1
    return checked


def run() -> None:
    unsat_queries = 0
    for length in range(3):
        clauses, _ = _model(length, require_acceptance=True)
        assert _solve(clauses) is None, f"counterexample at bounded length {length}"
        unsat_queries += 1

    clauses, variables = _model(2, require_acceptance=False)
    witness = _solve(clauses)
    assert witness is not None
    witness_classes = tuple(
        "safe" if witness[variables[position, "safe"]] else "forbidden"
        for position in range(2)
    )
    assert "forbidden" in witness_classes
    checked = _exhaustive_conformance()
    print(f"SAT model: {unsat_queries} UNSAT framing queries (lengths 0..2)")
    print(f"SAT negative control: {witness_classes!r} (forbidden-byte mutant witness)")
    print(f"public-export conformance: {checked} bounded cases")


if __name__ == "__main__":
    run()
