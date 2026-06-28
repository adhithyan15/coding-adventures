#!/usr/bin/env python3
"""ladder_eval.py — the ADJ-LADDER two-arm scoreboard.

THE BIG IDEA (read this first). We want one falsifiable, externally-legible proof
that *reasoning and math live in the framework, not the weights*. So we climb a
**complexity ladder** — grade-school arithmetic → fractions → algebra → calculus →
physics/units → clinical → MLE — and at every rung we run the SAME question set
through TWO arms:

    Arm A — the small model ALONE.   It reads the multiple-choice question and
            picks a letter. Whatever arithmetic the question needs, the model does
            in its head (this is exactly what language models are bad at).

    Arm B — the small model + the ADJ engine.   The model only DECOMPOSES the
            question into an ADJ expression (ASCII arithmetic or ADJ's native
            `latex "..."` form over the numbers that appear in the stem); the
            **engine** does every bit of arithmetic on the CPU, exactly, and SELECTS
            the option whose value equals the computed answer — emitting a
            machine-checkable proof.

The headline number is the **divergence, B − A**. At rung 0 the gap is small (a
small model can do `7 * 8 + 3`). As the ladder climbs and the computation deepens,
Arm A degrades while Arm B stays pinned near 100% — because the engine never makes
an arithmetic slip. That widening gap is the money curve.

----------------------------------------------------------------------------------
HOW ARM B SELECTS AN OPTION WITHOUT EVER COMPUTING THE ANSWER ITSELF
----------------------------------------------------------------------------------
For a question whose options are {A:59, B:60, C:61, D:58, E:62} and whose gold
decomposition is the formula `7 * 8 + 3`, Arm B builds this ADJ program:

    prior 0.0001 for opt_a            % five equal-prior hypotheses, one per option
    prior 0.0001 for opt_b
    prior 0.0001 for opt_c
    prior 0.0001 for opt_d
    prior 0.0001 for opt_e
    let answer = 7 * 8 + 3            % the ENGINE computes this — Python never does
    contributes 1000000 from answer == 59 to opt_a   % option VALUES come from the
    contributes 1000000 from answer == 60 to opt_b   % question, not from us solving
    contributes 1000000 from answer == 61 to opt_c
    contributes 1000000 from answer == 58 to opt_d
    contributes 1000000 from answer == 62 to opt_e
    ? opt_a … ? opt_e

The engine evaluates `answer` (= 59), the predicate `answer == 59` fires, opt_a's
log-odds jump by a huge amount, and the decision comes back `determinate` with
`leader = opt_a` → letter **A**. If the computed answer matches NO option (or, by a
duplicate-value accident, two), the hypotheses stay tied and the decision is a
`kickback` → we **ABSTAIN** rather than guess. Crucially, this harness supplies
only (i) the formula and (ii) the option values printed in the question; it never
evaluates the formula. The arithmetic, the comparison, and the selection are all
the engine's. (Verified shapes: a determinate decision names a `leader`; a tie
returns `kickback`. See proration.adj for the `let`+`contributes` pattern.)

----------------------------------------------------------------------------------
SCORING (reused from board_eval.py — three outcomes, never-fabricate gate)
----------------------------------------------------------------------------------
Every item resolves to exactly one of:

    correct   — chose the gold letter
    abstained — declined to commit (kickback / no model letter) — the HONEST miss
    wrong     — committed to the wrong letter (the only real failure)

and we report `defensibility = (correct + abstained) / total`,
`accuracy_on_attempted = correct / (correct + wrong)`, plus the cross-arm
**divergence**. Arm B is GATED: if the engine ever computes a value and selects a
*wrong* option (`wrong > 0` in cached mode), the run exits non-zero — the engine's
arithmetic must be exact, by construction.

----------------------------------------------------------------------------------
MODES
----------------------------------------------------------------------------------
    --mode cached   (default)  Arm B only, using each item's GOLD `formula` as the
                               decomposition. This isolates the ENGINE: it should
                               score ~100% correct, proving the mechanism with no
                               model in the loop. This is what CI runs.

    --model <spec>             Run BOTH arms with a real local model. The model
                               answers directly (Arm A) and produces the formula
                               decomposition (Arm B). Emits the two-arm divergence.
                               <spec> is one of:
                                 mlx:<hf-repo>        load via mlx_lm (Apple silicon)
                                 cmd:<shell-command>  prompt on stdin → text on stdout
                               A model-produced formula must pass the
                               **no-result-literals** gate (every number in the
                               formula appears in the stem) or that item abstains in
                               Arm B as a decompose-error — the model may write the
                               recipe, never the answer.

Usage:
    python3 ladder_eval.py rung0_arithmetic                      # cached, pretty
    python3 ladder_eval.py rung0_arithmetic --quiet              # scorecard JSON only
    python3 ladder_eval.py rung0_arithmetic --model mlx:mlx-community/Qwen2.5-0.5B-Instruct-4bit
"""

from __future__ import annotations

import argparse
import ast
import json
import operator
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Union

HERE = Path(__file__).resolve().parent

