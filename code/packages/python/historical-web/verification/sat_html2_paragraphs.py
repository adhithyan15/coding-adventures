"""Bounded SAT model for the strict HWEB06 P/H1 block transitions.

The model deliberately contains structural tokens only; text and attributes are
outside this slice. It delegates DIMACS to the repository's Go CDCL solver.
"""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

TOKENS = ("P", "/P", "H1", "/H1", "EOF")
ROOT, PARAGRAPH, HEADING, NESTED_P_H1, NESTED_P_P = range(5)
STATES = (ROOT, PARAGRAPH, HEADING, NESTED_P_H1, NESTED_P_P)
LENGTH = 4  # Exactly four structural token slots, with EOF in the last slot.


class CNF:
    def __init__(self) -> None:
        self.count = 0
        self.clauses: list[tuple[int, ...]] = []

    def var(self) -> int:
        self.count += 1
        return self.count

    def clause(self, *literals: int) -> None:
        self.clauses.append(tuple(literals))

    def exactly_one(self, literals: tuple[int, ...]) -> None:
        self.clause(*literals)
        for i, left in enumerate(literals):
            for right in literals[i + 1 :]:
                self.clause(-left, -right)

    def dimacs(self) -> str:
        lines = [f"p cnf {self.count} {len(self.clauses)}"]
        lines.extend(" ".join(map(str, clause)) + " 0" for clause in self.clauses)
        return "\n".join(lines) + "\n"


def _model(
    *,
    mutant: str | None,
    fixed_trace: tuple[str, ...] | None,
    seek_nested: bool,
) -> CNF:
    formula = CNF()
    events = [{token: formula.var() for token in TOKENS} for _ in range(LENGTH)]
    states = [{state: formula.var() for state in STATES} for _ in range(LENGTH + 1)]
    for row in events:
        formula.exactly_one(tuple(row.values()))
    for row in states:
        formula.exactly_one(tuple(row.values()))
    formula.clause(states[0][ROOT])
    formula.clause(events[-1]["EOF"])
    for slot in range(LENGTH - 1):
        formula.clause(-events[slot]["EOF"])

    # The HWEB06 structural transitions: P repeats replace the prior P;
    # H1 starts after closing P; H1 itself needs an explicit end tag.
    transitions = {
        (ROOT, "P"): PARAGRAPH,
        (ROOT, "H1"): HEADING,
        (ROOT, "EOF"): ROOT,
        (PARAGRAPH, "P"): PARAGRAPH,
        (PARAGRAPH, "/P"): ROOT,
        (PARAGRAPH, "H1"): HEADING,
        (PARAGRAPH, "EOF"): ROOT,
        (HEADING, "/H1"): ROOT,
    }
    if mutant == "P-to-H1 close removed":
        transitions[(PARAGRAPH, "H1")] = NESTED_P_H1
        transitions[(NESTED_P_H1, "/H1")] = PARAGRAPH
    elif mutant == "P-to-P close removed":
        transitions[(PARAGRAPH, "P")] = NESTED_P_P
        transitions[(NESTED_P_P, "/P")] = PARAGRAPH

    for slot in range(LENGTH):
        for state in STATES:
            for token in TOKENS:
                next_state = transitions.get((state, token))
                if next_state is None:
                    formula.clause(-states[slot][state], -events[slot][token])
                else:
                    # Exactly-one next-state constraints make this single
                    # implication sufficient to determine the transition.
                    formula.clause(
                        -states[slot][state],
                        -events[slot][token],
                        states[slot + 1][next_state],
                    )

    formula.clause(states[LENGTH][ROOT])
    if fixed_trace is not None:
        assert len(fixed_trace) == LENGTH
        for slot, token in enumerate(fixed_trace):
            formula.clause(events[slot][token])
    if seek_nested:
        formula.clause(
            *(
                states[boundary][nested]
                for boundary in range(1, LENGTH)
                for nested in (NESTED_P_H1, NESTED_P_P)
            )
        )
    else:
        for boundary in range(1, LENGTH):
            formula.clause(-states[boundary][NESTED_P_H1])
            formula.clause(-states[boundary][NESTED_P_P])
    return formula


def _solve(formula: CNF) -> str:
    bridge = Path(__file__).with_name("go_sat")
    go = os.environ.get("HWEB_GO", "go")
    result = subprocess.run(
        [go, "run", "."], cwd=bridge, input=formula.dimacs(), text=True,
        capture_output=True, check=True, timeout=30,
    )
    return result.stdout.strip().upper()


def run() -> None:
    legal = ("P", "H1", "/H1", "EOF")
    repeated_paragraphs = ("P", "P", "/P", "EOF")
    queries = (
        (
            "legal P/H1 trace",
            _model(mutant=None, fixed_trace=legal, seek_nested=False),
            "SAT",
        ),
        (
            "distinct repeated-P trace",
            _model(
                mutant=None,
                fixed_trace=repeated_paragraphs,
                seek_nested=False,
            ),
            "SAT",
        ),
        (
            "baseline nested-scope invariant",
            _model(mutant=None, fixed_trace=None, seek_nested=True),
            "UNSAT",
        ),
        (
            "P-to-H1 close-removal mutant",
            _model(
                mutant="P-to-H1 close removed",
                fixed_trace=legal,
                seek_nested=True,
            ),
            "SAT",
        ),
        (
            "P-to-P close-removal mutant",
            _model(
                mutant="P-to-P close removed",
                fixed_trace=repeated_paragraphs,
                seek_nested=True,
            ),
            "SAT",
        ),
    )
    for label, formula, expected in queries:
        actual = _solve(formula)
        if actual != expected:
            raise RuntimeError(
                f"{label}: solver returned {actual}, expected {expected}"
            )
    print(
        "Go CDCL bounded HWEB06 model: token alphabet {P, /P, H1, /H1, EOF}; "
        "exactly 4 tokens; EOF final"
    )
    print(
        "Assumptions: structural markers only; exact tags; one P/H1 block scope; "
        "invalid transitions rejected; final state root"
    )
    print(
        "Baseline legal traces: SAT for P, H1, /H1, EOF and P, P, /P, EOF "
        "(distinct repeated paragraphs)"
    )
    print(
        "Baseline invariant: UNSAT for any depth-2 P/H1 scope in the bounded "
        "transition relation"
    )
    print(
        "Negative control: SAT for P, H1, /H1, EOF when P-to-H1 close is "
        "removed (nested scope witness)"
    )
    print(
        "Negative control: SAT for P, P, /P, EOF when P-to-P close is removed "
        "(nested scope witness)"
    )
    print("This is bounded transition-model evidence, not source equivalence.")
    print(
        "Exhaustive real-parser replay is separate evidence and currently "
        "covers the Python source only."
    )


if __name__ == "__main__":
    run()
