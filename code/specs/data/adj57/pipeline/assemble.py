#!/usr/bin/env python3
"""ADJ57 framework driver — ties the four layers together from a slice-workflow
result and enforces the byte-provenance invariant at each.

  L1  coverage: assert the case partition reconstructs the input exactly
                (every byte represented or discarded-with-reason).
  L0  CAS:      intern every source the spider read, content-addressed.
  L3  grounding: for each grounded link, record the byte-provenanced citation
                span IN the interned source (cite() rejects any quote that is not
                literally present — a citation must point at real bytes), and the
                onward edge toward the root source.

Output: a byte-addressable rulebook where every magnitude points at a CAS span,
plus a coverage/provenance report.

Run: python assemble.py <slice-results.json>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cas  # noqa: E402
import coverage  # noqa: E402

HERE = Path(__file__).resolve().parent.parent  # .../adj57


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    ingest, spidered = res["ingest"], res["spidered"]

    print("=" * 70)
    print("L1 — case -> IR byte coverage")
    print("=" * 70)
    cov = coverage.check(ingest["case_text"], ingest["segments"])
    print(json.dumps({k: v for k, v in cov.items()}, indent=2))
    if not cov["covered"]:
        print(f"\n>>> COVERAGE VIOLATION at byte {cov['first_divergence_offset']}: "
              f"case expects {cov['expected_next']!r}, partition gave {cov['got_next']!r}")
    else:
        print(f"\n>>> TOTAL COVERAGE: {cov['facts']} facts ({cov['pct_in_facts']}%) + "
              f"{cov['discards']} discards ({cov['pct_discarded']}%) = 100% of {cov['total_bytes']} bytes.")

    print("\n" + "=" * 70)
    print("L0+L3 — intern sources into CAS, byte-provenance each citation to root")
    print("=" * 70)
    rulebook_nodes = []
    for link in spidered["grounded_links"]:
        prov_chain = []
        for hop in link["chain"]:
            content = hop.get("content_excerpt", "")
            if not content:
                prov_chain.append({"hop": hop["hop"], "error": "no content_excerpt — source not retrievable"})
                continue
            # reuse if we already have this exact source content
            h = cas.find_by_url(hop.get("source_url", "")) or cas.intern(
                content, url=hop.get("source_url", ""), title=hop.get("source_title", ""))
            cited = hop.get("cited_quote", "")
            try:
                span = cas.cite(h, cited, used_for=f"{link['finding']}->{link['target_diagnosis']}")
                prov_chain.append({"hop": hop["hop"], "cas": h[:16], "span": [span["start"], span["end"]],
                                   "root": hop.get("gives_root_data", False), "url": hop.get("source_url", "")})
            except ValueError as e:
                prov_chain.append({"hop": hop["hop"], "cas": h[:16],
                                   "error": f"citation not verbatim in source: {str(e)[:80]}"})
            if hop.get("onward_citation"):
                cas.add_onward(h, hop["onward_citation"])
        rulebook_nodes.append({
            "finding": link["finding"], "target": link["target_diagnosis"],
            "lr": link["computed_lr"], "verdict": link["verdict"],
            "reached_root": link.get("reached_root", False),
            "lr_formula": link.get("lr_formula", ""),
            "provenance": prov_chain,
        })
        ok = sum(1 for p in prov_chain if "error" not in p)
        print(f"  {link['finding']} -> {link['target_diagnosis']}: LR={link.get('computed_lr')} "
              f"[{link['verdict']}] chain {len(prov_chain)} hops, {ok} byte-anchored, "
              f"root={link.get('reached_root')}")

    rulebook = {
        "case_source": ingest.get("source_url", ""),
        "candidate_diagnoses": ingest.get("candidate_diagnoses", []),
        "coverage": {k: cov.get(k) for k in ("covered", "clean", "facts", "discards", "total_bytes", "pct_in_facts")},
        "nodes": rulebook_nodes,
    }
    (HERE / "rulebook.json").write_text(json.dumps(rulebook, indent=2))
    print(f"\n>>> wrote rulebook.json; CAS: {json.dumps(cas.stats())}")


if __name__ == "__main__":
    main()