# The native adj-lang CLI — the ONE engine that does all of Arm B's math. We look in
# a few standard locations (worktree vs main-repo target, debug vs release) and allow
# an explicit override, so the harness runs whether you built in this checkout or a
# shared one. If none is found, Arm B cannot run and every engine item abstains
# honestly (no Python fallback — computing the answer in Python is exactly what this
# whole exercise refuses to do).
def _find_cli() -> Path | None:
    override = os.environ.get("ADJ_LANG_CLI")
    if override and Path(override).exists():
        return Path(override)
    rust = HERE.parents[2] / "packages" / "rust"        # adj-ladder → data → specs → code
    candidates = [
        rust / "target" / "debug" / "adj-lang-cli",
        rust / "target" / "release" / "adj-lang-cli",
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


_CLI = _find_cli()

# A "number" anywhere in a stem or formula: an optional sign is intentionally NOT
# matched (signs are operators in the formula), just integer/decimal magnitudes.
_NUM = re.compile(r"\d+(?:\.\d+)?")
# A single answer letter the model may emit for Arm A, e.g. "Answer: C" or "(b)".
_LETTER = re.compile(r"\b([A-E])\b")


# ----------------------------------------------------------------------------------
# Arm B — build the ADJ program and read the engine's selection.
# ----------------------------------------------------------------------------------
OptionValue = Union[int, float, str, list[Union[int, float]]]

_BINOPS = {
    ast.Add: operator.add,
    ast.Sub: operator.sub,
    ast.Mult: operator.mul,
    ast.Div: operator.truediv,
}
_UNOPS = {ast.UAdd: operator.pos, ast.USub: operator.neg}


def _render_option_expr(value: OptionValue) -> str:
    if isinstance(value, list):
        raise ValueError("root-set options are supported only by solve_roots items")
    if isinstance(value, str):
        value = value.strip()
        if not value:
            raise ValueError("option expression must not be empty")
        return value
    v = float(value)
    return str(int(v)) if v.is_integer() else str(value)


def _safe_number_expr(expr: str) -> float:
    def ev(node):
        if isinstance(node, ast.Expression):
            return ev(node.body)
        if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
            return float(node.value)
        if isinstance(node, ast.BinOp) and type(node.op) in _BINOPS:
            return _BINOPS[type(node.op)](ev(node.left), ev(node.right))
        if isinstance(node, ast.UnaryOp) and type(node.op) in _UNOPS:
            return _UNOPS[type(node.op)](ev(node.operand))
        raise ValueError(f"disallowed option expression element: {ast.dump(node)}")

    return ev(ast.parse(expr, mode="eval"))


def _option_number(value: OptionValue) -> float:
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        return _safe_number_expr(value)
    raise ValueError(f"unsupported option value {value!r}")


def _option_label(value: OptionValue) -> str:
    if not isinstance(value, str):
        raise ValueError(f"unsupported label option value {value!r}")
    label = value.strip().lower()
    if not label:
        raise ValueError("label option must not be empty")
    return label


def _option_roots(value: OptionValue) -> tuple[float, ...]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"unsupported root-set option value {value!r}")
    roots = []
    for root in value:
        if not isinstance(root, (int, float)):
            raise ValueError(f"unsupported root value {root!r}")
        roots.append(float(root))
    return tuple(sorted(roots))


def build_arm_b_program(formula: str, options: dict[str, OptionValue]) -> str:
    """Render the option-selection ADJ program described in the module docstring.

    `options` maps letters (A..E) to their numeric value or ADJ arithmetic expression
    as printed in the question. We declare one equal-prior hypothesis per option and
    one `contributes` predicate that fires when the engine-computed `answer` equals
    that option's value. The huge likelihood ratio (1e6) makes a single matching
    option dominate decisively; if none match, the hypotheses stay tied and the
    engine returns a kickback."""
    lines = [f"prior 0.0001 for opt_{ltr.lower()}" for ltr in options]
    lines.append(f"let answer = {formula}")
    for ltr, val in options.items():
        # Render whole-valued floats as ints so the predicate reads cleanly (59 not
        # 59.0). String values are already ADJ expressions such as `3 / 10`.
        lines.append(
            f"contributes 1000000 from answer == {_render_option_expr(val)} to opt_{ltr.lower()}"
        )
    lines += [f"? opt_{ltr.lower()}" for ltr in options]
    return "\n".join(lines) + "\n"


def run_program(program: str) -> dict | None:
    """Write a native ADJ program to a temp .adj and run the CLI.

    Formula rungs inspect only the `decision`; solve-backed rungs inspect the `solve`
    section. In both cases the native CLI is the only component that executes the
    math/reasoning program."""
    if _CLI is None:
        return None
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".ladder_")
    try:
        os.write(fd, program.encode("utf-8"))
        os.close(fd)
        out = subprocess.run([str(_CLI), path], capture_output=True, text=True)
        doc = json.loads(out.stdout)
    except (json.JSONDecodeError, ValueError, OSError):
        return None
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass
    return doc if isinstance(doc, dict) else None


def run_decision(program: str) -> dict | None:
    """Run a program and return its `decision` dict, or None on compile/runtime miss."""
    doc = run_program(program)
    return doc.get("decision") if isinstance(doc, dict) else None


def decision_to_letter(decision: dict | None) -> str | None:
    """Map a decision to an answer letter, or None to ABSTAIN.

    A `determinate` decision names a winning hypothesis `opt_x` → letter X. Any other
    decision kind (kickback on a tie, empty on no hypotheses, or a missing decision
    when the CLI is absent / failed to compile) is an honest abstention."""
    if not decision or decision.get("type") != "determinate":
        return None
    leader = decision.get("leader") or ""
    if leader.startswith("opt_") and len(leader) == 5:
        return leader[-1].upper()
    return None


def _letter_for_engine_value(value: float, options: dict[str, OptionValue]) -> str | None:
    matches = []
    for ltr, option in options.items():
        try:
            if abs(float(value) - _option_number(option)) <= 1e-9:
                matches.append(ltr)
        except (ValueError, SyntaxError, ZeroDivisionError):
            return None
    return matches[0] if len(matches) == 1 else None


def _letter_for_engine_roots(roots: list[float], options: dict[str, OptionValue]) -> str | None:
    root_set = tuple(sorted(float(r) for r in roots))
    matches = []
    for ltr, option in options.items():
        try:
            option_roots = _option_roots(option)
        except (TypeError, ValueError):
            return None
        if len(root_set) == len(option_roots) and all(
            abs(a - b) <= 1e-9 for a, b in zip(root_set, option_roots)
        ):
            matches.append(ltr)
    return matches[0] if len(matches) == 1 else None


