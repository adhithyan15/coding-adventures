#!/usr/bin/env python3
"""ADJ25 PR-6 foundation bench: hierarchical decomposition × per-level coverage.

Iterates 8 declarations × 5 models = 40 cells. For each cell it
invokes the compiled `adj_pr6_bench` binary with the source text +
model + Ollama endpoint, captures the JSON output, and aggregates
results.

The eight declarations mirror ADJ18 — same single-item shapes
isolating one verdict per item. The five models mirror ADJ12 — the
gemma4 / llama3.1 8B reference pair plus the qwen2.5 3B / 1.5B /
0.5B small-model tier.

For each cell the bench captures:

  * Per-level coverage pass/fail across the four boundaries
    (Document→Sentence, Sentence→Phrase, Phrase→Claim,
    Fact→TypedComponent).
  * Flattening violations (separate from coverage gaps so we can
    see how often the LLM tries the `50_wh` flatten-into-atom
    trick).
  * Correlation completeness on the orchestrator's output.
  * Wallclock latency.
  * Total LLM calls dispatched (initial + retries).

The bench expects the binary to be built:

    cargo build -p adjudication-pipeline --bin adj_pr6_bench --release

And Ollama to be running locally with all 5 models pulled. The
harness handles cell-level timeouts and writes per-cell results to
the output JSON file incrementally so a crash loses at most the
in-flight cell.

Output: `code/specs/data/adj25-pr6-foundation-bench-YYYY-MM-DD.json`.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import pathlib
import subprocess
import sys
import time
from typing import List, Optional


# ---------------------------------------------------------------------------
# Declaration set (mirrors ADJ18)
# ---------------------------------------------------------------------------

DECLARATIONS: List[dict] = [
    {
        "id": "matches",
        "text": "1 carry-on bag, matches.",
        "expected_verdict": "NON-COMPLIANT",
    },
    {
        "id": "large-lithium",
        "text": "1 carry-on bag, lithium battery, 200 Wh.",
        "expected_verdict": "NON-COMPLIANT",
    },
    {
        "id": "large-toothpaste",
        "text": "1 carry-on bag, 4 oz toothpaste.",
        "expected_verdict": "NON-COMPLIANT",
    },
    {
        "id": "pocket-knife",
        "text": "1 carry-on bag, 4 inch pocket knife.",
        "expected_verdict": "NON-COMPLIANT",
    },
    {
        "id": "wine-bottle",
        "text": "1 carry-on bag, 1 bottle of wine, 750 ml.",
        "expected_verdict": "NON-COMPLIANT",
    },
    {
        "id": "small-lithium",
        "text": "1 carry-on bag, lithium battery, 50 Wh.",
        "expected_verdict": "COMPLIANT",
    },
    {
        "id": "small-perfume",
        "text": "1 carry-on bag, 3 oz perfume.",
        "expected_verdict": "COMPLIANT",
    },
    {
        "id": "lighter-disposable",
        "text": "1 carry-on bag, disposable lighter.",
        "expected_verdict": "COMPLIANT",
    },
]


# ---------------------------------------------------------------------------
# Model set (mirrors ADJ12)
# ---------------------------------------------------------------------------

MODELS: List[str] = [
    "gemma4:latest",
    "llama3.1:8b",
    "qwen2.5:3b",
    "qwen2.5:1.5b",
    "qwen2.5:0.5b",
]


# ---------------------------------------------------------------------------
# Cell driver
# ---------------------------------------------------------------------------


@dataclasses.dataclass
class CellResult:
    """Per-cell record written to the output JSON file."""

    declaration_id: str
    declaration_text: str
    expected_verdict: str
    model: str
    wallclock_secs: float
    raw_output: dict
    exit_code: int
    stderr_excerpt: str


def run_cell(
    *,
    binary: pathlib.Path,
    declaration: dict,
    model: str,
    endpoint: str,
    timeout_secs: int,
    max_retries: int,
    cell_timeout: int,
) -> CellResult:
    env = os.environ.copy()
    env["ADJ_PR6_SOURCE"] = declaration["text"]
    env["ADJ_PR6_MODEL"] = model
    env["ADJ_PR6_ENDPOINT"] = endpoint
    env["ADJ_PR6_TIMEOUT_SECS"] = str(timeout_secs)
    env["ADJ_PR6_MAX_RETRIES"] = str(max_retries)
    env["ADJ_PR6_DOCUMENT_ID"] = f"pr6-{declaration['id']}-{model.replace(':', '_').replace('/', '_')}"

    started = time.monotonic()
    try:
        proc = subprocess.run(
            [str(binary)],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=cell_timeout,
        )
        elapsed = time.monotonic() - started
        try:
            parsed = json.loads(proc.stdout.decode("utf-8", errors="replace") or "{}")
        except json.JSONDecodeError as e:
            parsed = {
                "error": {
                    "kind": "json_decode_error",
                    "message": str(e),
                    "stdout_excerpt": proc.stdout[:512].decode("utf-8", errors="replace"),
                }
            }
        return CellResult(
            declaration_id=declaration["id"],
            declaration_text=declaration["text"],
            expected_verdict=declaration["expected_verdict"],
            model=model,
            wallclock_secs=elapsed,
            raw_output=parsed,
            exit_code=proc.returncode,
            stderr_excerpt=proc.stderr[:512].decode("utf-8", errors="replace"),
        )
    except subprocess.TimeoutExpired:
        return CellResult(
            declaration_id=declaration["id"],
            declaration_text=declaration["text"],
            expected_verdict=declaration["expected_verdict"],
            model=model,
            wallclock_secs=time.monotonic() - started,
            raw_output={"error": {"kind": "cell_timeout", "message": f"cell exceeded {cell_timeout}s"}},
            exit_code=-1,
            stderr_excerpt="",
        )


# ---------------------------------------------------------------------------
# Aggregation
# ---------------------------------------------------------------------------


def summarise(cells: List[CellResult]) -> dict:
    """Compute headline metrics across all cells.

    Reported metrics:

      * Per-model: % cells that produced any IR at all (no error),
        % cells whose IR passed the full hierarchical coverage check,
        average wallclock.
      * Per-level: across all cells that produced IR, % that passed
        each of the four boundaries.
      * Correlation completeness: % of IR-producing cells where the
        check passed.
    """
    by_model: dict = {}
    by_level: dict = {
        "DocumentToSentence": {"passed": 0, "total": 0},
        "SentenceToPhrase": {"passed": 0, "total": 0},
        "PhraseToClaim": {"passed": 0, "total": 0},
        "FactToTypedComponent": {"passed": 0, "total": 0},
    }
    correlation_pass = 0
    correlation_total = 0
    overall_pass = 0
    overall_total = 0

    for cell in cells:
        model = cell.model
        slot = by_model.setdefault(
            model,
            {"cells": 0, "ir_produced": 0, "overall_pass": 0, "wallclock_sum": 0.0},
        )
        slot["cells"] += 1
        slot["wallclock_sum"] += cell.wallclock_secs
        raw = cell.raw_output
        if "error" in raw and raw["error"] is not None:
            continue
        slot["ir_produced"] += 1
        overall_total += 1
        per_level = raw.get("per_level_coverage", {})
        if per_level.get("overall_pass"):
            slot["overall_pass"] += 1
            overall_pass += 1
        for entry in per_level.get("by_level", []):
            lvl_name = entry.get("level", "")
            if lvl_name not in by_level:
                continue
            by_level[lvl_name]["total"] += 1
            if entry.get("passed"):
                by_level[lvl_name]["passed"] += 1
        correlation_total += 1
        if raw.get("correlation_completeness") == "pass":
            correlation_pass += 1

    return {
        "totals": {
            "cells_run": len(cells),
            "ir_produced": overall_total,
            "fully_passing": overall_pass,
            "correlation_complete": correlation_pass,
            "correlation_total": correlation_total,
        },
        "by_model": by_model,
        "by_level": by_level,
    }


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--endpoint", default="http://127.0.0.1:11434")
    p.add_argument(
        "--binary",
        default="code/packages/rust/target/release/adj_pr6_bench",
        help="path to the built adj_pr6_bench binary",
    )
    p.add_argument("--timeout-secs", type=int, default=300, help="per-call LLM timeout")
    p.add_argument("--max-retries", type=int, default=3)
    p.add_argument("--cell-timeout", type=int, default=900, help="hard cap per cell")
    p.add_argument(
        "--out",
        default=None,
        help="output JSON file; defaults to "
        "code/specs/data/adj25-pr6-foundation-bench-YYYY-MM-DD.json",
    )
    p.add_argument(
        "--models",
        default=",".join(MODELS),
        help="comma-separated subset of models (for incremental runs)",
    )
    p.add_argument(
        "--declarations",
        default=",".join(d["id"] for d in DECLARATIONS),
        help="comma-separated subset of declaration ids",
    )
    args = p.parse_args(argv)

    binary = pathlib.Path(args.binary)
    if not binary.exists():
        print(f"binary not found at {binary}", file=sys.stderr)
        print(
            "build it with: cargo build -p adjudication-pipeline --bin adj_pr6_bench --release",
            file=sys.stderr,
        )
        return 2

    out_path = pathlib.Path(
        args.out
        if args.out
        else f"code/specs/data/adj25-pr6-foundation-bench-{time.strftime('%Y-%m-%d')}.json"
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)

    selected_models = [m.strip() for m in args.models.split(",") if m.strip()]
    selected_ids = set(d.strip() for d in args.declarations.split(",") if d.strip())
    selected_decls = [d for d in DECLARATIONS if d["id"] in selected_ids]

    cells: List[CellResult] = []
    total_cells = len(selected_decls) * len(selected_models)
    print(
        f"running {total_cells} cells "
        f"({len(selected_decls)} declarations × {len(selected_models)} models)",
        file=sys.stderr,
    )
    cell_index = 0
    for decl in selected_decls:
        for model in selected_models:
            cell_index += 1
            print(
                f"  [{cell_index}/{total_cells}] {decl['id']} × {model}", file=sys.stderr
            )
            result = run_cell(
                binary=binary,
                declaration=decl,
                model=model,
                endpoint=args.endpoint,
                timeout_secs=args.timeout_secs,
                max_retries=args.max_retries,
                cell_timeout=args.cell_timeout,
            )
            cells.append(result)
            # Persist after every cell so a crash loses at most the
            # in-flight cell.
            with open(out_path, "w") as fh:
                json.dump(
                    {
                        "harness_version": "adj25-pr6-v1",
                        "endpoint": args.endpoint,
                        "binary": str(binary),
                        "models": selected_models,
                        "declarations": [d["id"] for d in selected_decls],
                        "cells": [dataclasses.asdict(c) for c in cells],
                        "summary": summarise(cells),
                    },
                    fh,
                    indent=2,
                )
    print(f"wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
