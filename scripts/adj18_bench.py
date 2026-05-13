#!/usr/bin/env python3
"""ADJ18 benchmark harness.

Iterates the canonical ADJ12/ADJ15/ADJ17 bench shape across:

  * 8 TSA declaration variants (single-item declarations isolating
    one verdict per item).
  * 5 models (gemma4:latest, llama3.1:8b, qwen2.5:{3b,1.5b,0.5b}).
  * 3 modes:
      - `none`              : no rulebook injected, default single-turn Arm A.
      - `fixture-single`    : `tsa_rulebook_fixture` injected, v0.11 single-turn dispatch.
      - `fixture-priming`   : same fixture rulebook, v0.12 two-turn priming dispatch
                              (turn 1: rulebook + ACK, turn 2: declaration + verdict).

Total: 8 × 5 × 3 = 120 cells.  Each cell makes 1-2 Ollama calls.
Allow 5-30s per cell; full bench is roughly 2-4 hours on commodity
hardware, longer if Ollama swaps models between runs.

Output: one JSON file with per-cell records (verdict, latency,
truncation, raw Arm A text). Re-runs with --resume skip cells that
already have a record in the output file, so an overnight bench
that crashes can be resumed without losing progress.

Usage:
    # Bench against a local Ollama on the default port:
    python3 scripts/adj18_bench.py \
        --endpoint http://127.0.0.1:11434 \
        --cache-dir /tmp/adj18_cache \
        --out code/specs/data/adj18-tsa-bench-$(date +%F).json

    # Resume an interrupted run:
    python3 scripts/adj18_bench.py --resume --out code/specs/data/adj18-tsa-bench-2026-05-13.json

Each cell is a subprocess call into the built adjudication-tsa-demo
binary; the harness sets env vars per cell and parses the binary's
stdout. The demo binary's Arm A output shape is stable across v0.7
through v0.12 (the `VERDICT:` line position changed in v0.12 but
the regex below tolerates both first-line and last-line positions).

The benchmark is intentionally LLM-driven — it measures the
LLM-with-rulebook story (Arms A and B), NOT the deterministic
engine arm (Arm C). Arm C bench will be a separate harness once
the fact-elicitation primitive lands (see ADJ19 draft).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional

# ---------------------------------------------------------------------------
# Bench matrix
# ---------------------------------------------------------------------------

DECLARATIONS = [
    {
        "id": "matches",
        "text": "1 carry-on bag, matches.",
        "expected": "NON-COMPLIANT",
        "rationale": "Strike-anywhere matches prohibited under TSA flammable rule.",
    },
    {
        "id": "large-lithium",
        "text": "1 carry-on bag, lithium battery, 200 Wh.",
        "expected": "NON-COMPLIANT",
        "rationale": "Lithium batteries above 100 Wh prohibited in carry-on.",
    },
    {
        "id": "large-toothpaste",
        "text": "1 carry-on bag, 4 oz toothpaste.",
        "expected": "NON-COMPLIANT",
        "rationale": "4 oz exceeds the 3.4 oz / 100 ml liquid limit.",
    },
    {
        "id": "pocket-knife",
        "text": "1 carry-on bag, 4 inch pocket knife.",
        "expected": "NON-COMPLIANT",
        "rationale": "Pocket knife blade > 2.36 in (60 mm) prohibited in carry-on.",
    },
    {
        "id": "wine-bottle",
        "text": "1 carry-on bag, 1 bottle of wine, 750 ml.",
        "expected": "NON-COMPLIANT",
        "rationale": "750 ml liquid exceeds the 3.4 oz / 100 ml limit.",
    },
    {
        "id": "small-lithium",
        "text": "1 carry-on bag, lithium battery, 50 Wh.",
        "expected": "COMPLIANT",
        "rationale": "Lithium batteries under 100 Wh permitted in carry-on.",
    },
    {
        "id": "small-perfume",
        "text": "1 carry-on bag, 3 oz perfume.",
        "expected": "COMPLIANT",
        "rationale": "3 oz fits within the 3.4 oz liquid limit.",
    },
    {
        "id": "lighter-disposable",
        "text": "1 carry-on bag, disposable lighter.",
        "expected": "COMPLIANT",
        "rationale": "One disposable lighter per passenger is permitted.",
    },
]

MODELS = [
    "gemma4:latest",
    "llama3.1:8b",
    "qwen2.5:3b",
    "qwen2.5:1.5b",
    "qwen2.5:0.5b",
]

# Each mode is a dict of env var overrides applied on top of the
# defaults set in `build_env()`. The empty-dict `none` baseline runs
# with no rulebook and default v0.12 single-turn dispatch.
MODES = {
    "none": {},
    "fixture-single": {
        "ADJ_DEMO_RULEBOOK_MODE": "fixture",
        "ADJ_DEMO_ARM_A_MODE": "single-turn",
    },
    "fixture-priming": {
        "ADJ_DEMO_RULEBOOK_MODE": "fixture",
        "ADJ_DEMO_ARM_A_MODE": "priming",
    },
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

VERDICT_RE = re.compile(r"VERDICT:\s*(COMPLIANT|NON-COMPLIANT)", re.IGNORECASE)
ARM_A_BLOCK_RE = re.compile(r"--- ARM A: raw model ---(.*?)(?:---|$)", re.DOTALL)
TRUNCATION_RE = re.compile(r"Arm A failed:.*?truncated", re.IGNORECASE)
FINISH_REASON_RE = re.compile(r"finish reason:\s+(\S+)")
LATENCY_RE = re.compile(r"latency:\s+(\d+)\s*ms")
TOKENS_RE = re.compile(r"tokens \(in/out\):\s+(\d+)\s*/\s*(\d+)")


def parse_arm_a_block(stdout: str) -> Dict[str, Optional[str]]:
    """Extract structured fields from the Arm A block of stdout."""
    block_match = ARM_A_BLOCK_RE.search(stdout)
    block = block_match.group(1) if block_match else stdout
    verdict = None
    m = VERDICT_RE.search(block)
    if m:
        verdict = m.group(1).upper()
    finish = None
    m = FINISH_REASON_RE.search(block)
    if m:
        finish = m.group(1)
    latency_ms = None
    m = LATENCY_RE.search(block)
    if m:
        latency_ms = int(m.group(1))
    in_tok = out_tok = None
    m = TOKENS_RE.search(block)
    if m:
        in_tok = int(m.group(1))
        out_tok = int(m.group(2))
    truncated = bool(TRUNCATION_RE.search(stdout))
    return {
        "verdict": verdict,
        "finish_reason": finish,
        "latency_ms": latency_ms,
        "input_tokens": in_tok,
        "output_tokens": out_tok,
        "truncated": truncated,
        "raw_block": block.strip()[:4000],  # cap for storage sanity
    }


def cell_id(declaration_id: str, model: str, mode: str) -> str:
    """Stable per-cell identifier used for resume-aware skipping."""
    return f"{declaration_id}::{model}::{mode}"


def build_env(
    base: Dict[str, str],
    declaration_text: str,
    model: str,
    mode_overrides: Dict[str, str],
) -> Dict[str, str]:
    env = base.copy()
    env["ADJ_DEMO_SOURCE"] = declaration_text
    env["ADJ_DEMO_MODEL"] = model
    env["ADJ_DEMO_IR_MODE"] = "hand"
    env["ADJ_DEMO_TIMEOUT_SECS"] = "300"
    env["ADJ_DEMO_MAX_ANSWER_TOKENS"] = "2048"
    for k, v in mode_overrides.items():
        env[k] = v
    return env


def run_cell(binary: str, env: Dict[str, str], cell_timeout_s: int) -> Dict:
    """Run a single bench cell and parse the output."""
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
        parsed = parse_arm_a_block(result.stdout)
        parsed["exit_code"] = result.returncode
        parsed["wallclock_s"] = round(elapsed, 1)
        parsed["stderr_excerpt"] = (result.stderr or "").strip()[:1000]
        return parsed
    except subprocess.TimeoutExpired:
        return {
            "verdict": None,
            "finish_reason": "timeout",
            "wallclock_s": cell_timeout_s,
            "truncated": False,
            "exit_code": -1,
            "raw_block": "",
            "stderr_excerpt": "harness timeout",
        }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        default="code/packages/rust/target/release/adjudication-tsa-demo",
        help="Path to the built demo binary. Build with: cargo build -p adjudication-tsa-demo --release",
    )
    parser.add_argument(
        "--endpoint",
        default="http://127.0.0.1:11434",
        help="Ollama endpoint. macOS users: prefer 127.0.0.1 over localhost (the latter resolves to ::1 which Ollama doesn't bind to by default).",
    )
    parser.add_argument(
        "--cache-dir",
        default="/tmp/adj18_cache",
        help="Disk cache directory shared across runs.",
    )
    parser.add_argument(
        "--out",
        required=True,
        help="Output JSON path. If --resume is set, existing cells in this file are skipped.",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Skip cells whose record already exists in --out.",
    )
    parser.add_argument(
        "--cell-timeout",
        type=int,
        default=600,
        help="Per-cell wallclock cap in seconds.",
    )
    parser.add_argument(
        "--models",
        default=",".join(MODELS),
        help="Comma-separated subset of MODELS to run.",
    )
    parser.add_argument(
        "--modes",
        default=",".join(MODES.keys()),
        help="Comma-separated subset of MODES to run.",
    )
    parser.add_argument(
        "--declarations",
        default=",".join(d["id"] for d in DECLARATIONS),
        help="Comma-separated subset of declaration ids to run.",
    )
    args = parser.parse_args()

    if not Path(args.binary).exists():
        print(
            f"error: binary not found at {args.binary}\n"
            f"build it first: cargo build -p adjudication-tsa-demo --release",
            file=sys.stderr,
        )
        return 1

    selected_models = [m.strip() for m in args.models.split(",") if m.strip()]
    selected_modes = [m.strip() for m in args.modes.split(",") if m.strip()]
    selected_decl_ids = {d.strip() for d in args.declarations.split(",") if d.strip()}
    declarations = [d for d in DECLARATIONS if d["id"] in selected_decl_ids]

    # Load existing results if resuming.
    existing: Dict[str, Dict] = {}
    if args.resume and Path(args.out).exists():
        with open(args.out) as f:
            data = json.load(f)
        for record in data.get("cells", []):
            existing[record["cell_id"]] = record
        print(f"resume: {len(existing)} existing cells loaded; will skip them")

    base_env = os.environ.copy()
    base_env["ADJ_DEMO_ENDPOINT"] = args.endpoint
    if args.cache_dir:
        Path(args.cache_dir).mkdir(parents=True, exist_ok=True)
        base_env["ADJ_DEMO_CACHE_DIR"] = args.cache_dir

    cells: List[Dict] = list(existing.values())
    total = len(declarations) * len(selected_models) * len(selected_modes)
    completed = len(existing)
    print(f"running {total - completed} cells (already done: {completed}, total matrix: {total})")

    for declaration in declarations:
        for model in selected_models:
            for mode in selected_modes:
                cid = cell_id(declaration["id"], model, mode)
                if cid in existing:
                    continue
                env = build_env(base_env, declaration["text"], model, MODES[mode])
                started = time.time()
                print(f"  [{completed + 1}/{total}] {cid} ...", end="", flush=True)
                result = run_cell(args.binary, env, args.cell_timeout)
                completed += 1
                print(
                    f" verdict={result.get('verdict')!r}"
                    f" finish={result.get('finish_reason')!r}"
                    f" truncated={result.get('truncated')}"
                    f" wallclock={result.get('wallclock_s')}s"
                )
                record = {
                    "cell_id": cid,
                    "declaration_id": declaration["id"],
                    "declaration_text": declaration["text"],
                    "expected_verdict": declaration["expected"],
                    "model": model,
                    "mode": mode,
                    "rationale": declaration["rationale"],
                    "result": result,
                }
                cells.append(record)
                # Persist after every cell so a crash loses at most
                # the cell currently in flight.
                with open(args.out, "w") as f:
                    json.dump(
                        {
                            "harness_version": "adj18-v1",
                            "endpoint": args.endpoint,
                            "binary": args.binary,
                            "cells": cells,
                        },
                        f,
                        indent=2,
                    )

    # Final summary.
    correct = 0
    truncated = 0
    for c in cells:
        if c["result"].get("verdict") == c["expected_verdict"]:
            correct += 1
        if c["result"].get("truncated"):
            truncated += 1
    print()
    print(f"=== ADJ18 bench summary ===")
    print(f"total cells:        {len(cells)}")
    print(f"correct verdicts:   {correct} / {len(cells)} ({100*correct/max(1,len(cells)):.1f}%)")
    print(f"truncated answers:  {truncated} / {len(cells)} ({100*truncated/max(1,len(cells)):.1f}%)")
    print(f"results:            {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