def _letter_for_engine_label(label: str, options: dict[str, OptionValue]) -> str | None:
    label = label.strip().lower()
    matches = []
    for ltr, option in options.items():
        try:
            if _option_label(option) == label:
                matches.append(ltr)
        except ValueError:
            return None
    return matches[0] if len(matches) == 1 else None


_PROGRAM_WEIGHT_LINE = re.compile(
    r"^(\s*(?:prior|contributes|interacts)\s+)\d+(?:\.\d+)?"
)


def decomposition_numbers(
    decomposition: str, *, program: bool = False, structural_weights: bool = True
) -> list[str]:
    """Return numeric literals that must be grounded in the stem.

    Program-backed rungs may include structural confidence weights such as
    `prior 0.001` or `contributes 1000000 ...`; those are not math facts from the
    word problem. Strip just that leading weight and keep every number in observed
    facts, constraints, and predicate thresholds under the no-result-literals gate.
    """
    if not program or not structural_weights:
        return _NUM.findall(decomposition)
    nums: list[str] = []
    for raw in decomposition.splitlines():
        line = _PROGRAM_WEIGHT_LINE.sub(r"\1", raw)
        nums.extend(_NUM.findall(line))
    return nums


def program_requirements_hold(doc: dict | None, answer_from: dict | None) -> bool:
    """Check optional native-program requirements beyond the solved value.

    The first mixed reasoning rung requires a rule-derived premise to fire a
    queried decision before the numeric solve is accepted. This keeps Python out of
    the reasoning path: it only inspects the CLI JSON for a determinate leader and,
    when requested, the evidence proof ADJ produced for that leader.
    """
    if not answer_from:
        return True
    requirements = answer_from.get("requires") or []
    if not requirements:
        return True
    if not doc:
        return False
    for req in requirements:
        if req.get("type") != "decision":
            return False
        decision = doc.get("decision")
        leader = req.get("leader")
        if not leader or not isinstance(decision, dict):
            return False
        if decision.get("type") != "determinate" or decision.get("leader") != leader:
            return False
        evidence = req.get("evidence")
        if evidence:
            ranked = doc.get("ranked") or []
            proof_steps = []
            for entry in ranked:
                if isinstance(entry, dict) and entry.get("hypothesis") == leader:
                    proof_steps = entry.get("proof") or []
                    break
            matched = [
                step for step in proof_steps
                if isinstance(step, dict)
                and step.get("kind") == "contribution"
                and step.get("evidence") == evidence
                and step.get("evidence_proof")
            ]
            if not matched:
                return False
    return True


def solve_assignment_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ solver assignment to an option letter.

    A rung-2 item can carry a full ADJ program such as `symbol x; constrain ...;
    solve for { x }`. The engine returns the solved value; this helper only performs
    option lookup against the printed choices, preserving the invariant that Python
    never solves the equation."""
    if not doc or not answer_from or answer_from.get("type") != "solve_assignment":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    name = answer_from.get("name")
    solve = doc.get("solve")
    if not name or not isinstance(solve, dict) or solve.get("outcome") != "solved":
        return None
    assignments = solve.get("assignments") or []
    values = [a.get("value") for a in assignments if isinstance(a, dict) and a.get("name") == name]
    if len(values) != 1:
        return None
    try:
        return _letter_for_engine_value(float(values[0]), options)
    except (TypeError, ValueError):
        return None


def solve_roots_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ root solve to an option letter.

    The engine owns the nonlinear solve and returns the real roots; the harness
    only compares that returned root set to the printed multiple-choice root sets.
    """
    if not doc or not answer_from or answer_from.get("type") != "solve_roots":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    name = answer_from.get("name")
    solve = doc.get("solve")
    if not name or not isinstance(solve, dict) or solve.get("outcome") != "solved_roots":
        return None
    if solve.get("var") != name:
        return None
    roots = solve.get("roots")
    if not isinstance(roots, list):
        return None
    try:
        return _letter_for_engine_roots([float(r) for r in roots], options)
    except (TypeError, ValueError):
        return None


def optimize_value_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ optimization optimum to an option letter.

    Linear-programming rungs ask the model to emit only constraints plus a
    `maximize`/`minimize` objective. ADJ owns the optimization; the harness only
    compares `optimize.value` against the printed choices.
    """
    if not doc or not answer_from or answer_from.get("type") != "optimize_value":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    optimize = doc.get("optimize")
    if not isinstance(optimize, dict) or optimize.get("outcome") != "optimal":
        return None
    try:
        return _letter_for_engine_value(float(optimize.get("value")), options)
    except (TypeError, ValueError):
        return None


def optimize_assignment_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ optimization witness assignment to an option letter.

    This is the assignment analogue of `optimize_value`: ADJ optimizes the linear
    program and emits a witness; the harness only selects the printed option that
    equals the requested variable's witness value.
    """
    if not doc or not answer_from or answer_from.get("type") != "optimize_assignment":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    name = answer_from.get("name")
    optimize = doc.get("optimize")
    if not name or not isinstance(optimize, dict) or optimize.get("outcome") != "optimal":
        return None
    assignments = optimize.get("assignments") or []
    values = [a.get("value") for a in assignments if isinstance(a, dict) and a.get("name") == name]
    if len(values) != 1:
        return None
    try:
        return _letter_for_engine_value(float(values[0]), options)
    except (TypeError, ValueError):
        return None


