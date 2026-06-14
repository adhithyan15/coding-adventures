#!/usr/bin/env python3
"""ground_sources.py — decompose cited SOURCES into the CAS + verify every citation.

The recursion step of "nothing on blind trust": a grounded fact cites a source with
a byte-quote. This driver takes the sources the facts cite, commits each DECOMPOSED
source (output of the decompose-source spider) to the CAS as a source object, and
then VERIFIES every fact's byte-quote against its source object — does the source
actually contain what the fact implies? A citation whose quote is NOT in the
decomposed source is flagged 'unverified' (to be fixed up). Finally it regenerates
the system-wide provenance ledger.

Flow:
  1. `--list`  → emit grounding/source-list.json: the unique sources the grounded
                 organism-id facts cite (url + title), for the spider to decompose.
  2. (default) → read grounding/source-objects.json (the spider's output), write each
                 to the CAS (cas/sources/<hash>.json), verify each grounding record's
                 citation against its source object, and rebuild PROVENANCE-LEDGER.md.

Usage:  python3 ground_sources.py --list      # produce the source worklist for the spider
        python3 ground_sources.py             # commit sources + verify + ledger
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
import harness  # noqa: E402

ORG_GROUNDING = HERE / "organism-id-grounding.json"
HOST_GROUNDING = HERE / "host-factor-grounding.json"
SOURCE_LIST = HERE / "source-list.json"
SOURCE_OBJECTS = HERE / "source-objects.json"
LEDGER = MYCIN / "PROVENANCE-LEDGER.md"
ORG_MANIFEST = MYCIN / "diagnosis" / "organisms" / "organism-id-manifest.json"
DOSE_GROUNDING = HERE / "dose-window-grounding.json"
FORMULARY_REGISTRY = MYCIN / "treatment" / "antibiotics" / "cas" / "registry.json"
UTI_GROUNDING = HERE / "uti-id-grounding.json"                       # G4 — new specialty
UTI_MANIFEST = MYCIN / "diagnosis" / "uti" / "uti-id-manifest.json"
TC_GROUNDING = HERE / "treatment-constraints-grounding.json"         # CC-3 — contraindication rules
TC_MANIFEST = MYCIN / "treatment" / "constraints" / "treatment-constraints.json"
BSI_GROUNDING = HERE / "bsi-prior-grounding.json"                    # G5 — bacteremia priors
BSI_MANIFEST = MYCIN / "diagnosis" / "bacteremia" / "bsi-prior-manifest.json"

# Every grounding file whose records cite sources we decompose + verify against.
GROUNDING_FILES = [ORG_GROUNDING, HOST_GROUNDING, DOSE_GROUNDING, UTI_GROUNDING,
                   TC_GROUNDING, BSI_GROUNDING]


def _records(*files: Path) -> list[dict]:
    """Grounding records (with a cited source) from the given files."""
    out: list[dict] = []
    for f in files:
        if f.exists():
            out += [r for r in json.loads(f.read_text())["records"]
                    if (r.get("grounded") or {}).get("resolved_url")]
    return out


def _verify_rows(records: list[dict], by_url: dict[str, str]) -> tuple[list, dict]:
    """Verify each record's citation against its decomposed source; return (rows, counts)."""
    rows, c = [], {"verified": 0, "partial": 0, "unverified": 0, "nosrc": 0}
    for r in records:
        g = r["grounded"]
        h = by_url.get(g["resolved_url"])
        so = harness.load_source_object(h) if h else None
        if so is None:
            rows.append((r["id"], r["spider_status"], "—", g.get("source_title", "")[:36], "no-source-obj"))
            c["nosrc"] += 1
            continue
        v = harness.verify_citation(g["byte_quote"], so)
        m, t = v["fragments_matched"], v["fragments_total"]
        if v["verified"]:
            c["verified"] += 1
            mark = f"✓ verified ({m}/{t})"
        elif v["core_verified"]:
            c["partial"] += 1
            mark = f"◑ core ✓ ({m}/{t} spans; over-reach)"
        else:
            c["unverified"] += 1
            mark = f"✗ UNVERIFIED ({m}/{t})"
        rows.append((r["id"], r["spider_status"], harness.gate(r["spider_status"])[0], f"src:{h}", mark))
    return rows, c


