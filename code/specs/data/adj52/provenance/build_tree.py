#!/usr/bin/env python3
"""Build a TREE-SHAPED JSON rulebook for case-5 from its adj-lang rulebook, and
record the patient's observations. This is Experiment B: keep the rulebook as a
tree (diagnosis -> mechanisms/findings, each carrying provenance) and answer the
query DIRECTLY on the tree — never flatten it into an adj-lang `program`.

The hypothesis under test: the program conversion (flat `observe`/`contributes`
lines compiled through the engine) loses the correlation structure and inflates
the posterior. The tree keeps "which findings are manifestations of one
mechanism" explicit, so correlated evidence fires once by construction.

Run: python build_tree.py   ->   writes case5-tree.json
"""
from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE.parent
RULEBOOK = (ADJ / "cases/case-5/rulebook.adj").read_text()
PROGRAM = (ADJ / "cases/case-5/program.adj").read_text()

# Observations the patient actually has (confirmatory test results are NOT here —
# they are the open uncertainties, so their LRs stay dormant).
OBSERVED = {
    ln.strip()[len("observe "):].strip()
    for ln in PROGRAM.splitlines()
    if ln.strip().startswith("observe ")
}
QUERIES = [
    ln.strip()[1:].strip()
    for ln in PROGRAM.splitlines()
    if ln.strip().startswith("?")
]
DIFFERENTIAL = [q for q in QUERIES if q.startswith("diagnosis(")]

lines = RULEBOOK.splitlines()


def source_after(idx: int) -> str:
    for j in range(idx + 1, min(idx + 3, len(lines))):
        s = lines[j].strip().lstrip("%").strip()
        if s.startswith("source"):
            m = re.search(r'source "([^"]*)"', s)
            if m:
                return m.group(1)
    return ""


hyps: dict[str, dict] = {}


def hyp(dx: str) -> dict:
    return hyps.setdefault(dx, {"dx": dx, "prior": None, "prior_source": "", "evidence": []})


claim_n = 0
for idx, ln in enumerate(lines):
    s = ln.strip()
    m = re.match(r"prior\s+([0-9.]+)\s+for\s+(diagnosis\([^)]*\))", s)
    if m:
        h = hyp(m.group(2))
        h["prior"] = float(m.group(1))
        h["prior_source"] = source_after(idx)
        continue
    m = re.match(r"contributes\s+([0-9.]+)\s+from\s+(.+?)\s+to\s+(diagnosis\([^)]*\))", s)
    if m:
        claim_n += 1
        finding = m.group(2).strip()
        hyp(m.group(3))["evidence"].append({
            "claim_id": f"c{claim_n}", "type": "finding", "finding": finding,
            "lr": float(m.group(1)), "observed": finding in OBSERVED,
            "source": source_after(idx),
            # verdict is filled in from the spider; default unknown.
            "provenance_verdict": "unknown",
        })
        continue
    m = re.match(r"%\s*mechanism\s+(\w+)\s+for\s+(diagnosis\([^)]*\))\s+lr\s+([0-9.]+)\s*:\s*(.+)", s)
    if m:
        claim_n += 1
        manifestations = [t.strip() for t in m.group(4).split(",") if t.strip()]
        fired = any(t in OBSERVED for t in manifestations)
        hyp(m.group(2))["evidence"].append({
            "claim_id": f"c{claim_n}", "type": "mechanism", "name": m.group(1),
            "lr": float(m.group(3)), "manifestations": manifestations,
            "observed_manifestations": [t for t in manifestations if t in OBSERVED],
            "fires": fired, "source": source_after(idx),
            "provenance_verdict": "unknown",
        })

tree = {
    "case": "case-5",
    "ground_truth": "diagnosis(bladder_urothelial_carcinoma) — flat high-grade CIS, mislabeled ~2y as chronic prostatitis",
    "observations": sorted(OBSERVED),
    "differential": DIFFERENTIAL,
    "hypotheses": [hyps[d] for d in DIFFERENTIAL if d in hyps],
}
(HERE / "case5-tree.json").write_text(json.dumps(tree, indent=2))
print(f"wrote case5-tree.json: {len(tree['hypotheses'])} hypotheses, "
      f"{sum(len(h['evidence']) for h in tree['hypotheses'])} evidence nodes, "
      f"{len(OBSERVED)} observations")
for h in tree["hypotheses"]:
    active = sum(1 for e in h["evidence"] if e.get("observed") or e.get("fires"))
    print(f"  {h['dx']:52s} prior={h['prior']}  evidence={len(h['evidence'])} (active={active})")