def check_outcome_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ feasibility verdict to an option letter.

    Constraint-feasibility rungs ask the model to emit only variables,
    constraints, and `check`. ADJ decides whether the system is feasible over the
    supported domain; the harness only maps `check.outcome` to printed categorical
    choices such as "feasible" or "infeasible".
    """
    if not doc or not answer_from or answer_from.get("type") != "check_outcome":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    check = doc.get("check")
    if not isinstance(check, dict):
        return None
    outcome = check.get("outcome")
    labels = {
        "sat": "feasible",
        "sat_real": "feasible",
        "unsat": "infeasible",
        "unknown": "unknown",
    }
    labels.update(answer_from.get("labels") or {})
    label = labels.get(outcome)
    if not isinstance(label, str):
        return None
    return _letter_for_engine_label(label, options)


def decision_leader_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    """Map a native ADJ probabilistic decision leader to an option letter.

    Probability-decision rungs ask the model to emit priors, likelihood-ratio
    contributions, observations, and queries. ADJ owns the posterior ranking; the
    harness only maps `decision.leader` to the printed categorical choices.
    """
    if not doc or not answer_from or answer_from.get("type") != "decision_leader":
        return None
    if not program_requirements_hold(doc, answer_from):
        return None
    decision = doc.get("decision")
    if not isinstance(decision, dict) or decision.get("type") != "determinate":
        return None
    leader = decision.get("leader")
    if not isinstance(leader, str):
        return None
    labels = answer_from.get("labels") or {}
    label = labels.get(leader, leader)
    if not isinstance(label, str):
        return None
    return _letter_for_engine_label(label, options)


def program_answer_to_letter(
    doc: dict | None, answer_from: dict | None, options: dict[str, OptionValue]
) -> str | None:
    if not answer_from:
        return None
    if answer_from.get("type") == "solve_assignment":
        return solve_assignment_to_letter(doc, answer_from, options)
    if answer_from.get("type") == "solve_roots":
        return solve_roots_to_letter(doc, answer_from, options)
    if answer_from.get("type") == "optimize_value":
        return optimize_value_to_letter(doc, answer_from, options)
    if answer_from.get("type") == "optimize_assignment":
        return optimize_assignment_to_letter(doc, answer_from, options)
    if answer_from.get("type") == "check_outcome":
        return check_outcome_to_letter(doc, answer_from, options)
    if answer_from.get("type") == "decision_leader":
        return decision_leader_to_letter(doc, answer_from, options)
    return None


def formula_is_faithful(
    formula: str, stem: str, *, program: bool = False, structural_weights: bool = True
) -> bool:
    """The no-result-literals gate: every number in the formula must also appear in
    the stem. This is what stops a model (in --model mode) from smuggling the answer
    into the "decomposition" — it may write `7 * 8 + 3` (all from the stem) but not
    `59`. The gold formulae in items.json are authored to satisfy this too; the test
    suite re-checks them, so the gate is exercised even in cached mode."""
    stem_nums = set(_NUM.findall(stem))
    return all(
        tok in stem_nums
        for tok in decomposition_numbers(
            formula, program=program, structural_weights=structural_weights
        )
    )


# ----------------------------------------------------------------------------------
# Arm A — the model alone.
# ----------------------------------------------------------------------------------
def arm_a_prompt(item: dict) -> str:
    opts = "\n".join(f"{ltr}. {val}" for ltr, val in item["options"].items())
    return (
        "Answer this multiple-choice question. Reply with ONLY the single letter "
        "(A, B, C, D, or E) of the correct option.\n\n"
        f"{item['stem']}\n{opts}\n\nAnswer:"
    )


def parse_letter(text: str) -> str | None:
    """Pull the first standalone A-E out of a model's reply, or None if it produced
    nothing usable (→ abstain, never a fabricated guess on the harness's behalf)."""
    m = _LETTER.search((text or "").strip().upper())
    return m.group(1) if m else None


# ----------------------------------------------------------------------------------
# Scoring — three outcomes + per-arm metrics + cross-arm divergence.
# ----------------------------------------------------------------------------------
@dataclass
class ItemResult:
    item_id: str
    gold: str
    arm_a: str | None          # chosen letter, or None on abstain
    arm_a_outcome: str         # correct | abstained | wrong
    arm_b: str | None
    arm_b_outcome: str
    bucket: str | None         # Arm B failure bucket (see classify_bucket), else None


def outcome(letter: str | None, gold: str) -> str:
    if letter is None:
        return "abstained"
    return "correct" if letter == gold else "wrong"


def classify_bucket(arm_b_outcome: str, faithful: bool) -> str | None:
    """Two-factor diagnostic for Arm B misses (per MLE-PASS §2.5), so we can tell a
    framework gap from a decompose error:

        b — decompose-error   the model's formula failed the faithfulness gate
        c — engine-gap        a faithful formula still produced a wrong/abstained
                              selection (the engine couldn't express/compute it)

    (Buckets a "missing-lib" and d "genuinely-hard" appear at higher rungs.) A
    `correct` Arm B result has no bucket."""
    if arm_b_outcome == "correct":
        return None
    return "b" if not faithful else "c"


@dataclass
class Scorecard:
    rung: str
    mode: str
    model: str | None = None
    results: list[ItemResult] = field(default_factory=list)

    def _arm_summary(self, arm: str) -> dict:
        outs = [getattr(r, f"{arm}_outcome") for r in self.results]
        total = len(outs)
        correct = outs.count("correct")
        wrong = outs.count("wrong")
        abstained = outs.count("abstained")
        attempted = correct + wrong
        return {
            "total": total,
            "correct": correct,
            "abstained": abstained,
            "wrong": wrong,
            "raw_accuracy": round(correct / total, 4) if total else 0.0,
            "defensibility": round((correct + abstained) / total, 4) if total else 0.0,
            "accuracy_on_attempted": round(correct / attempted, 4) if attempted else None,
        }

    def summary(self) -> dict:
        a = self._arm_summary("arm_a")
        b = self._arm_summary("arm_b")
        buckets: dict[str, int] = {}
        for r in self.results:
            if r.bucket:
                buckets[r.bucket] = buckets.get(r.bucket, 0) + 1
        return {
            "rung": self.rung,
            "mode": self.mode,
            "model": self.model,
            "arm_a_model_alone": a,
            "arm_b_model_plus_adj": b,
            # The money number: how much the engine arm out-scores the model alone.
            "divergence": {
                "raw_accuracy": round(b["raw_accuracy"] - a["raw_accuracy"], 4),
                "correct": b["correct"] - a["correct"],
            },
            "arm_b_failure_buckets": buckets,
        }


def score_item(item: dict, gen) -> ItemResult:
    """Run both arms for one item. `gen` is a callable prompt→text for the model, or
    None in cached mode (Arm A is skipped → abstained; Arm B uses the gold formula)."""
    gold = item["gold_letter"]

    # --- Arm B: decompose → engine selects ---
    if gen is None:
        formula = item.get("formula")
        program = item.get("program")
        faithful = True                                     # cached: trust the gold
    else:
        raw = gen(decompose_prompt(item))
        if "program" in item:
            formula = None
            program = extract_program(raw)
            answer_from = item.get("answer_from") or {}
            faithful = bool(program) and formula_is_faithful(
                program,
                item["stem"],
                program=True,
                structural_weights=answer_from.get("structural_weights", True),
            )
        else:
            program = None
            formula = extract_formula(raw)
            faithful = bool(formula) and formula_is_faithful(formula, item["stem"])
    if not faithful:
        arm_b_letter = None                                  # decompose failed → abstain
    elif program:
        arm_b_letter = program_answer_to_letter(
            run_program(program), item.get("answer_from"), item["options"]
        )
    elif formula:
        arm_b_letter = decision_to_letter(
            run_decision(build_arm_b_program(formula, item["options"]))
        )
    else:
        arm_b_letter = None
    arm_b_out = outcome(arm_b_letter, gold)

    # --- Arm A: model alone (skipped in cached mode) ---
    if gen is None:
        arm_a_letter, arm_a_out = None, "abstained"
    else:
        arm_a_letter = parse_letter(gen(arm_a_prompt(item)))
        arm_a_out = outcome(arm_a_letter, gold)

    return ItemResult(
        item["id"], gold, arm_a_letter, arm_a_out, arm_b_letter, arm_b_out,
        classify_bucket(arm_b_out, faithful),
    )


# ----------------------------------------------------------------------------------
# Model decomposition (only used in --model mode).
# ----------------------------------------------------------------------------------
def decompose_prompt(item: dict) -> str:
    if "program" in item:
        answer_from = item.get("answer_from") or {}
        name = answer_from.get("name", "x")
        if answer_from.get("type") == "decision_leader":
            requires = answer_from.get("requires") or []
            decision_req = next((r for r in requires if r.get("type") == "decision"), None)
            if decision_req:
                leader = decision_req.get("leader", "tuberculosis")
                evidence = decision_req.get("evidence", "tb_pattern")
                return (
                    "Translate the word problem into a native ADJ derived-evidence "
                    "probability decision program. Use observe statements for the "
                    "stated findings, derive the requested intermediate evidence "
                    "with `rule { head: ... when: ... }`, add every stated prior and "
                    "likelihood-ratio contribution, then query every candidate with "
                    "`?`. Use ONLY numbers and labels that appear in the question. "
                    "Do NOT choose the answer, do NOT mention the answer choices, "
                    "and output ONLY the ADJ program.\n\n"
                    "Question: Two diagnoses start with prior 0.05 for tuberculosis "
                    "and 0.25 for bronchitis. Findings are prolonged_cough and "
                    "night_sweats. Those findings derive tb_pattern. Tb_pattern has "
                    "likelihood ratio 25 for tuberculosis and 0.5 for bronchitis. "
                    "Which diagnosis leads?\n"
                    "Program:\n"
                    "prior 0.05 for tuberculosis\n"
                    "prior 0.25 for bronchitis\n"
                    "contributes 25 from tb_pattern to tuberculosis\n"
                    "contributes 0.5 from tb_pattern to bronchitis\n"
                    "observe prolonged_cough\n"
                    "observe night_sweats\n"
                    "rule { head: tb_pattern when: prolonged_cough, night_sweats }\n"
                    "? tuberculosis\n"
                    "? bronchitis\n\n"
                    "Question: Two causes start with prior 0.10 for appendicitis "
                    "and 0.20 for gastroenteritis. Findings are rlq_pain and "
                    "rebound_tenderness. Those findings derive peritoneal_pattern. "
                    "Peritoneal_pattern has likelihood ratio 18 for appendicitis and "
                    "0.8 for gastroenteritis. Which cause leads?\n"
                    "Program:\n"
                    "prior 0.10 for appendicitis\n"
                    "prior 0.20 for gastroenteritis\n"
                    "contributes 18 from peritoneal_pattern to appendicitis\n"
                    "contributes 0.8 from peritoneal_pattern to gastroenteritis\n"
                    "observe rlq_pain\n"
                    "observe rebound_tenderness\n"
                    "rule { head: peritoneal_pattern when: rlq_pain, rebound_tenderness }\n"
                    "? appendicitis\n"
                    "? gastroenteritis\n\n"
                    f"Required decision leader: {leader}\n"
                    f"Required derived evidence: {evidence}\n"
                    f"Question: {item['stem']}\nProgram:"
                )
            return (
                "Translate the word problem into a native ADJ probability decision "
                "program. Declare the priors for the candidate hypotheses, add every "
                "stated likelihood-ratio contribution, observe the stated evidence, "
                "then query every candidate with `?`. Use ONLY numbers and labels "
                "that appear in the question. Do NOT choose the answer, do NOT "
                "mention the answer choices, and output ONLY the ADJ program.\n\n"
                "Question: Two diagnoses start with prior 0.30 each: bacterial and "
                "viral. Observed evidence is csf(neutrophilic). That evidence has "
                "likelihood ratio 15 for bacterial and 1.2 for viral. Which diagnosis "
                "leads?\n"
                "Program:\n"
                "prior 0.30 for bacterial\n"
                "prior 0.30 for viral\n"
                "contributes 15 from csf(neutrophilic) to bacterial\n"
                "contributes 1.2 from csf(neutrophilic) to viral\n"
                "observe csf(neutrophilic)\n"
                "? bacterial\n"
                "? viral\n\n"
                "Question: Two causes start with prior 0.20 each: asthma and panic. "
                "Observed evidence is wheeze and inhaler_response. Wheeze has "
                "likelihood ratio 10 for asthma and 0.8 for panic. Inhaler_response "
                "has likelihood ratio 6 for asthma and 0.7 for panic. Which cause "
                "leads?\n"
                "Program:\n"
                "prior 0.20 for asthma\n"
                "prior 0.20 for panic\n"
                "contributes 10 from wheeze to asthma\n"
                "contributes 0.8 from wheeze to panic\n"
                "contributes 6 from inhaler_response to asthma\n"
                "contributes 0.7 from inhaler_response to panic\n"
                "observe wheeze\n"
                "observe inhaler_response\n"
                "? asthma\n"
                "? panic\n\n"
                f"Question: {item['stem']}\nProgram:"
            )
        if answer_from.get("type") == "check_outcome":
            return (
                "Translate the word problem into a native ADJ constraint "
                "feasibility program. Declare the variables, add every stated "
                "constraint, then end with `check`. Use ONLY numbers that appear "
                "in the question. Do NOT decide feasibility, do NOT mention the "
                "answer choices, and output ONLY the ADJ program.\n\n"
                "Question: Is there a value of x with x >= 3 and x <= 5?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain x >= 3\n"
                "constrain x <= 5\n"
                "check\n\n"
                "Question: Is there a value of x with x >= 5 and x <= 3?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain x >= 5\n"
                "constrain x <= 3\n"
                "check\n\n"
                "Question: Are there values x and y with x + y = 10, x >= 4, "
                "and y >= 4?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "symbol y : scalar\n"
                "constrain x + y = 10\n"
                "constrain x >= 4\n"
                "constrain y >= 4\n"
                "check\n\n"
                f"Question: {item['stem']}\nProgram:"
            )
        if answer_from.get("type") in {"optimize_value", "optimize_assignment"}:
            target = (
                "the optimum value"
                if answer_from.get("type") == "optimize_value"
                else f"the requested `{name}` witness value"
            )
            return (
                "Translate the word problem into a native ADJ linear optimization "
                "program. Declare the variables, add every stated constraint, then "
                "end with the requested `maximize` or `minimize` objective. Use "
                f"ONLY numbers that appear in the question. Do NOT compute {target}, "
                "do NOT mention the answer choices, and output ONLY the ADJ program.\n\n"
                "Question: Choose x and y with x and y at least 0. The constraints "
                "are x + y <= 4 and x <= 3. Maximize 3x + 2y. What is the maximum "
                "value?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "symbol y : scalar\n"
                "constrain x + y <= 4\n"
                "constrain x <= 3\n"
                "constrain x >= 0\n"
                "constrain y >= 0\n"
                "maximize 3 * x + 2 * y\n\n"
                "Question: Choose x and y with x >= 2 and y >= 3. Minimize x + y. "
                "What is the minimum value?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "symbol y : scalar\n"
                "constrain x >= 2\n"
                "constrain y >= 3\n"
                "minimize x + y\n\n"
                f"Question: {item['stem']}\nProgram:"
            )
        if answer_from.get("type") == "solve_roots":
            return (
                "Translate the word problem into a native ADJ solve program that "
                f"finds all real roots for `{name}`. Use ONLY numbers that appear "
                "in the question. Do NOT compute the roots, do NOT mention the "
                "answer choices, and output ONLY the ADJ program.\n\n"
                "Question: What real values of x solve x^2 = 121?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain x * x = 121\n"
                "solve for { x }\n\n"
                "Question: What real values of x solve x^2 = 144?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain latex \"$x^2 = 144$\"\n"
                "solve for { x }\n\n"
                "Question: What real values of x solve x^3 - 6x^2 + 11x - 6 = 0?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain x * x * x - 6 * x * x + 11 * x - 6 = 0\n"
                "solve for { x }\n\n"
                "Question: What real values of x solve x^4 - 10x^3 + 35x^2 - 50x + 24 = 0?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain x * x * x * x - 10 * x * x * x + 35 * x * x - 50 * x + 24 = 0\n"
                "solve for { x }\n\n"
                "Question: What real values of x solve (x - 2)(x - 5) = 0?\n"
                "Program:\n"
                "symbol x : scalar\n"
                "constrain (x - 2) * (x - 5) = 0\n"
                "solve for { x }\n\n"
                f"Question: {item['stem']}\nProgram:"
            )
        requires = answer_from.get("requires") or []
        decision_req = next((r for r in requires if r.get("type") == "decision"), None)
        if decision_req:
            leader = decision_req.get("leader", "setup_ready")
            evidence = decision_req.get("evidence", "repeated_groups_problem")
            return (
                "Translate the word problem into a native ADJ program. Use observe "
                "statements for the given quantities, derive the setup premise with "
                "`rule { head: ... when: ... }`, make the derived premise fire the "
                f"`{leader}` decision, then solve for `{name}`. Use ONLY numbers that "
                "appear in the question except structural prior/LR weights. Do NOT "
                "compute the answer, do NOT mention the answer choices, and output "
                "ONLY the ADJ program.\n\n"
                "Question: There are 21 boxes with 22 pencils in each box. How many "
                "pencils are there?\n"
                "Program:\n"
                "prior 0.001 for setup_ready\n"
                "contributes 1000000 from repeated_groups_problem to setup_ready\n"
                "observe groups(21)\n"
                "observe per_group(22)\n"
                "rule { head: repeated_groups_problem when: groups(21), per_group(22) }\n"
                "? setup_ready\n"
                "symbol x : scalar\n"
                "constrain x = groups * per_group\n"
                "solve for { x }\n\n"
                "Question: There are 23 bags with 24 beads in each bag and 25 loose "
                "beads. How many beads are there altogether?\n"
                "Program:\n"
                "prior 0.001 for setup_ready\n"
                "contributes 1000000 from repeated_groups_with_extra to setup_ready\n"
                "observe groups(23)\n"
                "observe per_group(24)\n"
                "observe extra(25)\n"
                "rule { head: repeated_groups_with_extra when: groups(23), per_group(24), extra(25) }\n"
                "? setup_ready\n"
                "symbol x : scalar\n"
                "constrain x = groups * per_group + extra\n"
                "solve for { x }\n\n"
                f"Required decision leader: {leader}\n"
                f"Required derived evidence: {evidence}\n"
                f"Question: {item['stem']}\nProgram:"
            )
        return (
            "Translate the word problem into a native ADJ solve program. Name the "
            f"requested unknown `{name}`. Use ONLY numbers that appear in the "
            "question. Do NOT compute the answer, do NOT mention the answer choices, "
            "and output ONLY the ADJ program.\n\n"
            "Question: A number plus 5 is 17. What is the number?\n"
            "Program:\n"
            "symbol x : scalar\n"
            "constrain x + 5 = 17\n"
            "solve for { x }\n\n"
            "Question: 3 times a number is 24. What is the number?\n"
            "Program:\n"
            "symbol x : scalar\n"
            "constrain 3 * x = 24\n"
            "solve for { x }\n\n"
            "Question: Two numbers x and y have sum 10 and difference 2. What is x?\n"
            "Program:\n"
            "symbol x : scalar\n"
            "symbol y : scalar\n"
            "constrain x + y = 10\n"
            "constrain x - y = 2\n"
            "solve for { x, y }\n\n"
            "Question: For x and y, x + 2y = 23 and x + y = 14. What is y?\n"
            "Program:\n"
            "symbol x : scalar\n"
            "symbol y : scalar\n"
            "constrain latex \"$x + 2y = 23$\"\n"
            "constrain latex \"$x + y = 14$\"\n"
            "solve for { x, y }\n\n"
            f"Question: {item['stem']}\nProgram:"
        )

    # Two worked examples (a bare expression and a one-step word problem) give a small
    # model a fair shot at the FORMAT — we are measuring its decomposition ability, not
    # its prompt-guessing. The examples carry no overlap with any bank item's numbers.
    return (
        "Translate the arithmetic in a question into a SINGLE ADJ expression using ONLY "
        "the numbers that appear in the question. Prefer + - * / and parentheses; if "
        "you use LaTeX, wrap it as ADJ syntax like latex \"$5 \\times 12$\". Do NOT "
        "compute the answer. Output ONLY the expression, on one line.\n\n"
        "Question: What is 11 * 4 - 6?\nFormula: 11 * 4 - 6\n\n"
        "Question: A box holds 8 pens. How many pens are in 3 boxes?\nFormula: 3 * 8\n\n"
        f"Question: {item['stem']}\nFormula:"
    )


_FORMULA_OK = re.compile(r"^[\d\s+\-*/().]+$")
_LABEL = re.compile(r"(?i)^\s*formula\s*[:=]?\s*")
_PROGRAM_LABEL = re.compile(r"(?i)^\s*program\s*[:=]?\s*")
_NATIVE_LATEX_EXPR = re.compile(r'^latex\s+"([^"\\]|\\.)*"$')
_CODE_FENCE = re.compile(r"```(?:adj|adj-lang)?\s*(.*?)```", re.IGNORECASE | re.DOTALL)
_ADJ_LINE_PREFIXES = (
    "symbol ",
    "observe ",
    "constrain ",
    "solve ",
    "check",
    "minimize ",
    "maximize ",
    "let ",
    "prior ",
    "contributes ",
    "interacts ",
    "rule ",
    "relate ",
    "? ",
)


def extract_formula(text: str) -> str | None:
    """Take the model's reply and return the first line that is a plain ASCII
    arithmetic expression — digits, the four operators + - * /, and parentheses.
    Or a native ADJ `latex "..."` expression. Anything else → None (abstain).

    The only cosmetic step is stripping a leading "Formula:" label the model may echo;
    we never rewrite the math. A model that wants LaTeX must emit ADJ's native
    `latex "..."` expression syntax; parsing and solving belong to adj-lang."""
    for raw in (text or "").splitlines():
        line = _LABEL.sub("", raw.strip()).rstrip(".")
        if line and (_FORMULA_OK.match(line) or _NATIVE_LATEX_EXPR.match(line)):
            return line
    return None


def extract_program(text: str) -> str | None:
    """Extract a small native ADJ program from a model response.

    For the first solve-backed rung we keep this conservative: prose is ignored,
    code fences are accepted, and only known ADJ statement prefixes are retained."""
    if not text:
        return None
    m = _CODE_FENCE.search(text)
    body = m.group(1) if m else text
    lines: list[str] = []
    for raw in body.splitlines():
        line = _PROGRAM_LABEL.sub("", raw.strip())
        if not line or line.startswith("#") or line.startswith("```"):
            continue
        if line.startswith(_ADJ_LINE_PREFIXES):
            lines.append(line)
    program = "\n".join(lines).strip()
    if not program:
        return None
    return program + "\n"


# Gemma is the canonical base target for the ladder: a small, fully-LOCAL, non-frontier
# model (no API, runs offline on commodity Apple-silicon via MLX). The headline claim is
# explicitly "a Haiku- or Gemma-class model + ADJ passes an exam the model alone cannot",
# so these aliases let `--model gemma` Just Work against the cached instruct checkpoints.
MODEL_ALIASES = {
    "gemma": "mlx:mlx-community/gemma-3-4b-it-bf16",      # default base target (4B)
    "gemma-4b": "mlx:mlx-community/gemma-3-4b-it-bf16",
    "gemma-1b": "mlx:mlx-community/gemma-3-1b-it-bf16",   # even smaller — wider gap earlier
}


def load_gen(spec: str):
    """Build a prompt→text callable from a --model spec. Accepts a Gemma alias
    (`gemma`, `gemma-1b`, …), `mlx:<repo>` for any local MLX checkpoint (Apple
    silicon), or `cmd:<shell>` which pipes the prompt to a command's stdin and reads
    its stdout (works with any local inference server / wrapper, e.g. ollama)."""
    spec = MODEL_ALIASES.get(spec, spec)
    if spec.startswith("cmd:"):
        shell = spec[4:]

        def gen(prompt: str) -> str:
            out = subprocess.run(shell, shell=True, input=prompt,
                                 capture_output=True, text=True)
            return out.stdout

        return gen
    if spec.startswith("mlx:"):
        repo = spec[4:]
        from mlx_lm import generate, load              # lazy: only needed in model mode
        from mlx_lm.sample_utils import make_sampler

        model, tok = load(repo)
        sampler = make_sampler(temp=0.0)              # greedy → reproducible runs

        def gen(prompt: str) -> str:
            msgs = [{"role": "user", "content": prompt}]
            templated = tok.apply_chat_template(msgs, add_generation_prompt=True, tokenize=False)
            return generate(model, tok, prompt=templated, max_tokens=64,
                            sampler=sampler, verbose=False)

        return gen
    raise SystemExit(f"unknown --model spec {spec!r} (use a gemma alias, mlx:<repo>, or cmd:<shell>)")


# ----------------------------------------------------------------------------------
# Driver.
# ----------------------------------------------------------------------------------
def run(rung: str, gen, model: str | None = None) -> Scorecard:
    items = json.loads((HERE / rung / "items.json").read_text())["items"]
    mode = "cached" if gen is None else "model"
    card = Scorecard(rung, mode, model)
    for it in items:
        card.results.append(score_item(it, gen))
    return card


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="ADJ-LADDER two-arm scoreboard")
    ap.add_argument("rung", help="rung directory name, e.g. rung0_arithmetic")
    ap.add_argument("--model", help="model spec: a gemma alias (gemma, gemma-1b), mlx:<repo>, or cmd:<shell> (omit for cached engine-only run)")
    ap.add_argument("--quiet", action="store_true", help="emit scorecard JSON only")
    args = ap.parse_args(argv)

    gen = load_gen(args.model) if args.model else None
    card = run(args.rung, gen, args.model)
    summary = card.summary()

    scorecard = {
        "summary": summary,
        "results": [
            {"id": r.item_id, "gold": r.gold,
             "arm_a": r.arm_a, "arm_a_outcome": r.arm_a_outcome,
             "arm_b": r.arm_b, "arm_b_outcome": r.arm_b_outcome,
             "bucket": r.bucket}
            for r in card.results
        ],
    }
    # Write per-mode/model files so a cached CI run never clobbers a committed two-arm
    # headline: cached → ladder-scorecard.json; model → ladder-scorecard.<model>.json.
    if card.mode == "cached":
        out_name = "ladder-scorecard.json"
    else:
        slug = re.sub(r"[^a-z0-9.-]+", "-", args.model.lower()).strip("-")
        out_name = f"ladder-scorecard.{slug}.json"
    (HERE / out_name).write_text(json.dumps(scorecard, indent=2) + "\n")

    if not args.quiet:
        _pretty(card, summary)

    # GATE: in cached mode the engine must never miscompute — a single wrong Arm B
    # selection is a hard failure. (In model mode wrong answers are expected data,
    # so we don't gate on them — the scorecard is the artifact.)
    if card.mode == "cached":
        if _CLI is None:
            print("\n  ⚠ adj-lang-cli not found — Arm B abstained on every item. "
                  "Build it: cargo build -p adj-lang-cli", file=sys.stderr)
            return 1
        return 1 if summary["arm_b_model_plus_adj"]["wrong"] > 0 else 0
    return 0


