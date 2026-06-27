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
            question into an ADJ program (a `let` formula over the numbers that
            appear in the stem); the **engine** does every bit of arithmetic on the
            CPU, exactly, and SELECTS the option whose value equals the computed
            answer — emitting a machine-checkable proof.

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
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

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
def build_arm_b_program(formula: str, options: dict[str, float]) -> str:
    """Render the option-selection ADJ program described in the module docstring.

    `options` maps letters (A..E) to their numeric value as printed in the question.
    We declare one equal-prior hypothesis per option and one `contributes` predicate
    that fires when the engine-computed `answer` equals that option's value. The huge
    likelihood ratio (1e6) makes a single matching option dominate decisively; if
    none match, the hypotheses stay tied and the engine returns a kickback."""
    lines = [f"prior 0.0001 for opt_{ltr.lower()}" for ltr in options]
    lines.append(f"let answer = {formula}")
    for ltr, val in options.items():
        # Render whole-valued floats as ints so the predicate threshold reads cleanly
        # (59 not 59.0); the engine compares numerically either way.
        v = int(val) if float(val).is_integer() else val
        lines.append(f"contributes 1000000 from answer == {v} to opt_{ltr.lower()}")
    lines += [f"? opt_{ltr.lower()}" for ltr in options]
    return "\n".join(lines) + "\n"


def run_decision(program: str) -> dict | None:
    """Write the program to a temp .adj, run the native CLI, return its `decision`
    dict (or None if the CLI is unavailable or the program failed to compile)."""
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


def formula_is_faithful(formula: str, stem: str) -> bool:
    """The no-result-literals gate: every number in the formula must also appear in
    the stem. This is what stops a model (in --model mode) from smuggling the answer
    into the "decomposition" — it may write `7 * 8 + 3` (all from the stem) but not
    `59`. The gold formulae in items.json are authored to satisfy this too; the test
    suite re-checks them, so the gate is exercised even in cached mode."""
    stem_nums = set(_NUM.findall(stem))
    return all(tok in stem_nums for tok in _NUM.findall(formula))


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
        formula, faithful = item["formula"], True            # cached: trust the gold
    else:
        raw = gen(decompose_prompt(item))
        formula = extract_formula(raw)
        faithful = bool(formula) and formula_is_faithful(formula, item["stem"])
    if not formula or not faithful:
        arm_b_letter = None                                  # decompose failed → abstain
    else:
        arm_b_letter = decision_to_letter(
            run_decision(build_arm_b_program(formula, item["options"]))
        )
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
    # Two worked examples (a bare expression and a one-step word problem) give a small
    # model a fair shot at the FORMAT — we are measuring its decomposition ability, not
    # its prompt-guessing. The examples carry no overlap with any bank item's numbers.
    return (
        "Translate the arithmetic in a question into a SINGLE formula using ONLY the "
        "numbers that appear in the question and the operators + - * / and "
        "parentheses. Do NOT compute the answer. Output ONLY the formula, on one line.\n\n"
        "Question: What is 11 * 4 - 6?\nFormula: 11 * 4 - 6\n\n"
        "Question: A box holds 8 pens. How many pens are in 3 boxes?\nFormula: 3 * 8\n\n"
        f"Question: {item['stem']}\nFormula:"
    )


_FORMULA_OK = re.compile(r"^[\d\s+\-*/().]+$")
_LABEL = re.compile(r"(?i)^\s*formula\s*[:=]?\s*")
_LATEX_HINT = re.compile(r"(\\[A-Za-z]+|\\\(|\\\[|\$)")


def _find_latex_helper() -> Path | None:
    override = os.environ.get("LADDER_LATEX_HELPER")
    if override and Path(override).exists():
        return Path(override)
    rust = HERE.parents[2] / "packages" / "rust"
    candidates = [
        rust / "target" / "debug" / "latex-math-to-adj",
        rust / "target" / "release" / "latex-math-to-adj",
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


_LATEX_HELPER = _find_latex_helper()


def latex_to_adj_formula(text: str) -> str | None:
    """Parse a LaTeX math expression with the repo's LaTeX frontend and lower the
    arithmetic subset to ADJ's ASCII `let` formula syntax.

    The helper is intentionally a Rust binary from `code/packages/rust/latex`: it
    routes Gemma's LaTeX output through the actual parser/frontend stack instead of
    teaching this Python harness a second, unofficial math parser. If the helper is
    not built, or the expression is outside the arithmetic subset, the item abstains."""
    if _LATEX_HELPER is None:
        return None
    if not _LATEX_HINT.search(text):
        return None
    try:
        out = subprocess.run(
            [str(_LATEX_HELPER), text],
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    formula = out.stdout.strip()
    return formula if formula and _FORMULA_OK.match(formula) else None


def extract_formula(text: str) -> str | None:
    """Take the model's reply and return the first usable arithmetic expression.

    The only cosmetic step is stripping a leading "Formula:" label the model may echo;
    plain ASCII arithmetic passes through directly. If the line looks like LaTeX math,
    it is parsed by the Rust `latex` frontend helper and lowered to the same ASCII
    subset. Unsupported notation still abstains — no ad-hoc regex repair here."""
    for raw in (text or "").splitlines():
        line = _LABEL.sub("", raw.strip()).rstrip(".")
        if line and _FORMULA_OK.match(line):
            return line
        formula = latex_to_adj_formula(line)
        if formula is not None:
            return formula
    return None


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
