#!/usr/bin/env python3
"""ADJ58 — full run under the universal stage contract.

Drives a full-run workflow result through EVERY stage's coverage gate, composing
one auditable Trail, interning sources into the CAS, and computing the verdict.
A stage that fails to account for 100% of its input shows up as a HOLE in the
trail — the framework no longer claims auditability it doesn't have.

Stages gated here:
  decompose   case_text (bytes)        -> typed facts + reasoned discards
  derive      facts (elements)         -> used (link/prior) + discarded-with-reason
  ground:N    each source page (bytes) -> cited spans + discarded context
  aggregate   grounded LRs (elements)  -> used in posterior + abstained-with-reason

Run: python run.py <full-results.json>
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cas  # noqa: E402
import stage  # noqa: E402

HERE = Path(__file__).resolve().parent.parent


def sigmoid(x: float) -> float:
    return 1.0 / (1.0 + math.exp(-x))


def intern_chain(chain: list, used_for: str) -> list:
    prov = []
    for hop in chain:
        content = hop.get("content_excerpt", "")
        if not content:
            continue
        h = cas.find_by_url(hop.get("source_url", "")) or cas.intern(
            content, url=hop.get("source_url", ""), title=hop.get("source_title", ""))
        try:
            sp = cas.cite(h, hop.get("cited_quote", ""), used_for=used_for)
            prov.append({"hop": hop.get("hop"), "cas": h[:12], "span": [sp["start"], sp["end"]],
                         "root": hop.get("gives_root_data", False), "quote": hop.get("cited_quote", "")})
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
    trail = stage.Trail()

    print("=" * 74)
    print(f"  ADJ58 FULL RUN under the universal stage contract — leading dx: {lead}")
    print(f"  case: {ingest.get('source_url','')}")
    print("=" * 74)

    # ---- STAGE: decompose (case bytes -> facts) ----
    segs = [
        ({"text": s["text"], "kind": "used", "produced": s.get("term")} if s.get("kind") == "fact"
         else {"text": s["text"], "kind": "discard", "reason": s.get("reason")})
        for s in ingest["segments"]
    ]
    trail.record(stage.gate_text("decompose", ingest["case_text"], segs))
    fact_terms = [s["term"] for s in ingest["segments"] if s.get("kind") == "fact"]

    # ---- STAGE: derive (facts -> used/discarded) ----
    disp = derived.get("fact_dispositions", [])
    used = [{"id": d["fact"], "produced": d.get("role") or "used"} for d in disp if d.get("used")]
    disc = [{"id": d["fact"], "reason": d.get("reason")} for d in disp if not d.get("used")]
    trail.record(stage.gate_elements("derive", fact_terms, used, disc))

    # ---- STAGE(s): ground (each source page -> cited spans + discarded context) ----
    prior = spidered["prior"]
    intern_chain(prior.get("chain", []), used_for=f"prior:{lead}")
    DISCARD_CTX = "surrounding source context; the load-bearing figure is the cited span"
    for i, hop in enumerate(prior.get("chain", [])):
        ce = hop.get("content_excerpt", "")
        if ce:
            p = stage.partition_text_by_used(ce, [{"text": hop.get("cited_quote", ""), "produced": f"prior P({lead})"}], DISCARD_CTX)
            trail.record(stage.gate_text("ground:prior", ce, p))
    for gf in spidered["grounded_findings"]:
        intern_chain(gf.get("chain", []), used_for=f"{gf['finding']}->{lead}")
        for hop in gf.get("chain", []):
            ce = hop.get("content_excerpt", "")
            if ce:
                p = stage.partition_text_by_used(ce, [{"text": hop.get("cited_quote", ""), "produced": f"LR {gf.get('finding')}"}], DISCARD_CTX)
                trail.record(stage.gate_text(f"ground:{gf['finding'][:18]}", ce, p))

    # ---- STAGE: aggregate (grounded LRs -> posterior, abstain-with-reason) ----
    p0 = prior.get("value", 0)
    agg_input = [f"prior:{lead}"] + [gf["finding"] for gf in spidered["grounded_findings"]]
    agg_used, agg_disc = [], []
    logodds, used_steps = None, []
    if prior.get("verdict") == "grounded" and 0 < p0 < 1:
        logodds = math.log(p0 / (1 - p0))
        agg_used.append({"id": f"prior:{lead}", "produced": f"P0={p0}"})
    else:
        agg_disc.append({"id": f"prior:{lead}", "reason": f"prior not grounded ({prior.get('verdict')})"})
    for gf in spidered["grounded_findings"]:
        if gf.get("verdict") == "grounded" and gf.get("computed_lr") and logodds is not None:
            logodds += math.log(gf["computed_lr"])
            used_steps.append((gf["finding"], gf["computed_lr"], sigmoid(logodds)))
            agg_used.append({"id": gf["finding"], "produced": f"xLR {gf['computed_lr']}"})
        else:
            agg_disc.append({"id": gf["finding"], "reason": f"{gf.get('verdict','?')}: no root LR — abstained"})
    trail.record(stage.gate_elements("aggregate", agg_input, agg_used, agg_disc))

    # ---- THE TRAIL + THE VERDICT ----
    print("\n" + trail.summary())

    # The verdict reports the STRONGEST defensible conclusion available, never less.
    # QUANTITATIVE when a grounded prior + >=1 root-grounded finding give a posterior.
    # Otherwise QUALITATIVE: commit to the derive-stage leading hypothesis and present
    # its byte-provenanced evidential basis (each used fact traces to case bytes via
    # decompose) — an auditable ANSWER, explicitly marked as having no quantified
    # posterior because the domain publishes no usable likelihood ratios. The byte-
    # provenance invariant holds either way; we just stop requiring a NUMBER to count
    # as an answer (the abstain-on-no-number policy lost 0-3 outside medicine).
    quantitative = (logodds is not None) and (len(used_steps) >= 1)
    gf = spidered["grounded_findings"]
    n_grounded = sum(1 for x in gf if x.get("verdict") == "grounded")
    posterior = sigmoid(logodds) if logodds is not None else None

    print("\n## Verdict")
    if quantitative:
        print("   mode: QUANTITATIVE (grounded prior x root-grounded likelihood ratios)")
        print(f"   prior P({lead}) = {p0:.3f}")
        for fn, lr, p in used_steps:
            print(f"   x LR {lr:<6} ({fn:40s}) -> P = {p:.3f}")
        print(f"\n   >>> P({lead}) = {posterior:.4f}   "
              f"({len(used_steps)} byte-grounded findings; {len(agg_disc)} abstained)")
    else:
        print("   mode: QUALITATIVE (no quantified posterior groundable in this domain —")
        print("         the answer is the byte-provenanced evidential conclusion from the IR)")
        print(f"\n   >>> LEADING ANSWER: {lead}")
        print("   evidential basis (each fact traces to case bytes via the decompose stage):")
        for d in derived.get("fact_dispositions", []):
            if d.get("used"):
                print(f"     - {d['fact'][:44]:44s} -> {(d.get('role') or '')[:74]}")
        print(f"   grounding status: prior '{prior.get('verdict')}'; "
              f"{n_grounded}/{len(gf)} findings reached root data "
              f"(rest direction_only — no published likelihood ratio).")

    print(f"\n   ground truth (held aside): {ingest.get('ground_truth','')[:180]}")
    print(f"   CAS: {json.dumps(cas.stats())}")
    print(f"\n   >>> AUDIT TRAIL {'UNBROKEN — every stage accounted for 100% of its input' if trail.ok() else 'HAS HOLES (see above)'}")

    out = {"leading_answer": lead, "verdict_mode": "quantitative" if quantitative else "qualitative",
           "posterior": posterior if quantitative else None,
           "trail_ok": trail.ok(), "holes": trail.holes(),
           "trail": json.loads(trail.to_json()),
           "verdict_steps": [{"finding": f, "lr": lr} for f, lr, _ in used_steps],
           "evidential_basis": [{"fact": d["fact"], "role": d.get("role")}
                                for d in derived.get("fact_dispositions", []) if d.get("used")]}
    (HERE / "run.json").write_text(json.dumps(out, indent=2))
    sys.exit(0 if trail.ok() else 3)


if __name__ == "__main__":
    main()
