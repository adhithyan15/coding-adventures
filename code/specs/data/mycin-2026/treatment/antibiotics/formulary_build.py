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


def build(check: bool = False) -> int:
    authored = json.loads(AUTHORED.read_text())
    if not GROUNDING.exists():
        print(f"formulary_build: {GROUNDING} not found — run the formulary-spider first.",
              file=sys.stderr)
        return 2
    grounding = {r["id"]: r for r in json.loads(GROUNDING.read_text()).get("records", [])}

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
        kept_covers, dropped_covers, flagged_covers = [], [], []
        for org in f["covers"]:
            rid = f"{drug}__covers__{org}"
            verdict = gate_status(grounding.get(rid))
            g = (grounding.get(rid) or {}).get("grounded") or {}
            if verdict == "ACCEPT":
                kept_covers.append(org)
                cite = (g.get("source_title") or g.get("resolved_url") or "grounded")
                adj_lines.append(f"observe covers({drug}, {org})   % [{cite}]")
                report["accepted"] += 1
            elif verdict == "DROP":
                dropped_covers.append(org)
                report["dropped"].append(f"covers({drug},{org})")
            else:
                flagged_covers.append(org)
                report["flagged"].append(f"covers({drug},{org})")
        # CSF penetration
        pen = gate_status(grounding.get(f"{drug}__csf"))
        if pen == "ACCEPT":
            adj_lines.append(f"observe csf_penetrant({drug})")
        elif pen != "ACCEPT":
            report["flagged"].append(f"csf_penetrant({drug})")
        # contraindications (authored; carried through)
        for c in f.get("contraindications", []):
            adj_lines.append(f"observe contraindicated({drug}, {c})")
        drugs[drug] = {
            "covers_accepted": kept_covers, "covers_dropped": dropped_covers,
            "covers_flagged": flagged_covers, "csf_penetrant": pen == "ACCEPT",
            "contraindications": f.get("contraindications", []),
            "betalactam": f.get("betalactam", False), "tier": f["tier"],
            "dose": f["dose"], "dose_note": "authored illustrative model (not spidered)",
        }
    adj_lines.append("")
    adj_text = "\n".join(adj_lines)
    digest = hashlib.sha256(adj_text.encode()).hexdigest()[:16]

    manifest = {
        "hash": digest, "kind": "formulary", "domain": authored.get("_doc", "")[:60],
        "source": "formulary-spider (coverage + CSF penetration) + adversarial gate; "
                  "dose/tier authored (illustrative).",
        "drugs": drugs,
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
