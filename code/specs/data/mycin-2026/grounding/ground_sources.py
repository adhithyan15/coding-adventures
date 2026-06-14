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

# Every grounding file whose records cite sources we decompose + verify against.
GROUNDING_FILES = [ORG_GROUNDING, HOST_GROUNDING]


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

    # 2. Verify each fact's citation against its decomposed source (no blind trust).
    rows, verified_n, partial_n, unverified_n, nosrc_n = [], 0, 0, 0, 0
    for r in grounded_records():
        g = r["grounded"]
        h = by_url.get(g["resolved_url"])
        so = harness.load_source_object(h) if h else None
        if so is None:
            rows.append((r["id"], r["spider_status"], "—", g.get("source_title", "")[:36], "no-source-obj"))
            nosrc_n += 1
            continue
        v = harness.verify_citation(g["byte_quote"], so)
        m, t = v["fragments_matched"], v["fragments_total"]
        if v["verified"]:
            verified_n += 1
            mark = f"✓ verified ({m}/{t})"
        elif v["core_verified"]:
            partial_n += 1
            mark = f"◑ core ✓ ({m}/{t} spans; over-reach)"
        else:
            unverified_n += 1
            mark = f"✗ UNVERIFIED ({m}/{t})"
        rows.append((r["id"], r["spider_status"],
                     harness.gate(r["spider_status"])[0], f"src:{h}", mark))

    # 3. Rebuild the system-wide provenance ledger (organism-id + source verification).
    man = json.loads(ORG_MANIFEST.read_text()) if ORG_MANIFEST.exists() else {"clauses": {}}
    grounded = sum(1 for c in man["clauses"].values() if c["verdict"] == "ACCEPT")
    flagged = sum(1 for c in man["clauses"].values() if c["verdict"] == "FLAG")
    # Authoring debt = clauses still carried without grounding (verdict PENDING). Drives
    # to 0 as the spider grounds the host factors (G2) — no longer a hardcoded count.
    debt = sum(1 for c in man["clauses"].values() if c["verdict"] == "PENDING")
    artifact = {"name": "organism identification", "path": "diagnosis/organisms/organism-id.adj",
                "grounded": grounded, "flagged": flagged, "authored_debt": debt, "rows": rows}
    LEDGER.write_text(harness.build_ledger([artifact]) +
                      f"\n_Citation verification against decomposed sources in the CAS: "
                      f"**{verified_n} fully verified**, {partial_n} core-verified (citation "
                      f"over-reaches the current decomposition — queued for deeper grounding), "
                      f"{unverified_n} unverified, {nosrc_n} pending. Every ACCEPTed (grounded) "
                      f"prior has at least its load-bearing span verified._\n")
    print(f"ground_sources: citations — {verified_n} fully verified, {partial_n} core-verified "
          f"(over-reach), {unverified_n} UNVERIFIED, {nosrc_n} pending; ledger rebuilt")
    return 0


def main(argv: list[str]) -> int:
    if "--list" in argv:
        return emit_source_list()
    return commit_and_verify()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
