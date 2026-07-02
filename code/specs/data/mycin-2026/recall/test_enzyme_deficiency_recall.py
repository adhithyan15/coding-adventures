#!/usr/bin/env python3
"""test_enzyme_deficiency_recall.py — the ADJ-only enzyme-deficiency disease recall library (MYCIN-2026 ENZYME).

A new pure-ADJ recall domain (high-yield biochemistry board content): the deficient enzyme
and the inborn error of metabolism it causes. `enzyme-deficiency-edges.adj` is the SOLE
source of truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest.
This test pins the property that matters: the native adj-lang engine, given a query .adj
that only `import`s the library and asks binding queries
(`enzyme-deficiency-recall.query.adj` — the shape an LLM produces), binds the grounded
disease for each enzyme, each carrying an AUTHORITATIVE citation with a real source span +
locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_enzyme_deficiency_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "enzyme-deficiency-edges.adj"
QUERY = HERE / "enzyme-deficiency-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: deficient enzyme -> the disease the library binds.
# Each enzyme maps to one canonical disease, so each query binds exactly one answer.
EXPECTED = {
    "glucocerebrosidase": "gaucher_disease",                                    # NBK448080
    "sphingomyelinase": "niemann_pick_disease",                                 # NBK556129
    "alpha_galactosidase_a": "fabry_disease",                                   # NBK435996
    "arylsulfatase_a": "metachromatic_leukodystrophy",                          # NBK560744
    "glucose_6_phosphatase": "von_gierke_disease",                              # NBK534196
    "acid_alpha_glucosidase": "pompe_disease",                                  # NBK470558
    "galactose_1_phosphate_uridyltransferase": "galactosemia",                  # NBK441957
    "homogentisate_oxidase": "alkaptonuria",                                    # NBK560571
}
REL = "enzyme_deficiency_disease"
VAR = "Disease"


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
    assert "trust consensus" not in text, "ENZYME ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 8
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "enzyme_deficiency_edge_ground.py").exists()
    assert not (HERE / "enzyme-deficiency-edge-grounding.json").exists()
    assert not (HERE / "enzyme-deficiency-edge-manifest.json").exists()


# ---- 2. the engine binds each disease, each with an authoritative citation ---------

def test_engine_binds_every_disease_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for enzyme, disease in EXPECTED.items():
        q = f"{REL}({enzyme}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {disease}, f"{enzyme}: bound {set(bound)} != {{{disease}}}"
        cite = bound[disease]
        assert cite.get("trust") == "authoritative", f"{enzyme}/{disease} not authoritative"
        assert cite.get("source"), f"{enzyme}/{disease} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary enzyme abstains, never fabricates -------------------------

def test_unknown_enzyme_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".enzyme_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # hexosaminidase_a (Tay-Sachs) is not in the library -> must abstain
            fh.write(f'import "enzyme-deficiency-edges.adj"\n? {REL}(hexosaminidase_a, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded enzyme must abstain"
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
    print(f"\ntest_enzyme_deficiency_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