def grounded_records() -> list[dict]:
    """All grounding records (priors/morphology + host factors) that cite a real
    source (verdict-bearing) — the facts whose citations we verify against sources."""
    out: list[dict] = []
    for f in GROUNDING_FILES:
        if not f.exists():
            continue
        for r in json.loads(f.read_text())["records"]:
            if (r.get("grounded") or {}).get("resolved_url"):
                out.append(r)
    return out


def emit_source_list() -> int:
    """Unique sources the grounded facts cite — the worklist for the decompose spider."""
    seen, items = set(), []
    for r in grounded_records():
        g = r["grounded"]
        url = g["resolved_url"]
        if url in seen:
            continue
        seen.add(url)
        items.append({"source_id": url, "resolved_url": url,
                      "title": g.get("source_title", "")[:120]})
    SOURCE_LIST.write_text(json.dumps({"sources": items}, indent=2, ensure_ascii=False) + "\n")
    print(f"ground_sources --list: {len(items)} unique sources -> {SOURCE_LIST.name}")
    return 0


def commit_and_verify() -> int:
    if not SOURCE_OBJECTS.exists():
        print(f"ground_sources: {SOURCE_OBJECTS} not found — run the decompose-source "
              "spider first (see grounding/workflows/decompose-source.workflow.js).",
              file=sys.stderr)
        return 2
    objs = json.loads(SOURCE_OBJECTS.read_text())
    objs = objs.get("result", objs) if isinstance(objs, dict) else objs

    # 1. Commit each decomposed source to the CAS, indexed by its source_id (URL).
    by_url: dict[str, str] = {}
    for rec in objs:
        so = harness.source_object_from_record(rec)
        if not so.claims:
            continue
        h = harness.write_source_object(so)
        by_url[so.resolved_url] = h
        by_url[so.source_id] = h
    print(f"ground_sources: committed {len(set(by_url.values()))} source objects to cas/sources/")

    # 2. Verify each fact's citation against its decomposed source (no blind trust),
    #    grouped into one artifact per domain.
    artifacts, tot = [], {"verified": 0, "partial": 0, "unverified": 0, "nosrc": 0}

    # 2a. Organism identification (priors + morphology + host factors).
    org_rows, c = _verify_rows(_records(ORG_GROUNDING, HOST_GROUNDING), by_url)
    for k in tot:
        tot[k] += c[k]
    man = json.loads(ORG_MANIFEST.read_text()) if ORG_MANIFEST.exists() else {"clauses": {}}
    artifacts.append({
        "name": "organism identification", "path": "diagnosis/organisms/organism-id.adj",
        "grounded": sum(1 for x in man["clauses"].values() if x["verdict"] == "ACCEPT"),
        "flagged": sum(1 for x in man["clauses"].values() if x["verdict"] == "FLAG"),
        # debt = clauses carried without grounding (verdict PENDING) — computed, not hardcoded.
        "authored_debt": sum(1 for x in man["clauses"].values() if x["verdict"] == "PENDING"),
        "rows": org_rows})

    # 2b. Meningitis dosing (G3) — dose anchors. grounded = primary source confirms the
    #     adult CNS dose; direction_only → flagged; refuted/pending → debt (needs a better
    #     primary source; the canonical IDSA dose table is an unreadable image).
    if DOSE_GROUNDING.exists():
        dose_rows, c = _verify_rows(_records(DOSE_GROUNDING), by_url)
        for k in tot:
            tot[k] += c[k]
        ds = {"grounded": 0, "direction_only": 0, "refuted": 0, "pending": 0}
        if FORMULARY_REGISTRY.exists():
            reg = json.loads(FORMULARY_REGISTRY.read_text())
            # The registry is build-generated (object == objects/<sha256>.adj), but
            # validate before joining so a tampered registry can't escape the CAS dir.
            obj = str(reg.get("object", ""))
            if obj == f"objects/{reg.get('root', '')}.adj" and re.fullmatch(r"objects/[0-9a-f]{1,64}\.adj", obj):
                p = (FORMULARY_REGISTRY.parent / obj.replace(".adj", ".json")).resolve()
                if FORMULARY_REGISTRY.parent.resolve() in p.parents and p.exists():
                    ds = json.loads(p.read_text()).get("dose_grounding_summary", ds)
        artifacts.append({
            "name": "meningitis dosing", "path": "treatment/antibiotics/formulary.json",
            "grounded": ds["grounded"], "flagged": ds["direction_only"],
            "authored_debt": ds["refuted"] + ds["pending"], "rows": dose_rows})

    # 2c. UTI organism identification (G4 — the first specialty expansion). Its cited
    #     sources are not yet decomposed, so citation rows show pending until a UTI
    #     decompose-source run lands them in the CAS (a noted follow-up).
    if UTI_GROUNDING.exists():
        uti_rows, c = _verify_rows(_records(UTI_GROUNDING), by_url)
        for k in tot:
            tot[k] += c[k]
        uman = json.loads(UTI_MANIFEST.read_text()) if UTI_MANIFEST.exists() else {"clauses": {}}
        artifacts.append({
            "name": "UTI organism identification", "path": "diagnosis/uti/uti-id.adj",
            "grounded": sum(1 for x in uman["clauses"].values() if x["verdict"] == "ACCEPT"),
            "flagged": sum(1 for x in uman["clauses"].values() if x["verdict"] == "FLAG"),
            "authored_debt": sum(1 for x in uman["clauses"].values() if x["verdict"] == "PENDING"),
            "rows": uti_rows})

    # 2d. Treatment constraints (CC-3 — the contraindication/interaction rules behind the
    #     optimizer's exclusions). Manifest keys rules (not clauses); sources pending decompose.
    if TC_GROUNDING.exists():
        tc_rows, c = _verify_rows(_records(TC_GROUNDING), by_url)
        for k in tot:
            tot[k] += c[k]
        tman = json.loads(TC_MANIFEST.read_text()).get("rules", {}) if TC_MANIFEST.exists() else {}
        artifacts.append({
            "name": "treatment constraints", "path": "treatment/constraints/treatment-constraints.json",
            "grounded": sum(1 for x in tman.values() if x["verdict"] == "ACCEPT"),
            "flagged": sum(1 for x in tman.values() if x["verdict"] == "FLAG"),
            "authored_debt": sum(1 for x in tman.values() if x["verdict"] == "PENDING"),
            "rows": tc_rows})

    # 2e. Bacteremia organism priors (G5 — MYCIN's primary domain). Sources pending decompose.
    if BSI_GROUNDING.exists():
        bsi_rows, c = _verify_rows(_records(BSI_GROUNDING), by_url)
        for k in tot:
            tot[k] += c[k]
        bman = json.loads(BSI_MANIFEST.read_text()).get("clauses", {}) if BSI_MANIFEST.exists() else {}
        artifacts.append({
            "name": "bacteremia organism priors", "path": "diagnosis/bacteremia/source-id.adj",
            "grounded": sum(1 for x in bman.values() if x["verdict"] == "ACCEPT"),
            "flagged": sum(1 for x in bman.values() if x["verdict"] == "FLAG"),
            "authored_debt": sum(1 for x in bman.values() if x["verdict"] == "PENDING"),
            "rows": bsi_rows})

    # 3. Rebuild the system-wide provenance ledger.
    LEDGER.write_text(harness.build_ledger(artifacts) +
                      f"\n_Citation verification against decomposed sources in the CAS: "
                      f"**{tot['verified']} fully verified**, {tot['partial']} core-verified "
                      f"(over-reach), {tot['unverified']} unverified, {tot['nosrc']} pending._\n")
    print(f"ground_sources: citations — {tot['verified']} fully verified, {tot['partial']} "
          f"core-verified, {tot['unverified']} UNVERIFIED, {tot['nosrc']} pending; "
          f"{len(artifacts)} artifacts; ledger rebuilt")
    return 0


def main(argv: list[str]) -> int:
    if "--list" in argv:
        return emit_source_list()
    return commit_and_verify()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
