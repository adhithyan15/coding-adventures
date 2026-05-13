#!/usr/bin/env python3
"""ADJ23 decomposition bench: measure typed-quantity IR extraction.

ADJ18 measured Arm A (raw LLM verdict). ADJ23 measures **Arm B**'s
decomposition step — the call to `decompose_text` that turns source
text into typed IR. After ADJ21 landed `decompose-text-v5`, the
prompt explicitly teaches the model to emit `quantity(value, unit)`
compounds for every numerical literal. This bench measures whether
the model actually does so in practice.

Per-cell capture:

  * Source declaration (same 8 from ADJ18).
  * IR produced by decompose_text (audit-trail snapshot from
    `ADJ_DEMO_AUDIT=1`).
  * For each numerical literal in the source, was a corresponding
    `quantity(<lit>, _)` compound produced anywhere in the IR?
  * ADJ22 verdict (Pass / Fail with violations).
  * Wall-clock latency for the decompose call.

The matrix: 8 declarations × 5 models = 40 cells. Each cell
invokes the demo with `ADJ_DEMO_IR_MODE=llm` + `ADJ_DEMO_AUDIT=1`,
parses the dumped audit trail, walks the IR for `quantity(...)`
compounds, and records the result.

The verdict-side accuracy is NOT the bench's focus here; the
decomposition is. Arm A verdict is captured for completeness but
the headline metrics are:

  - **Typed-quantity recall**: fraction of source numerical
    literals that became `quantity(...)` compounds.
  - **ADJ22 pass rate per model**: how often the model's IR
    passes the typed-quantity coverage check on the first try.

Usage:

    cargo build -p adjudication-tsa-demo --release
    python3 scripts/adj23_decomposition_bench.py \\
        --endpoint http://127.0.0.1:11434 \\
        --cache-dir /tmp/adj23_cache \\
        --out code/specs/data/adj23-decomposition-bench-$(date +%F).json

This run takes longer than ADJ18 (per-cell Arm B includes
ADJ04/05 LLM calls plus decompose_text), so expect ~30-60 min
end-to-end.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Same 8 declarations as ADJ18.
DECLARATIONS = [
    {"id": "matches",            "text": "1 carry-on bag, matches.",                          "literals": ["1"]},
    {"id": "large-lithium",      "text": "1 carry-on bag, lithium battery, 200 Wh.",          "literals": ["1", "200"]},
    {"id": "large-toothpaste",   "text": "1 carry-on bag, 4 oz toothpaste.",                  "literals": ["1", "4"]},
    {"id": "pocket-knife",       "text": "1 carry-on bag, 4 inch pocket knife.",              "literals": ["1", "4"]},
    {"id": "wine-bottle",        "text": "1 carry-on bag, 1 bottle of wine, 750 ml.",         "literals": ["1", "1", "750"]},
    {"id": "small-lithium",      "text": "1 carry-on bag, lithium battery, 50 Wh.",           "literals": ["1", "50"]},
    {"id": "small-perfume",      "text": "1 carry-on bag, 3 oz perfume.",                     "literals": ["1", "3"]},
    {"id": "lighter-disposable", "text": "1 carry-on bag, disposable lighter.",               "literals": ["1"]},
]

MODELS = [
    "gemma4:latest",
    "llama3.1:8b",
    "qwen2.5:3b",
    "qwen2.5:1.5b",
    "qwen2.5:0.5b",
]

# Match the literal regex used by the ADJ22 checker — keep them
# aligned so analysis matches the in-Rust validator's view.
LITERAL_RE = re.compile(r"\d+(?:\.\d+)?")

# Quantity compound discriminator inside the audit-trail IR. A
# JSON-shaped Term::Compound has {"functor": "quantity", "args": [{"atom":"4"}, {"atom":"inches"}]}.
# We walk the JSON tree looking for any object with this shape.


def cell_id(decl_id: str, model: str) -> str:
    return f"{decl_id}::{model}"


def normalise_numeric(s: str) -> str:
    """Mirror the Rust normalise_numeric — `"4"`, `"4.0"`, `"04"` all canonicalise to `"4"`."""
    if "." in s:
        whole, frac = s.split(".", 1)
    else:
        whole, frac = s, ""
    whole = whole.lstrip("0") or "0"
    frac = frac.rstrip("0")
    return f"{whole}.{frac}" if frac else whole


def walk_for_quantities(value, found: List[Tuple[str, str]]) -> None:
    """Walk an audit-trail IR JSON value, append (literal, unit) for every
    quantity(value, unit) compound found at any depth.

    Subtlety: the v3 audit trail's `ir_nodes[].payload` only carries
    node *metadata* (id/kind/modality/polarity) — the term tree is
    NOT serialized there. The LLM-produced term trees live inside
    `dialogue[*].response.text` (string-encoded IR JSON), and
    inside `dialogue[*].question_text` (prior attempts quoted back
    in the reprompt). To collect every quantity the LLM emitted
    across all rounds, we need to drill into those JSON-string
    fields. We do that by:

      1. Walking the normal JSON tree for quantity(...) compounds
         (so any future audit-trail enrichment is automatically
         picked up).
      2. Whenever we encounter a string that *looks* like JSON
         (starts with `{`), trying to json.loads it and recursing
         into the parsed result. Inert strings (free-form text
         like question prompts) start with letters and short-circuit.
    """
    if isinstance(value, dict):
        # Check whether this object looks like a Term::Compound with functor=="quantity"
        functor = value.get("functor")
        args = value.get("args")
        if functor == "quantity" and isinstance(args, list) and len(args) >= 2:
            # First arg is value-atom (could be {"atom":"4"} or {"int":4} etc.)
            lit = extract_atom_or_num(args[0])
            unit = extract_atom_or_num(args[1])
            if lit is not None:
                found.append((lit, unit or "?"))
        # Recurse into all fields.
        for v in value.values():
            walk_for_quantities(v, found)
    elif isinstance(value, list):
        for item in value:
            walk_for_quantities(item, found)
    elif isinstance(value, str):
        # Heuristic: only try to parse strings that look like an
        # object literal — saves a try/except per non-JSON string.
        stripped = value.lstrip()
        if not stripped or stripped[0] != "{":
            # Look for any embedded JSON object (e.g. question_text
            # quotes the rejected IR with "Your previous output was: {...}").
            idx = value.find("{\"document_id\"")
            if idx < 0:
                return
            try_text = value[idx:]
        else:
            try_text = stripped
        try:
            parsed, _ = json.JSONDecoder().raw_decode(try_text)
        except (json.JSONDecodeError, ValueError):
            return
        walk_for_quantities(parsed, found)


def extract_atom_or_num(term_json) -> Optional[str]:
    """Given a Term-shaped JSON object, extract its atom-or-num string form."""
    if not isinstance(term_json, dict):
        return None
    if "atom" in term_json:
        return str(term_json["atom"])
    if "int" in term_json:
        return str(term_json["int"])
    if "float" in term_json:
        return str(term_json["float"])
    if "num" in term_json:
        # Tolerate alternate shape
        return str(term_json["num"])
    return None


def analyse_cell(
    raw_ir_json,
    audit_trail_json,
    declaration_literals: List[str],
) -> dict:
    """Return per-cell typed-quantity recall analysis.

    Walks BOTH the raw `decompose_text` IR (which has the term tree
    intact) AND the audit trail (which embeds prior attempts inside
    string fields like `dialogue[*].response.text`). De-duplicates
    `(value, unit)` pairs after canonicalisation so a quantity that
    appears in both sources is only counted once.
    """
    quantities_found: List[Tuple[str, str]] = []
    if raw_ir_json is not None:
        walk_for_quantities(raw_ir_json, quantities_found)
    if audit_trail_json is not None:
        walk_for_quantities(audit_trail_json, quantities_found)
    # De-duplicate while preserving order.
    seen = set()
    deduped: List[Tuple[str, str]] = []
    for value, unit in quantities_found:
        canonical = (normalise_numeric(value), unit)
        if canonical in seen:
            continue
        seen.add(canonical)
        deduped.append((value, unit))
    quantities_found = deduped

    # Canonicalise both sides for matching.
    found_canonical = [normalise_numeric(q[0]) for q in quantities_found]
    decl_canonical = [normalise_numeric(lit) for lit in declaration_literals]

    matched = []
    missing = []
    for lit_c in decl_canonical:
        if lit_c in found_canonical:
            matched.append(lit_c)
        else:
            missing.append(lit_c)

    return {
        "literals_in_source": declaration_literals,
        "literals_found_in_ir": [{"value": q[0], "unit": q[1]} for q in quantities_found],
        "matched_count": len(matched),
        "missing_count": len(missing),
        "missing_literals": missing,
        "adj22_pass": len(missing) == 0,
        "literal_total": len(decl_canonical),
    }


# Same parser bones as adj18_bench.py for arm A.
VERDICT_RE = re.compile(r"VERDICT:\s*(COMPLIANT|NON-COMPLIANT|ESCALATE)", re.IGNORECASE)
TRUNCATION_RE = re.compile(r"Arm A failed:.*?truncated", re.IGNORECASE)
# The demo prints `decompose_text FAILED: <error>` (followed by a
# `fallback: hand-built TSA fixture` line) when the LLM's IR JSON
# couldn't be parsed. Capture the error string so the bench report
# can distinguish "model emitted no quantities" from "model emitted
# malformed JSON and the demo fell back to a hand-built IR that has
# no quantities by construction."
DECOMPOSE_FAIL_RE = re.compile(r"decompose_text FAILED:\s*([^\n]+)")
FALLBACK_RE = re.compile(r"fallback:\s*hand-built", re.IGNORECASE)


def run_cell(binary: str, env: Dict[str, str], cell_timeout_s: int) -> dict:
    """Run a single decomposition-bench cell.

    Captures: stdout (Arm A + Arm B summary + audit-trail dump),
    stderr, exit code, latency. The audit trail is dumped at the
    end of stdout when ADJ_DEMO_AUDIT=1 is set, prefixed by the
    raw LLM-extracted IR JSON under its own marker.
    """
    started = time.time()
    try:
        result = subprocess.run(
            [binary],
            env=env,
            capture_output=True,
            text=True,
            timeout=cell_timeout_s,
            check=False,
        )
        elapsed = time.time() - started

        stdout = result.stdout

        # The demo with ADJ_DEMO_AUDIT=1 emits TWO blocks of interest:
        #   1) `--- LLM-extracted IR (raw decompose_text output) ---`
        #      with the FINAL committed IR JSON (term tree intact).
        #   2) `--- full audit trail (ADJ07-v1) ---` with the AuditTrail
        #      JSON. ir_nodes[].payload in the audit trail is metadata-
        #      only — the term tree lives in block 1 (plus inside any
        #      dialogue clarification turns).
        raw_ir = _extract_json_block(
            stdout, "--- LLM-extracted IR (raw decompose_text output) ---"
        )
        audit_trail = _extract_json_block(
            stdout, "--- full audit trail (ADJ07-v1) ---"
        )

        # Verdict + truncation from Arm A.
        verdict_m = VERDICT_RE.search(stdout)
        verdict = verdict_m.group(1).upper() if verdict_m else None
        truncated = bool(TRUNCATION_RE.search(stdout))

        # Detect decompose_text outright failure (LLM emitted
        # malformed JSON, demo fell back to hand-built).
        decompose_fail_m = DECOMPOSE_FAIL_RE.search(stdout)
        decompose_error = decompose_fail_m.group(1).strip() if decompose_fail_m else None
        fell_back = bool(FALLBACK_RE.search(stdout))

        return {
            "exit_code": result.returncode,
            "wallclock_s": round(elapsed, 1),
            "verdict": verdict,
            "truncated": truncated,
            "raw_ir": raw_ir,
            "audit_trail": audit_trail,
            "decompose_error": decompose_error,
            "fell_back_to_hand_built": fell_back,
            "stderr_excerpt": (result.stderr or "").strip()[:1000],
        }
    except subprocess.TimeoutExpired:
        return {
            "exit_code": -1,
            "wallclock_s": cell_timeout_s,
            "verdict": None,
            "truncated": False,
            "raw_ir": None,
            "audit_trail": None,
            "decompose_error": "harness timeout",
            "fell_back_to_hand_built": False,
            "stderr_excerpt": "harness timeout",
        }


def _extract_json_block(stdout: str, marker: str):
    """Return the first JSON object that follows `marker` in `stdout`,
    or `None` if not found / not parseable."""
    if marker not in stdout:
        return None
    after = stdout.split(marker, 1)[1].lstrip()
    try:
        parsed, _ = json.JSONDecoder().raw_decode(after)
        return parsed
    except (json.JSONDecodeError, ValueError) as e:
        return {"_parse_error": str(e), "_raw_excerpt": after[:2000]}


def build_env(
    base: Dict[str, str],
    declaration_text: str,
    model: str,
) -> Dict[str, str]:
    env = base.copy()
    env["ADJ_DEMO_SOURCE"] = declaration_text
    env["ADJ_DEMO_MODEL"] = model
    # The whole point — LlmExtracted IR mode triggers decompose_text.
    env["ADJ_DEMO_IR_MODE"] = "llm"
    # And we need the audit trail dumped so we can parse the IR back out.
    env["ADJ_DEMO_AUDIT"] = "1"
    # Use the latest v0.13 prompt for Arm A so the binary doesn't reject
    # our run. Mode doesn't affect decompose_text directly.
    env["ADJ_DEMO_ARM_A_MODE"] = "single-turn"
    env["ADJ_DEMO_MAX_ANSWER_TOKENS"] = "2048"
    env["ADJ_DEMO_TIMEOUT_SECS"] = "600"
    return env


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        default="code/packages/rust/target/release/adjudication-tsa-demo",
    )
    parser.add_argument("--endpoint", default="http://127.0.0.1:11434")
    parser.add_argument("--cache-dir", default="/tmp/adj23_cache")
    parser.add_argument("--out", required=True)
    parser.add_argument(
        "--audit-dir",
        default=None,
        help="If set, dump each cell's raw audit-trail JSON here so the "
             "walker can be re-run without re-invoking the LLM.",
    )
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--cell-timeout", type=int, default=900)
    parser.add_argument("--models", default=",".join(MODELS))
    parser.add_argument("--declarations", default=",".join(d["id"] for d in DECLARATIONS))
    args = parser.parse_args()

    if not Path(args.binary).exists():
        print(f"error: binary not found at {args.binary}", file=sys.stderr)
        return 1

    selected_models = [m.strip() for m in args.models.split(",") if m.strip()]
    selected_decl_ids = {d.strip() for d in args.declarations.split(",") if d.strip()}
    declarations = [d for d in DECLARATIONS if d["id"] in selected_decl_ids]

    existing: Dict[str, dict] = {}
    if args.resume and Path(args.out).exists():
        with open(args.out) as f:
            data = json.load(f)
        for record in data.get("cells", []):
            existing[record["cell_id"]] = record
        print(f"resume: {len(existing)} cells already done")

    base_env = os.environ.copy()
    base_env["ADJ_DEMO_ENDPOINT"] = args.endpoint
    if args.cache_dir:
        Path(args.cache_dir).mkdir(parents=True, exist_ok=True)
        base_env["ADJ_DEMO_CACHE_DIR"] = args.cache_dir

    audit_dir = None
    if args.audit_dir:
        audit_dir = Path(args.audit_dir)
        audit_dir.mkdir(parents=True, exist_ok=True)

    cells: List[dict] = list(existing.values())
    total = len(declarations) * len(selected_models)
    completed = len(existing)
    print(f"running {total - completed} cells (already done: {completed}, total: {total})")

    for declaration in declarations:
        for model in selected_models:
            cid = cell_id(declaration["id"], model)
            if cid in existing:
                continue
            env = build_env(base_env, declaration["text"], model)
            print(f"  [{completed + 1}/{total}] {cid} ...", end="", flush=True)
            result = run_cell(args.binary, env, args.cell_timeout)
            completed += 1

            # Persist both raw IR + audit trail for offline re-analysis.
            if audit_dir is not None:
                safe_id = cid.replace(":", "_").replace("/", "_")
                if result["audit_trail"] is not None:
                    with open(audit_dir / f"{safe_id}.audit.json", "w") as f:
                        json.dump(result["audit_trail"], f, indent=2)
                if result["raw_ir"] is not None:
                    with open(audit_dir / f"{safe_id}.ir.json", "w") as f:
                        json.dump(result["raw_ir"], f, indent=2)

            # Analyse the produced IR.
            if result["audit_trail"] is not None or result["raw_ir"] is not None:
                analysis = analyse_cell(
                    result.get("raw_ir"),
                    result["audit_trail"],
                    declaration["literals"],
                )
            else:
                analysis = {
                    "literals_in_source": declaration["literals"],
                    "literals_found_in_ir": [],
                    "matched_count": 0,
                    "missing_count": len(declaration["literals"]),
                    "missing_literals": declaration["literals"],
                    "adj22_pass": False,
                    "literal_total": len(declaration["literals"]),
                    "ir_capture_missing": True,
                }

            print(
                f" verdict={result['verdict']!r}"
                f" quantities_matched={analysis['matched_count']}/{analysis['literal_total']}"
                f" adj22_pass={analysis['adj22_pass']}"
                f" wallclock={result['wallclock_s']}s"
            )

            # Failure mode: did decompose_text crash and the demo fall
            # back to the hand-built fixture? That's a distinct
            # category from "model emitted IR, just without quantities."
            if result.get("decompose_error"):
                print(f"        ⚠  decompose_text FAILED: "
                      f"{result['decompose_error'][:140]}")
                if result.get("fell_back_to_hand_built"):
                    print(f"        ⚠  demo fell back to hand-built IR "
                          f"(no typed quantities by construction)")

            # Show the quantities the model extracted on this cell, so the
            # operator can spot misshapes early (e.g. `quantity(4, snorgles)`
            # would still pass ADJ22's value check but indicates a unit-
            # vocabulary problem worth eyeballing).
            extracted = analysis.get("literals_found_in_ir", [])
            if extracted:
                shape = ", ".join(f"{q['value']}/{q['unit']}" for q in extracted)
                print(f"        IR quantities: [{shape}]")
            missing = analysis.get("missing_literals", [])
            if missing:
                print(f"        missing literals: {missing}")

            record = {
                "cell_id": cid,
                "declaration_id": declaration["id"],
                "declaration_text": declaration["text"],
                "model": model,
                "result": {
                    "exit_code": result["exit_code"],
                    "wallclock_s": result["wallclock_s"],
                    "verdict": result["verdict"],
                    "truncated": result["truncated"],
                    "decompose_error": result.get("decompose_error"),
                    "fell_back_to_hand_built": result.get("fell_back_to_hand_built", False),
                    "stderr_excerpt": result["stderr_excerpt"],
                },
                "analysis": analysis,
                # Don't store the entire audit trail — too large.
                # Keep a summary of nodes that contained typed quantities.
                "ir_summary": (
                    {
                        "nodes_total": (
                            len(result["raw_ir"].get("nodes", []))
                            if isinstance(result.get("raw_ir"), dict) else 0
                        ),
                        "literals_extracted": analysis["literals_found_in_ir"],
                        "had_raw_ir": result.get("raw_ir") is not None,
                        "had_audit_trail": result.get("audit_trail") is not None,
                    }
                ),
            }
            cells.append(record)
            with open(args.out, "w") as f:
                json.dump(
                    {
                        "harness_version": "adj23-v1",
                        "endpoint": args.endpoint,
                        "binary": args.binary,
                        "cells": cells,
                    },
                    f,
                    indent=2,
                )

            # Running tally every 5 cells (and at the very end).
            if completed % 5 == 0 or completed == total:
                _running_pass = sum(1 for c in cells if c["analysis"]["adj22_pass"])
                _running_lits = sum(c["analysis"]["literal_total"] for c in cells)
                _running_matched = sum(c["analysis"]["matched_count"] for c in cells)
                print(
                    f"        ---- running: {completed}/{total} cells, "
                    f"ADJ22 {_running_pass}/{completed} "
                    f"({100*_running_pass/max(1,completed):.0f}%), "
                    f"recall {_running_matched}/{_running_lits} "
                    f"({100*_running_matched/max(1,_running_lits):.0f}%)"
                )

    # Summary.
    adj22_pass = sum(1 for c in cells if c["analysis"]["adj22_pass"])
    total_literals = sum(c["analysis"]["literal_total"] for c in cells)
    matched_literals = sum(c["analysis"]["matched_count"] for c in cells)
    print()
    print("=== ADJ23 decomposition bench summary ===")
    print(f"total cells:                {len(cells)}")
    print(f"ADJ22 pass:                 {adj22_pass} / {len(cells)} ({100*adj22_pass/max(1,len(cells)):.1f}%)")
    print(f"typed-quantity recall:      {matched_literals} / {total_literals} ({100*matched_literals/max(1,total_literals):.1f}%)")
    print(f"results:                    {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
