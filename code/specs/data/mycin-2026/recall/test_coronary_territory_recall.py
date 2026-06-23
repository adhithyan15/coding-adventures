#!/usr/bin/env python3
"""test_coronary_territory_recall.py — the ADJ-only coronary-territory recall library (MYCIN-2026 CORONARY).

A new pure-ADJ recall domain (high-yield cardiology/anatomy board content): which
coronary artery supplies which region of the heart (myocardial wall, the
interventricular septum, or a conduction node). `coronary-territory-edges.adj` is
the SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON,
no manifest. This test pins the property that matters: the native adj-lang engine,
given a query .adj that only `import`s the library and asks binding queries
(`coronary-territory-recall.query.adj` — the shape an LLM produces), binds the
grounded region(s) for each artery, each carrying an AUTHORITATIVE citation with a
real source span + locator.

Unlike the one-answer recall domains, an artery here may supply SEVERAL regions
(the RCA → inferior wall + SA node + AV node), so a query binds a SET of regions;
the test checks the full set per artery.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_coronary_territory_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "coronary-territory-edges.adj"
QUERY = HERE / "coronary-territory-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Each artery -> the SET of regions the library grounds it to. The RCA and LAD
# are deliberately multi-region: a single artery perfuses several territories.
EXPECTED = {
    "lad": {"anterior_wall", "interventricular_septum"},                          # NBK482375
    "right_coronary_artery": {"inferior_wall", "sinoatrial_node",                 # NBK470572
                              "atrioventricular_node"},                           # NBK534790 / NBK557664
    "left_circumflex_artery": {"lateral_wall"},                                   # NBK537228
    "posterior_descending_artery": {"posterior_septum"},                          # NBK537207
}
REL = "coronary_artery_territory"
VAR = "Region"
EDGE_COUNT = sum(len(v) for v in EXPECTED.values())  # 7 grounded clauses


def _cli_available() -> bool:
    return CLI.exists()


def _run(program: Path) -> dict:
    out = subprocess.run([str(CLI), str(program)], capture_output=True, text=True,
                         cwd=str(HERE), timeout=60)
    assert out.returncode == 0, f"adj-lang-cli failed: {out.stderr}"
    return json.loads(out.stdout)


# ---- 1. the ADJ file is the artifact — every clause grounded, no authored-debt ----

def test_library_is_pure_adj_and_fully_grounded() -> None:
    text = ADJ.read_text()
    assert "trust consensus" not in text, "CORONARY ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    assert text.count("    relate ") == EDGE_COUNT
    assert text.count("trust authoritative") == EDGE_COUNT
    assert text.count('\n        locator "') == EDGE_COUNT
    assert text.count('\n        source "') == EDGE_COUNT
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "coronary_territory_edge_ground.py").exists()
    assert not (HERE / "coronary-territory-edge-grounding.json").exists()
    assert not (HERE / "coronary-territory-edge-manifest.json").exists()


# ---- 2. the engine binds each region set, each with an authoritative citation ------

def test_engine_binds_every_region_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for artery, regions in EXPECTED.items():
        q = f"{REL}({artery}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == regions, f"{artery}: bound {set(bound)} != {regions}"
        for region, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{artery}/{region} not authoritative"
            assert cite.get("source"), f"{artery}/{region} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary artery abstains, never fabricates -------------------------

def test_unknown_artery_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".coronary_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # ramus_intermedius is a real coronary branch but is NOT in the library -> must abstain
            fh.write(f'import "coronary-territory-edges.adj"\n? {REL}(ramus_intermedius, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded artery must abstain"
    finally:
        os.unlink(path)


def _run_all() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"  FAIL  {t.__name__}: {exc}")
    print(f"\ntest_coronary_territory_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
