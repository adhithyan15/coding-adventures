#!/usr/bin/env python3
"""ADJ57 full-run verdict — the deterministic answer at the end of the byte-
provenance pipeline.

From a full-run workflow result it: (L1) checks the case partition reconstructs
the input byte-for-byte; (L0/L3) interns the prior + every finding source into the
CAS with byte-anchored citation spans; then computes the posterior for the leading
diagnosis as a sequential Bayesian update where EVERY step prints the CAS source
it came from. Ungrounded numbers (direction_only / fabricated) do not move the
posterior — the framework abstains rather than invent.

Run: python verdict.py <full-results.json>
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cas  # noqa: E402
import coverage  # noqa: E402

HERE = Path(__file__).resolve().parent.parent


def sigmoid(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))


def intern_chain(chain: list, used_for: str) -> list:
    """Intern every hop's source into the CAS and byte-provenance its quote."""
    prov = []
    for hop in chain:
        content = hop.get("content_excerpt", "")
        if not content:
            prov.append({"hop": hop.get("hop"), "error": "no content_excerpt"})
            continue
        h = cas.find_by_url(hop.get("source_url", "")) or cas.intern(
            content, url=hop.get("source_url", ""), title=hop.get("source_title", ""))
        try:
            span = cas.cite(h, hop.get("cited_quote", ""), used_for=used_for)
            prov.append({"hop": hop.get("hop"), "cas": h[:12], "span": [span["start"], span["end"]],
                         "root": hop.get("gives_root_data", False), "url": hop.get("source_url", ""),
                         "quote": hop.get("cited_quote", "")[:110]})
        except ValueError as e:
            prov.append({"hop": hop.get("hop"), "cas": h[:12], "error": str(e)[:80]})
        if hop.get("onward_citation"):
            cas.add_onward(h, hop["onward_citation"])
    return prov


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    ingest, derived, spidered = res["ingest"], res["derived"], res["spidered"]
    lead = derived["leading_diagnosis"]

    print("=" * 72)
    print(f"  ADJ57 FULL RUN — leading diagnosis: {lead}")
    print(f"  case source: {ingest.get('source_url','')}")
    print("=" * 72)

    print("\n## L1 — case -> IR byte coverage")
    cov = coverage.check(ingest["case_text"], ingest["segments"])
    if cov["covered"]:
        print(f"   TOTAL COVERAGE: {cov['facts']} typed facts ({cov['pct_in_facts']}%) + "
              f"{cov['discards']} reasoned discards ({cov['pct_discarded']}%) = 100% of {cov['total_bytes']} bytes."
              f"  clean={cov['clean']}")
    else:
        print(f"   COVERAGE VIOLATION at byte {cov['first_divergence_offset']}: "
              f"expects {cov['expected_next']!r}, got {cov['got_next']!r}")

    print("\n## L0/L3 — intern sources, byte-provenance to root, then VERDICT")
    prior = spidered["prior"]
    prior_prov = intern_chain(prior.get("chain", []), used_for=f"prior:{lead}")
    p0 = prior.get("value", 0)
    if prior.get("verdict") != "grounded" or not (0 < p0 < 1):
        print(f"   prior NOT grounded (verdict={prior.get('verdict')}, value={p0}) — cannot compute a grounded posterior.")
        return
    logodds = math.log(p0 / (1 - p0))
    root = next((p for p in reversed(prior_prov) if p.get("root")), prior_prov[-1] if prior_prov else {})
    print(f"\n   prior P({lead}) = {p0:.3f}   [grounded]  CAS {root.get('cas','?')} bytes {root.get('span')}")
    print(f"     \"{root.get('quote','')}\"")

    used, gaps = [], []
    for f in spidered["grounded_findings"]:
        prov = intern_chain(f.get("chain", []), used_for=f"{f['finding']}->{lead}")
        if f.get("verdict") != "grounded" or not f.get("computed_lr"):
            gaps.append((f["finding"], f.get("verdict"), prov))
            continue
        lr = f["computed_lr"]
        logodds += math.log(lr)
        r = next((p for p in reversed(prov) if p.get("root")), prov[-1] if prov else {})
        used.append((f["finding"], lr, sigmoid(logodds), r))
        print(f"   x LR {lr:<6} ({f['finding']:42s}) -> P = {sigmoid(logodds):.3f}  "
              f"[grounded] CAS {r.get('cas','?')} {r.get('span')}")

    print(f"\n   >>> VERDICT: P({lead}) = {sigmoid(logodds):.4f}  "
          f"(from {len(used)} byte-grounded findings x grounded prior)")
    if gaps:
        print("\n   data-gaps (did NOT move the posterior — no root number, framework abstains):")
        for fn, v, _ in gaps:
            print(f"     - {fn}: {v}")
    print(f"\n   ground truth (held aside from the pipeline): {ingest.get('ground_truth','')[:200]}")
    print(f"   CAS now holds: {json.dumps(cas.stats())}")

    out = {"leading_diagnosis": lead, "posterior": sigmoid(logodds), "prior": p0,
           "coverage": {k: cov.get(k) for k in ("covered", "clean", "facts", "discards", "total_bytes")},
           "used_findings": [{"finding": f, "lr": lr} for f, lr, _, _ in used],
           "data_gaps": [{"finding": fn, "verdict": v} for fn, v, _ in gaps]}
    (HERE / "verdict.json").write_text(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
