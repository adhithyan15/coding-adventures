#!/usr/bin/env python3
"""Generic, domain-agnostic corpus assembler.

Takes a forward-grounding workflow result + a finding skeleton (id -> finding/state
labels) and writes the canonical `corpus/<domain>/corpus.json` + `SOURCES.md`. Every
admitted LR carries its byte-anchored provenance; ungroundable links stay as explicit
data-gaps. Same format as the hand-built PE corpus — see ../README.md.

Run: python build.py <domain> <grounding-results.json> <skeleton.json>
  e.g. python build.py streptococcal_pharyngitis ../provenance/strep/grounding-results.json ../provenance/strep/findings.json
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent  # .../adj52/corpus


def main() -> None:
    domain, results_path, skeleton_path = sys.argv[1], sys.argv[2], sys.argv[3]
    res = json.loads(Path(results_path).read_text())
    per = res.get("per_finding", res.get("result", {}).get("per_finding", []))
    target = res.get("target", res.get("result", {}).get("target", f"diagnosis({domain})"))
    skel = {f["id"]: f for f in json.loads(Path(skeleton_path).read_text())}

    prior, findings = None, []
    for r in per:
        s = skel.get(r["id"], {})
        node = {
            "id": r["id"],
            "finding": s.get("finding", ""),
            "state": s.get("state", ""),
            "lr": r.get("computed_lr", 0) or 0,
            "verdict": r.get("verdict", "unknown"),
            "grounded": r.get("verdict") == "grounded",
            "provenance": {
                "study": (r.get("primary_data") or {}).get("study", ""),
                "values": (r.get("primary_data") or {}).get("values", ""),
                "byte_quote": (r.get("primary_data") or {}).get("byte_quote", ""),
                "formula": r.get("lr_formula", ""),
                "n": (r.get("primary_data") or {}).get("n", ""),
                "population": (r.get("primary_data") or {}).get("population", ""),
            },
            "note": r.get("note", ""),
        }
        if r["id"] == "f0":
            prior = node
        else:
            findings.append(node)

    corpus = {
        "domain": domain,
        "target": target,
        "built": "case-blind, provenance-first (forward grounding)",
        "invariant": "every LR carries a byte-anchored provenance chain to primary data; ungroundable links are explicit data-gaps, never invented numbers",
        "prior": prior,
        "findings": findings,
    }
    out_dir = HERE / domain
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "corpus.json").write_text(json.dumps(corpus, indent=2))

    # SOURCES.md
    g = sum(1 for n in findings if n["grounded"])
    rows = ["# %s — grounded corpus provenance" % domain.replace("_", " "),
            "",
            "Forward byte-provenance crawl to primary data. **%d/%d finding LRs grounded** "
            "(prior %s)." % (g, len(findings), "grounded" if (prior and prior["grounded"]) else "ungrounded"),
            "", "| finding | LR | formula | primary source | verdict |", "|---|---|---|---|---|"]
    for n in ([prior] if prior else []) + findings:
        fs = f"{n['finding']}({n['state']})" if n.get("state") else n.get("finding", "")
        rows.append(f"| {fs} | {n['lr']} | {n['provenance'].get('formula','')[:55]} | "
                    f"{n['provenance']['study'][:60]} | {n['verdict']} |")
    (out_dir / "SOURCES.md").write_text("\n".join(rows) + "\n")

    print(f"{domain}: prior {'GROUNDED' if (prior and prior['grounded']) else 'ungrounded'}="
          f"{prior['lr'] if prior else '?'}; {g}/{len(findings)} finding LRs grounded -> corpus/{domain}/")
    for n in ([prior] if prior else []) + findings:
        fs = f"{n['finding']}({n['state']})" if n.get("state") else n.get("finding", "")
        print(f"  {n['id']:4s} {fs:42s} LR={n['lr']!s:8} [{n['verdict']}]")


if __name__ == "__main__":
    main()
