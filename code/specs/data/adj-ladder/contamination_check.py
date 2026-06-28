#!/usr/bin/env python3
"""contamination_check.py — bank-integrity & anti-circularity gate for a ladder rung.

A two-arm proof is only worth anything if the question bank is honest. This gate runs
*offline* over a rung's items.json and asserts the structural properties that keep the
result legible. It is QA on the BANK — it is NOT on the answer path (the engine still
does every bit of the actual reasoning at eval time).

Checks performed (self-authored starter rungs):

  1. Unique ids                — no item counted twice.
  2. Five distinct options     — each item has options A..E with DISTINCT values, so
                                 the engine's `answer == value` match is unambiguous
                                 (a duplicate value would make a correct compute tie
                                 and abstain — a measurement artifact, not a miss).
  3. Gold points at an option  — gold_letter ∈ options.
  4. Gold is internally correct— formula items use a restricted, safe arithmetic eval
                                 (digits and + - * / () only). Program items run the
                                 native ADJ CLI and check that its solved value maps
                                 to gold. This is bank QA, never the answer path.
  5. No-result-literals        — every number in `formula` or `program` also appears
                                 in `stem`, so the gold decomposition itself never
                                 smuggles the answer in (the same gate Arm B applies
                                 to the model).
  6. No external provenance    — these starter rungs are self-contained: items carry
                                 no `source` / library import, so contamination
                                 against an external bank is structurally impossible
                                 here. (Higher sourced rungs add a source-disjointness
                                 check.)

Exit non-zero on any violation. Usage: python3 contamination_check.py rung0_arithmetic
"""

from __future__ import annotations

import ast
import json
import operator
import re
import sys
from pathlib import Path

import ladder_eval as le

HERE = Path(__file__).resolve().parent
_NUM = re.compile(r"\d+(?:\.\d+)?")

# A tiny, safe arithmetic evaluator (NOT Python's eval) — supports only +, -, *, /,
# unary minus, parentheses, and numeric literals. Anything else raises, so a malformed
# or sneaky formula can never execute arbitrary code.
_BINOPS = {ast.Add: operator.add, ast.Sub: operator.sub,
           ast.Mult: operator.mul, ast.Div: operator.truediv}
_UNOPS = {ast.UAdd: operator.pos, ast.USub: operator.neg}


def safe_eval(expr: str) -> float:
    def ev(node):
        if isinstance(node, ast.Expression):
            return ev(node.body)
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
            return node.value
        if isinstance(node, ast.BinOp) and type(node.op) in _BINOPS:
            return _BINOPS[type(node.op)](ev(node.left), ev(node.right))
        if isinstance(node, ast.UnaryOp) and type(node.op) in _UNOPS:
            return _UNOPS[type(node.op)](ev(node.operand))
        raise ValueError(f"disallowed expression element: {ast.dump(node)}")
    return ev(ast.parse(expr, mode="eval"))


def option_value(value) -> float:
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        return float(safe_eval(value))
    raise ValueError(f"unsupported option value {value!r}")


def option_signature(value):
    if isinstance(value, list):
        if not value:
            raise ValueError("root-set option must not be empty")
        roots = []
        for root in value:
            if not isinstance(root, (int, float)):
                raise ValueError(f"unsupported root value {root!r}")
            roots.append(round(float(root), 9))
        return ("roots", tuple(sorted(roots)))
    return ("number", round(option_value(value), 9))


def label_option_signature(value):
    if not isinstance(value, str):
        raise ValueError(f"unsupported label option value {value!r}")
    label = value.strip().lower()
    if not label:
        raise ValueError("label option must not be empty")
    return ("label", label)


def check(rung: str) -> list[str]:
    items = json.loads((HERE / rung / "items.json").read_text())["items"]
    errors: list[str] = []
    seen: set[str] = set()
    for it in items:
        iid = it.get("id", "<no-id>")
        if iid in seen:
            errors.append(f"{iid}: duplicate id")
        seen.add(iid)

        opts = it.get("options", {})
        if sorted(opts) != list("ABCDE"):
            errors.append(f"{iid}: options must be exactly A..E, got {sorted(opts)}")
        numeric_opts: dict[str, float] = {}
        option_keys = {}
        answer_type = (it.get("answer_from") or {}).get("type")
        label_options = "program" in it and answer_type == "check_outcome"
        for ltr, value in opts.items():
            try:
                if label_options:
                    option_keys[ltr] = label_option_signature(value)
                else:
                    option_keys[ltr] = option_signature(value)
                if option_keys[ltr][0] == "number":
                    numeric_opts[ltr] = option_keys[ltr][1]
            except (ValueError, SyntaxError, ZeroDivisionError) as e:
                errors.append(f"{iid}: option {ltr} value {value!r} did not evaluate: {e}")
        duplicates = [
            (a, b)
            for i, (a, av) in enumerate(option_keys.items())
            for b, bv in list(option_keys.items())[i + 1 :]
            if av == bv
        ]
        if duplicates:
            errors.append(f"{iid}: option values must be distinct, got duplicates {duplicates}")

        gold = it.get("gold_letter")
        if gold not in opts:
            errors.append(f"{iid}: gold_letter {gold!r} not among options")
            continue

        if "formula" in it:
            try:
                computed = safe_eval(it["formula"])
            except (ValueError, SyntaxError, ZeroDivisionError) as e:
                errors.append(f"{iid}: formula {it['formula']!r} did not evaluate: {e}")
                continue
            if gold in numeric_opts and abs(computed - numeric_opts[gold]) > 1e-9:
                errors.append(f"{iid}: gold {gold}={opts[gold]} ≠ formula value {computed}")
            decomposition = it["formula"]
            program_decomposition = False
        elif "program" in it:
            if not isinstance(it.get("answer_from"), dict):
                errors.append(f"{iid}: program items must declare answer_from")
            doc = le.run_program(it["program"])
            if doc is not None:
                letter = le.program_answer_to_letter(doc, it.get("answer_from"), opts)
                if letter != gold:
                    errors.append(
                        f"{iid}: gold {gold}={opts[gold]} ≠ ADJ program selection {letter}"
                    )
            decomposition = it["program"]
            program_decomposition = True
        else:
            errors.append(f"{iid}: item must include either formula or program")
            decomposition = ""
            program_decomposition = False

        stem_nums = set(_NUM.findall(it.get("stem", "")))
        leaked = [
            n for n in le.decomposition_numbers(decomposition, program=program_decomposition)
            if n not in stem_nums
        ]
        if leaked:
            errors.append(
                f"{iid}: decomposition numbers {leaked} not present in stem (result-literal leak)"
            )

        if "source" in it or "import" in it:
            errors.append(f"{iid}: starter rung items must be self-contained (no external source/import)")
    return errors


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: contamination_check.py <rung-dir>", file=sys.stderr)
        return 2
    rung = argv[0]
    errors = check(rung)
    if errors:
        print(f"contamination_check {rung}: {len(errors)} violation(s)")
        for e in errors:
            print(f"  ✗ {e}")
        return 1
    n = len(json.loads((HERE / rung / "items.json").read_text())["items"])
    print(f"contamination_check {rung}: ✓ {n} items clean "
          "(unique, distinct options, gold matches decomposition, no result-literal leak, self-contained)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
