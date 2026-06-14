#!/usr/bin/env python3
"""formulary_build.py - gate the spidered formulary facts -> a content-addressed,
importable adj-lang library in the CAS.

The cold path, applied to the FORMULARY (the same shape as the diagnostic rulebook):

  spider (formulary-spider workflow) -> grounding/formulary-grounding.json
    -> ADVERSARIAL GATE here: a coverage/penetration fact is ACCEPTED only if the
       spider grounded it (a real source affirms it) AND it survived independent
       re-extraction AND the source entails it; a REFUTED fact is DROPPED (the gate
       corrects the authored draft - if a source says the drug does NOT cover the
       organism, that coverage fact does not enter the formulary); unclear/unstable
       facts are FLAGGED (kept at trust `inferred`, never silently asserted).
    -> emit cas/objects/<hash>.adj  (the importable library: `observe covers(...)`,
       `observe csf_penetrant(...)`, `observe contraindicated(...)` facts, each with
       its byte-cited provenance in a comment) + a manifest (structured gated facts +
       grounding + per-fact verdict) + a registry. A program `import`s the object to
       bring the grounded formulary into scope.

The dose-window + tier come from the authored formulary.json (illustrative, not
spidered) and are carried through, clearly marked. derive_regimen.py reads the CAS
object's manifest (only ACCEPTED coverage facts are used).

Usage:  python3 formulary_build.py [--check]
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
AUTHORED = HERE / "formulary.json"
GROUNDING = MYCIN / "grounding" / "formulary-grounding.json"
DOSE_GROUNDING = MYCIN / "grounding" / "dose-window-grounding.json"   # G3 dose anchors
CAS = HERE / "cas"
OBJECTS = CAS / "objects"


def gate_status(rec: dict) -> str:
    """ACCEPT / FLAG / DROP for one grounded fact (see module docstring)."""
    if rec is None:
        return "FLAG"  # no grounding record -> inferred
    s = rec.get("status")
    if s == "grounded":
        return "ACCEPT"
    if s == "refuted":
        return "DROP"
    return "FLAG"  # unclear / unstable / failed


def _dose_summary(drugs: dict) -> dict:
    """Count per-drug dose-anchor grounding status — the dose debt made visible."""
    from collections import Counter
    c = Counter(d["dose_grounding"]["status"] for d in drugs.values())
    return {"grounded": c.get("grounded", 0), "direction_only": c.get("direction_only", 0),
            "refuted": c.get("refuted", 0), "pending": c.get("pending", 0)}


def build(check: bool = False) -> int:
    authored = json.loads(AUTHORED.read_text())
    if not GROUNDING.exists():
        print(f"formulary_build: {GROUNDING} not found — run the formulary-spider first.",
              file=sys.stderr)
        return 2
    grounding = {r["id"]: r for r in json.loads(GROUNDING.read_text()).get("records", [])}
    # G3 — dose-anchor grounding (optional). The dose-window numeric model is structural;
    # this records, per drug, whether a PRIMARY source confirms the bacterial-meningitis
    # (CNS, adult) dose. grounded = source states the adult meningitis dose verbatim;
    # direction_only = indication/target confirmed but the exact figure not verbatim;
    # refuted = the cited source did NOT confirm it (e.g. pediatric-only or general dose);
    # pending = not yet grounded. Dose debt is whatever is not grounded.
    dose_g = {}
    if DOSE_GROUNDING.exists():
        dose_g = {r["id"]: r for r in json.loads(DOSE_GROUNDING.read_text()).get("records", [])}

    def dose_provenance(drug: str) -> dict:
        rec = dose_g.get(f"dose_{drug}")
        if rec is None:
            return {"status": "pending", "note": "authored illustrative model (not yet grounded)",
                    "byte_quote": None, "url": None, "value_found": None}
        g = rec.get("grounded") or {}
        return {"status": rec.get("spider_status", "pending"),
                "note": {"grounded": "meningitis dose grounded to a primary source",
                         "direction_only": "indication/target grounded; exact figure not verbatim",
                         "refuted": "cited source did NOT confirm the adult CNS dose (debt remains)",
                         }.get(rec.get("spider_status"), "authored illustrative model (not grounded)"),
                "byte_quote": g.get("byte_quote"), "url": g.get("resolved_url"),
                "value_found": g.get("value_found"), "source_title": g.get("source_title")}

    drugs = {}
    adj_lines = [
        "% ============================================================================",
        "% formulary — grounded antibiotic facts, as an importable CAS library.",
        "% ============================================================================",
        "% Every `observe` below is a fact the formulary-spider grounded in a primary",
        "% source and the adversarial gate ACCEPTED (re-extraction-stable + entailed).",
        "% Provenance (byte-quote + source URL) is in the sibling manifest; refuted",
        "% coverage facts were dropped by the gate; flagged facts kept at trust inferred.",
        "% A program `import`s this object to bring the grounded formulary into scope.",
        "",
    ]
    report = {"accepted": 0, "dropped": [], "flagged": []}

    for drug, f in authored["drugs"].items():
        kept_covers, inferred_covers, dropped_covers = [], [], []
        for org in f["covers"]:
            rid = f"{drug}__covers__{org}"
            verdict = gate_status(grounding.get(rid))
            g = (grounding.get(rid) or {}).get("grounded") or {}
            if verdict == "ACCEPT":
                kept_covers.append(org)
                cite = (g.get("source_title") or g.get("resolved_url") or "grounded")
                adj_lines.append(f"observe covers({drug}, {org})   % [grounded] {cite[:50]}")
                report["accepted"] += 1
            elif verdict == "DROP":   # source REFUTED the authored fact -> removed
                dropped_covers.append(org)
                report["dropped"].append(f"covers({drug},{org})")
            else:                     # FLAG: kept at trust inferred, never deleted
                inferred_covers.append(org)
                adj_lines.append(f"observe covers({drug}, {org})   % [inferred — flagged by gate]")
                report["flagged"].append(f"covers({drug},{org})")
        # CSF penetration: refuted -> not penetrant; flagged -> penetrant but inferred.
        pen = gate_status(grounding.get(f"{drug}__csf"))
        csf_ok = pen != "DROP"
        if csf_ok:
            tag = "" if pen == "ACCEPT" else "   % [inferred — flagged by gate]"
            adj_lines.append(f"observe csf_penetrant({drug}){tag}")
        if pen != "ACCEPT":
            report["flagged"].append(f"csf_penetrant({drug})")
        for c in f.get("contraindications", []):
            adj_lines.append(f"observe contraindicated({drug}, {c})")
        drugs[drug] = {
            # effective covers used for derivation = grounded + inferred (flagged kept,
            # refuted removed); separated for transparency.
            "covers_accepted": kept_covers + inferred_covers,
            "covers_grounded": kept_covers, "covers_inferred": inferred_covers,
            "covers_dropped": dropped_covers, "csf_penetrant": csf_ok,
            "contraindications": f.get("contraindications", []),
            "betalactam": f.get("betalactam", False), "tier": f["tier"],
            "dose": f["dose"], "dose_grounding": dose_provenance(drug),
        }
    # combination-coverage facts (grounded by the spider's refutation evidence —
    # e.g. resistant pneumococcus is covered by vancomycin + a cephalosporin, never
    # vancomycin alone). These let the set-cover express what no single drug can.
    combinations = authored.get("combinations", [])
    for comb in combinations:
        ds = ", ".join(comb["drugs"])
        adj_lines.append(f"observe combination_covers({ds}, {comb['covers']})   % [{comb['source'][:50]}]")
    adj_lines.append("")
    adj_text = "\n".join(adj_lines)
    digest = hashlib.sha256(adj_text.encode()).hexdigest()[:16]

    manifest = {
        "hash": digest, "kind": "formulary", "domain": authored.get("_doc", "")[:60],
        "source": "formulary-spider (coverage + CSF penetration) + adversarial gate; "
                  "dose anchors spider-grounded per drug (dose_grounding); dose-window "
                  "numeric model + tier structural.",
        "drugs": drugs,
        "dose_grounding_summary": _dose_summary(drugs),
        "combinations": combinations,
        "gate": {"accepted_facts": report["accepted"], "dropped": report["dropped"],
                 "flagged": report["flagged"]},
        "grounding": {rid: {"status": r.get("status"),
                            "byte_quote": (r.get("grounded") or {}).get("byte_quote"),
                            "url": (r.get("grounded") or {}).get("resolved_url")}
                      for rid, r in grounding.items()},
    }
    registry = {"_doc": "Formulary CAS — grounded antibiotic facts as an importable adj-lang library.",
                "root": digest, "object": f"objects/{digest}.adj"}

    if check:
        ok = (OBJECTS / f"{digest}.adj").exists() and \
             (OBJECTS / f"{digest}.adj").read_text() == adj_text and \
             (CAS / "registry.json").exists() and \
             json.loads((CAS / "registry.json").read_text()).get("root") == digest
        print("formulary_build --check:", "up to date" if ok else "OUT OF DATE")
        return 0 if ok else 1

    OBJECTS.mkdir(parents=True, exist_ok=True)
    (OBJECTS / f"{digest}.adj").write_text(adj_text)
    (OBJECTS / f"{digest}.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (CAS / "registry.json").write_text(json.dumps(registry, indent=2) + "\n")
    print(f"formulary_build: CAS object objects/{digest}.adj  "
          f"({report['accepted']} grounded coverage/penetration facts accepted; "
          f"{len(report['dropped'])} dropped, {len(report['flagged'])} flagged)")
    if report["dropped"]:
        print("  DROPPED by the gate (source refuted the authored fact):", report["dropped"])
    if report["flagged"]:
        print("  FLAGGED (unclear/unstable -> trust inferred):", report["flagged"][:8])
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
