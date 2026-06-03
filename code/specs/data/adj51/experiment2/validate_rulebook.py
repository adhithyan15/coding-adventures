#!/usr/bin/env python3
"""Byte-accounting validator for derived rulebook files.

The contract: every clause (prior / contributes / interacts) must
have a rationale block — one or more `% ...` comment lines
IMMEDIATELY preceding the clause head — explaining what the clause
is supposed to encode. The verifier subagent will later read each
(rationale, clause) pair and check that the clause actually
implements what the rationale says.

This is the structural half of the byte-accounting contract on
the rulebook, mirroring the byte-accounting contract on ingestion.

Usage:
  python3 validate_rulebook.py <rulebook.adj>

Exits 0 on PASS; exits 1 with diagnostics on FAIL.

Emits the (rationale, clause) pairs on PASS so the next stage
(semantic verifier) can consume them.
"""

import json
import re
import sys


CLAUSE_RE = re.compile(r"^\s*(prior|contributes|interacts)\b")
CONTINUATION_RE = re.compile(r"^\s+(source|trust|for|when|from|to|and)\b")
COMMENT_RE = re.compile(r"^\s*%")
BLANK_RE = re.compile(r"^\s*$")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: validate_rulebook.py <rulebook.adj>")
        return 2

    path = sys.argv[1]
    with open(path) as f:
        lines = f.readlines()

    errors: list[str] = []
    clauses: list[dict] = []
    pending_rationale: list[str] = []
    current_clause: dict | None = None
    line_dispositions: list[str] = []  # one tag per line

    line_no = 0
    while line_no < len(lines):
        line = lines[line_no]
        if BLANK_RE.match(line):
            if current_clause is not None:
                # blank line ends the clause
                clauses.append(current_clause)
                current_clause = None
            pending_rationale = []
            line_dispositions.append("blank")
            line_no += 1
            continue

        if COMMENT_RE.match(line):
            if current_clause is not None:
                clauses.append(current_clause)
                current_clause = None
            pending_rationale.append(line.rstrip("\n"))
            line_dispositions.append("comment")
            line_no += 1
            continue

        if CLAUSE_RE.match(line):
            if current_clause is not None:
                clauses.append(current_clause)
            if not pending_rationale:
                errors.append(
                    f"line {line_no + 1}: clause has no rationale block "
                    f"(no immediately preceding % comment lines): {line.strip()!r}"
                )
            current_clause = {
                "head_line": line_no + 1,
                "head": line.rstrip("\n"),
                "continuations": [],
                "rationale": list(pending_rationale),
            }
            pending_rationale = []
            line_dispositions.append("clause_head")
            line_no += 1
            continue

        if CONTINUATION_RE.match(line):
            if current_clause is None:
                errors.append(
                    f"line {line_no + 1}: continuation line with no preceding clause: {line.strip()!r}"
                )
                line_dispositions.append("orphan_continuation")
            else:
                current_clause["continuations"].append(line.rstrip("\n"))
                line_dispositions.append("clause_continuation")
            pending_rationale = []
            line_no += 1
            continue

        # unrecognized
        errors.append(
            f"line {line_no + 1}: unrecognized line type (not a clause, continuation, comment, or blank): {line.strip()!r}"
        )
        line_dispositions.append("unrecognized")
        line_no += 1

    if current_clause is not None:
        clauses.append(current_clause)

    n_clauses = len(clauses)
    n_with_rationale = sum(1 for c in clauses if c["rationale"])
    if n_clauses != n_with_rationale:
        errors.append(
            f"{n_clauses - n_with_rationale} clause(s) lack rationale blocks "
            f"({n_with_rationale}/{n_clauses} clauses are justified)."
        )

    counts = {}
    for d in line_dispositions:
        counts[d] = counts.get(d, 0) + 1

    if errors:
        print("FAIL")
        print(f"  total lines: {len(lines)}")
        print(f"  clauses found: {n_clauses}")
        print(f"  clauses with rationale: {n_with_rationale}")
        print(f"  line dispositions: {counts}")
        for e in errors:
            print(f"  - {e}")
        return 1

    print("PASS — every clause has a rationale block; every line dispositioned")
    print(f"  total lines: {len(lines)}")
    print(f"  clauses: {n_clauses} (priors/contributes/interacts)")
    print(f"  line dispositions: {counts}")
    print(f"  rationale/clause pairs: {n_clauses}")

    # Emit the (rationale, clause) pairs as a sidecar JSON for the next stage.
    pairs_path = path + ".pairs.json"
    pairs = []
    for i, c in enumerate(clauses):
        pairs.append({
            "clause_id": f"c{i + 1}",
            "head_line": c["head_line"],
            "rationale": "\n".join(c["rationale"]),
            "clause": "\n".join([c["head"]] + c["continuations"]),
        })
    with open(pairs_path, "w") as f:
        json.dump(pairs, f, indent=2)
    print(f"  pairs emitted to: {pairs_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
