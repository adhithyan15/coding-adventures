"""Bounded SAT model for the strict HWEB06 P/H1 block transitions.

The model deliberately contains structural tokens only; text and attributes are
outside this slice. It delegates DIMACS to the repository's Go CDCL solver.
"""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

TOKENS = ("P", "/P", "H1", "/H1", "EOF")
ROOT, PARAGRAPH, HEADING, NESTED = range(4)
STATES = (ROOT, PARAGRAPH, HEADING, NESTED)
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


def _model(*, mutant: bool, fixed_trace: tuple[str, ...] | None, seek_nested: bool) -> CNF:
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

    # The only successful transitions in this structural-token subset.
    transitions = {
        (ROOT, "P"): ROOT,  # P marker is empty in the historical tree profile.
        (ROOT, "H1"): HEADING,
        (ROOT, "EOF"): ROOT,
        (PARAGRAPH, "P"): ROOT,
        (PARAGRAPH, "/P"): ROOT,
        (PARAGRAPH, "H1"): NESTED if mutant else HEADING,
        (PARAGRAPH, "EOF"): ROOT,
        (HEADING, "/H1"): ROOT,
    }
    # The reduced model's P state represents an open P scope in HWEB06.
    # A P start from root opens that scope; a repeated P closes then reopens it.
    transitions[(ROOT, "P")] = PARAGRAPH
    transitions[(PARAGRAPH, "P")] = PARAGRAPH
    transitions[(PARAGRAPH, "/P")] = ROOT
    if mutant:
        transitions[(NESTED, "/H1")] = PARAGRAPH

    for slot in range(LENGTH):
        for state in STATES:
            for token in TOKENS:
                next_state = transitions.get((state, token))
                if next_state is None:
                    for candidate in STATES:
                        formula.clause(-states[slot][state], -events[slot][token], -states[slot + 1][candidate])
                else:
                    for candidate in STATES:
                        if candidate != next_state:
                            formula.clause(-states[slot][state], -events[slot][token], -states[slot + 1][candidate])
                    formula.clause(-states[slot][state], -events[slot][token], states[slot + 1][next_state])

    formula.clause(states[LENGTH][ROOT])
    if fixed_trace is not None:
        assert len(fixed_trace) == LENGTH
        for slot, token in enumerate(fixed_trace):
            formula.clause(events[slot][token])
    if seek_nested:
        formula.clause(*(states[boundary][NESTED] for boundary in range(1, LENGTH)))
    else:
        for boundary in range(1, LENGTH):
            formula.clause(-states[boundary][NESTED])
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
    assert _solve(_model(mutant=False, fixed_trace=legal, seek_nested=False)) == "SAT"
    assert _solve(_model(mutant=False, fixed_trace=repeated_paragraphs, seek_nested=False)) == "SAT"
    assert _solve(_model(mutant=False, fixed_trace=None, seek_nested=True)) == "UNSAT"
    assert _solve(_model(mutant=True, fixed_trace=legal, seek_nested=True)) == "SAT"
    print("Go CDCL bounded HWEB06 model: token alphabet {P, /P, H1, /H1, EOF}; exactly 4 tokens; EOF final")
    print("Assumptions: structural markers only; exact tags; one P/H1 block scope; invalid transitions rejected; final state root")
    print("Baseline legal traces: SAT for P, H1, /H1, EOF and P, P, /P, EOF (distinct repeated paragraphs)")
    print("Baseline invariant: UNSAT for any depth-2 P/H1 scope in the bounded transition relation")
    print("Negative control: SAT for P, H1, /H1, EOF when the P-to-H1 implied close is removed (nested scope witness)")
    print("This is bounded transition-model evidence, not source equivalence; exhaustive real-parser replay is separate evidence.")


if __name__ == "__main__":
    run()
