#!/usr/bin/env python3
"""CAS human-override layer — "fix the fact, not the weight."

The corpus (CAS) is assembled from decomposed sources; every finding node is a
content-addressed claim with a likelihood ratio and a byte-provenance chain. When
the auditable trail localizes an error to a specific node, a human edits THAT node
here — a local, versioned, attributed override — rather than retraining anything.

The base corpus stays immutable. Overrides are an additive human layer (this file
+ the override JSON, both in git, so every edit has history). Applying them
produces an EFFECTIVE corpus that the evaluator runs on; each touched node records
its prior value, who changed it, when, why, and the citation — so the edit itself
is auditable, and `eval.py`'s trail shows "[human override]" at the exact step.

Override op schema (a JSON list):
  { "target_id": "f3", "set": {"lr": 1.0},            # edit an existing claim
    "editor": "...", "date": "...", "reason": "...", "source": "..." }
  { "add_finding": { ...full node... },                # add a missing claim
    "editor": "...", "date": "...", "reason": "...", "source": "..." }

Run: python override.py <base-corpus.json> <overrides.json> <effective-out.json>
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any


def apply_overrides(base: dict[str, Any], overrides: list[dict[str, Any]]) -> tuple[dict[str, Any], list[str]]:
    eff = copy.deepcopy(base)
    by_id: dict[str, dict[str, Any]] = {n["id"]: n for n in eff["findings"]}
    if "id" in eff.get("prior", {}):
        by_id[eff["prior"]["id"]] = eff["prior"]
    audit: list[str] = []

    for ov in overrides:
        meta = {
            "editor": ov.get("editor", "?"),
            "date": ov.get("date", "?"),
            "reason": ov.get("reason", ""),
            "source": ov.get("source", ""),
        }
        if "add_finding" in ov:
            node = copy.deepcopy(ov["add_finding"])
            node.setdefault("provenance", {})
            node["provenance"]["override"] = {**meta, "op": "added"}
            node.setdefault("grounded", True)
            eff["findings"].append(node)
            audit.append(f"ADD  {node['id']:4s} {node['finding']}({node.get('state','')}) lr={node['lr']}  by {meta['editor']}: {meta['reason'][:70]}")
            continue

        tid = ov["target_id"]
        node = by_id.get(tid)
        if node is None:
            audit.append(f"SKIP {tid}: no such node")
            continue
        changes = ov.get("set", {})
        prior_vals = {k: node.get(k) for k in changes}
        node.update(changes)
        node.setdefault("provenance", {})
        node["provenance"]["override"] = {**meta, "op": "edited", "prior": prior_vals, "now": changes}
        chg = ", ".join(f"{k}: {prior_vals[k]} -> {changes[k]}" for k in changes)
        audit.append(f"EDIT {tid:4s} {node['finding']}({node.get('state','')})  {chg}  by {meta['editor']}: {meta['reason'][:70]}")

    return eff, audit


def main() -> None:
    if len(sys.argv) < 4:
        print("usage: python override.py <base-corpus.json> <overrides.json> <effective-out.json>")
        sys.exit(2)
    base = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    overrides = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    eff, audit = apply_overrides(base, overrides)
    eff["derived_from"] = sys.argv[1]
    eff["human_overrides"] = sys.argv[2]
    Path(sys.argv[3]).write_text(json.dumps(eff, indent=2), encoding="utf-8")
    print(f"applied {len(overrides)} override(s) -> {sys.argv[3]}")
    for line in audit:
        print("  " + line)


if __name__ == "__main__":
    main()
