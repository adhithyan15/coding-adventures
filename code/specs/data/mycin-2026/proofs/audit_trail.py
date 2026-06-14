#!/usr/bin/env python3
"""audit_trail.py - render the proof DAG for a case: every step cited to a source.

MYCIN-2026 M8 (audit-trail + error-localization proofs). The diagnosis is not a
black-box number: each contribution in the differential carries the rulebook
clause's source + trust tier, and each rulebook clause traces (via
grounding/grounding-results.json) to a verbatim primary-source byte-quote. This
renders that chain for one case, so a reviewer can follow the diagnosis from the
verdict down to the bytes - and, on a wrong verdict, localize it to the one
contributing clause.

Usage:  python3 audit_trail.py [case_id]   (default: case_bacterial_culture)
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

IR_DIR = ROOT / "ir"
GROUNDING = ROOT / "grounding" / "grounding-results.json"


def byte_quote_index() -> dict:
    """Map 'functor(value)' / 'prior_<arm>' -> the spider's primary-source quote."""
    idx = {}
    for rec in json.loads(GROUNDING.read_text()).get("records", []):
        g = rec.get("grounded") or {}
        idx[rec["id"]] = {"url": g.get("resolved_url"), "quote": g.get("byte_quote")}
    return idx


def gid(evidence: str) -> str:
    # 'csf_gram_stain(positive)' -> 'csf_gram_stain_positive'
    return evidence.replace("(", "_").replace(")", "")


def main(argv: list[str]) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("audit_trail: SKIPPED (adj-lang-cli not built)")
        return 0
    case_id = argv[0] if argv else "case_bacterial_culture"
    ir = json.loads((IR_DIR / f"{case_id}.json").read_text())
    obs, kept, dropped = ir_mod.ir_to_adj(ir, ir_mod.load_domains())

    linked = decide_mod.CAS / f"{case_id}.linked.adj"
    linked.write_text(f'import "objects/{decide_mod.root_hash()}.adj"\n{obs}')
    try:
        r = subprocess.run([str(cli), str(linked)], capture_output=True, text=True)
        out = json.loads(r.stdout)
    finally:
        linked.unlink(missing_ok=True)

    quotes = byte_quote_index()
    print(f"AUDIT TRAIL - {case_id}")
    print(f"  observed (from decompose, vocab-gated): {', '.join(kept)}")
    if dropped:
        print(f"  dropped at the vocabulary/adversarial gate: {[d['term'] for d in dropped]}")
    dec = out.get("decision", {})
    print(f"  VERDICT: {dec.get('leader', '(see ranking)')}  [{dec.get('type')}]\n")
    for r_ in out.get("ranked", []):
        print(f"  {r_['hypothesis']}  posterior={r_['posterior']:.4f}")
        for s in r_.get("proof", []):
            kind = s.get("kind")
            ev = s.get("evidence", "(prior)")
            src = s.get("source", "")
            trust = s.get("trust", "")
            key = "prior_bacterial" if (kind == "prior" and "bacterial" in r_["hypothesis"]) else \
                  "prior_viral" if kind == "prior" else gid(ev)
            q = quotes.get(key, {})
            line = f"    - {kind:12s} {ev:34s} logit={s.get('logit', 0):+.3f}  [{trust}] {src}"
            print(line)
            if q.get("quote"):
                snippet = q["quote"][:110].replace("\n", " ")
                print(f"        ↳ bytes: \"{snippet}...\"  ({q.get('url')})")
        print()
    print("error-localization: a wrong verdict is one wrong contribution line above; "
          "edit that clause in the CAS and re-derive (see cost_to_correct.py).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
