#!/usr/bin/env python3
"""test_collagen_defect_recall.py — the ADJ-only collagen-type→associated-disease
recall library (MYCIN-2026 COLLAGEN).

A new pure-ADJ recall domain (classic high-yield biochemistry/genetics board
table): the heritable connective-tissue disease caused by a defect in each
collagen type. `collagen-defect-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`collagen-defect-recall.query.adj`
— the shape an LLM produces), binds the grounded disease for each collagen type,
each carrying an AUTHORITATIVE citation with a real source span + locator.
Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_collagen_defect_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "collagen-defect-edges.adj"
QUERY = HERE / "collagen-defect-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: collagen type -> the disease the library binds.
EXPECTED = {
    "type_i": "osteogenesis_imperfecta",                  # NBK536957
    "type_ii": "stickler_syndrome",                       # NBK1302
    "type_iv": "alport_syndrome",                         # NBK470419
    "type_v": "classical_ehlers_danlos",                  # NBK549814
    "type_vii": "dystrophic_epidermolysis_bullosa",       # NBK1304
}
REL = "defect_causes"
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
    assert "trust consensus" not in text, "COLLAGEN ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 5
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "collagen_defect_edge_ground.py").exists()
    assert not (HERE / "collagen-defect-edge-grounding.json").exists()
    assert not (HERE / "collagen-defect-edge-manifest.json").exists()


# ---- 2. the engine binds each disease with an authoritative citation ---------------

def test_engine_binds_every_disease_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for collagen, disease in EXPECTED.items():
        q = f"{REL}({collagen}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {disease}, f"{collagen}: bound {set(bound)} != {{{disease}}}"
        cite = bound[disease]
        assert cite.get("trust") == "authoritative", f"{collagen}/{disease} not authoritative"
        assert cite.get("source"), f"{collagen}/{disease} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary collagen type abstains, never fabricates ------------------

def test_unknown_collagen_type_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".collagen_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # type X collagen is a real collagen (defects cause Schmid metaphyseal
            # chondrodysplasia) but is deliberately not in the library, so the
            # engine must abstain rather than fabricate.
            fh.write(f'import "collagen-defect-edges.adj"\n? {REL}(type_x, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded collagen type must abstain"
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
    print(f"\ntest_collagen_defect_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
