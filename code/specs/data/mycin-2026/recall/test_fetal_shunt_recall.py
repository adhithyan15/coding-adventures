#!/usr/bin/env python3
"""test_fetal_shunt_recall.py — the ADJ-only fetal-structure→adult-remnant recall
library (MYCIN-2026 FETALSHUNT).

A pure-ADJ recall domain (classic high-yield embryology/anatomy board table): the
obliterated adult remnant each fetal circulatory shunt / vessel becomes after
birth. `fetal-shunt-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`fetal-shunt-recall.query.adj` —
the shape an LLM produces), binds the grounded remnant for each fetal structure,
each carrying an AUTHORITATIVE citation with a real source span + locator.
Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_fetal_shunt_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "fetal-shunt-edges.adj"
QUERY = HERE / "fetal-shunt-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: fetal structure -> the adult remnant the library binds.
EXPECTED = {
    "ductus_arteriosus": "ligamentum_arteriosum",            # NBK553173
    "ductus_venosus": "ligamentum_venosum",                  # NBK547759
    "umbilical_vein": "ligamentum_teres_hepatis",            # NBK557490
    "foramen_ovale": "fossa_ovalis",                         # NBK545203
    "umbilical_arteries": "medial_umbilical_ligaments",      # NBK551649
    "urachus": "median_umbilical_ligament",                  # NBK531465
}
REL = "becomes"
VAR = "Remnant"


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
    assert "trust consensus" not in text, "FETALSHUNT ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "fetal_shunt_edge_ground.py").exists()
    assert not (HERE / "fetal-shunt-edge-grounding.json").exists()
    assert not (HERE / "fetal-shunt-edge-manifest.json").exists()


# ---- 2. the engine binds each remnant with an authoritative citation ---------------

def test_engine_binds_every_remnant_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for fetal, remnant in EXPECTED.items():
        q = f"{REL}({fetal}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {remnant}, f"{fetal}: bound {set(bound)} != {{{remnant}}}"
        cite = bound[remnant]
        assert cite.get("trust") == "authoritative", f"{fetal}/{remnant} not authoritative"
        assert cite.get("source"), f"{fetal}/{remnant} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary fetal structure abstains, never fabricates ---------------

def test_unknown_structure_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".fetalshunt_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the notochord is a real embryonic structure (it becomes the nucleus
            # pulposus) but is deliberately not in this fetal-circulation library,
            # so the engine must abstain rather than fabricate.
            fh.write(f'import "fetal-shunt-edges.adj"\n? {REL}(notochord, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded structure must abstain"
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
    print(f"\ntest_fetal_shunt_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
