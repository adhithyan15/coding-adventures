#!/usr/bin/env python3
"""compose_pipeline.py — the WHOLE-PAPER decompose→derive→verify→explain pipeline (AC-3).

The multi-paragraph counterpart to decompose_pipeline.py (ADJ-ARGUMENT-COMPOSITION.md §5 AC-3).
Where decompose_pipeline drives a single paragraph argument against ONE snapshot, this drives a
whole *paper* — several paragraph arguments composed into one `argument`, each paragraph pinned to
its OWN source snapshot — and proves it end to end:

  1. EMIT     — the composed `.adj` is the authored artifact (a paper's paragraphs in one block,
                each citation naming its own paragraph's snapshot; the surface needs no new
                construct — see ADJ-ARGUMENT-COMPOSITION §4).
  2. DERIVE   — `adj-lang-cli` chains the inference rules ACROSS paragraphs to the paper's thesis
                (a later paragraph's inference references an earlier paragraph's conclusion).
  3. VERIFY   — `adj-verify --snapshots` byte-anchors EVERY citation against the paragraph it came
                from, with ALL paragraphs placed as content-addressed snapshots in one dir. This is
                the MULTI-SNAPSHOT proof: quotes_verified == the paper's citation count, spread
                across the paragraph snapshots.
  4. EXPLAIN  — `adj-lang-cli --explain` renders the cross-paragraph chain, each step carrying its
                own paragraph's provenance.

A "paper" is a directory holding some `*.source.txt` paragraph files + one composed `.adj` (e.g.
the committed `code/specs/data/adj-argument-ir/composition/`). Stages 2-4 shell out to the built
binaries and skip gracefully when absent (the paragraph inventory in stage 1 is always pure).

Usage:
  python3 compose_pipeline.py                       # the committed composition/ paper
  python3 compose_pipeline.py --paper <dir>
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import gen_argument_data as gad  # noqa: E402

# The committed AC-2 whole-paper example: train → mycin-2026 → data → specs, then the arg-ir dir.
DEFAULT_PAPER = HERE.parents[2] / "data" / "adj-argument-ir" / "composition"


def paragraphs_of(paper_dir: Path) -> dict[str, bytes]:
    """The paper's paragraph sources — every `*.source.txt`, keyed by its stem (the paragraph
    name), in sorted order so the report is deterministic."""
    return {
        p.name[: -len(".source.txt")]: p.read_bytes()
        for p in sorted(paper_dir.glob("*.source.txt"))
    }


def composed_adj(paper_dir: Path) -> Path:
    """The paper's one composed `.adj` program."""
    adjs = sorted(paper_dir.glob("*.adj"))
    if len(adjs) != 1:
        raise ValueError(f"expected exactly one composed .adj in {paper_dir}, found {len(adjs)}")
    return adjs[0]


def _cli(args: list[str]) -> str:
    return subprocess.run(args, capture_output=True, text=True, timeout=60).stdout


def run_paper(paper_dir: Path) -> dict:
    """Run the four-stage whole-paper pipeline over `paper_dir`. Returns a report dict:
    {paper, paragraphs {name: sha}, adj, citations, derived, verified, quotes_verified, explained,
     ran_cli}. `ran_cli` is False (CLI fields None) when the binaries are absent — only the pure
     paragraph/citation inventory ran."""
    paras = paragraphs_of(paper_dir)
    adj_path = composed_adj(paper_dir)
    adj_text = adj_path.read_text()
    report = {
        "paper": paper_dir.name,
        "paragraphs": {name: hashlib.sha256(b).hexdigest() for name, b in paras.items()},
        "adj": adj_path.name,
        # One `quote "` per premise + per inference — the paper's citation count.
        "citations": adj_text.count('quote "'),
        "derived": None, "verified": None, "quotes_verified": None,
        "explained": None, "ran_cli": False,
    }
    if not (gad.CLI.exists() and gad.VERIFY.exists()):
        return report

    report["derived"] = _cli([str(gad.CLI), str(adj_path)])  # stage 2
    with tempfile.TemporaryDirectory() as td:  # stage 3 — MULTI-snapshot
        snaps = Path(td)
        for b in paras.values():
            (snaps / hashlib.sha256(b).hexdigest()).write_bytes(b)
        vout = _cli([str(gad.VERIFY), "--snapshots", str(snaps), str(adj_path)])
    import json  # noqa: PLC0415
    v = json.loads(vout) if vout.strip().startswith("{") else {}
    report["verified"] = v.get("verified")
    report["quotes_verified"] = v.get("totals", {}).get("quotes_verified")
    report["explained"] = _cli([str(gad.CLI), "--explain", str(adj_path)])  # stage 4
    report["ran_cli"] = True
    return report


def print_report(report: dict) -> None:
    """A human-readable whole-paper report."""
    print(f"\n=== paper: {report['paper']} ({report['adj']}) ===")
    print(f"1. EMIT — {len(report['paragraphs'])} paragraph(s), {report['citations']} citation(s), "
          "each pinned to its own snapshot:")
    for name, sha in report["paragraphs"].items():
        print(f"     {name:20s} snapshot {sha[:12]}…")
    if not report["ran_cli"]:
        print("   (adj-lang-cli / adj-verify not built — stages 2-4 skipped)")
        return
    derived_ok = '"abstained":false' in (report["derived"] or "")
    print(f"2. DERIVE — the paper's thesis (recall) — {'derived' if derived_ok else 'ABSTAINED'}")
    print(f"3. VERIFY — MULTI-snapshot byte-anchor: verified={report['verified']} "
          f"quotes_verified={report['quotes_verified']}/{report['citations']} "
          f"across {len(report['paragraphs'])} snapshots")
    print("4. EXPLAIN — the cross-paragraph chain (premises → connective → conclusion):")
    for line in (report["explained"] or "").splitlines():
        print(f"     {line}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--paper", type=Path, default=DEFAULT_PAPER,
                    help="a directory of paragraph *.source.txt + one composed .adj")
    args = ap.parse_args()
    if not args.paper.is_dir():
        print(f"compose_pipeline: not a directory: {args.paper}", file=sys.stderr)
        return 2
    print_report(run_paper(args.paper))
    return 0


if __name__ == "__main__":
    sys.exit(main())