def _pretty(card: Scorecard, summary: dict) -> None:
    print(f"ADJ-LADDER — {card.rung}  (mode: {card.mode})\n")
    for r in card.results:
        a = r.arm_a or "·"
        b = r.arm_b or "·"
        mark = {"correct": "✓", "abstained": "·", "wrong": "✗"}
        bucket = f"  [{r.bucket}]" if r.bucket else ""
        print(f"  {r.item_id:<10} gold {r.gold}   "
              f"A:{a}{mark[r.arm_a_outcome]}  B:{b}{mark[r.arm_b_outcome]}{bucket}")
    b = summary["arm_b_model_plus_adj"]
    a = summary["arm_a_model_alone"]
    print(f"\n  Arm B (model+ADJ)  raw {b['raw_accuracy']:.0%}  "
          f"correct {b['correct']}/{b['total']}  wrong {b['wrong']}")
    if card.mode == "model":
        print(f"  Arm A (model alone) raw {a['raw_accuracy']:.0%}  "
              f"correct {a['correct']}/{a['total']}  wrong {a['wrong']}")
        d = summary["divergence"]
        print(f"  ► divergence (B − A): +{d['raw_accuracy']:.0%}  (+{d['correct']} items)")
    if summary["arm_b_failure_buckets"]:
        print(f"  Arm B failure buckets: {summary['arm_b_failure_buckets']}")
    if card.mode == "cached" and b["wrong"] == 0:
        print("\n  ✓ engine computed every answer exactly — no miscomputation.")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
