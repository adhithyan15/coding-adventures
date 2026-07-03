#!/usr/bin/env python3
"""test_muscle_fiber_recall.py — the ADJ-only skeletal-muscle fiber-type→property
recall library (MYCIN-2026 MUSCLEFIBER).

A pure-ADJ recall domain (high-yield physiology board table): the defining
contractile/metabolic properties of the three classic skeletal-muscle fiber types
(Type I, Type IIa, Type IIb). `muscle-fiber-edges.adj` is the SOLE source of truth
— facts + byte-provenance inline, no Python gate, no JSON, no manifest. This test
pins the property that matters: the native adj-lang engine, given a query .adj that
only `import`s the library and asks binding queries (`muscle-fiber-recall.query.adj`
— the shape an LLM produces), binds the grounded properties for each fiber type,
each carrying an AUTHORITATIVE citation with a real source span + locator. Type I
maps to several properties, so a query binds several answers. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_muscle_fiber_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "muscle-fiber-edges.adj"
QUERY = HERE / "muscle-fiber-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded fiber type -> the SET of properties the library binds for it.
EXPECTED = {
    "type_i_fiber": {"slow_twitch", "high_myoglobin", "fatigue_resistant"},   # NBK537139
    "type_iia_fiber": {"fast_oxidative"},                                     # NBK537139
    "type_iib_fiber": {"fast_twitch_glycolytic"},                             # NBK537139
}
EDGE_COUNT = sum(len(v) for v in EXPECTED.values())  # 5
REL = "has_property"
VAR = "Property"


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
    assert "trust consensus" not in text, "MUSCLEFIBER ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    assert text.count("    relate ") == EDGE_COUNT
    assert text.count("trust authoritative") == EDGE_COUNT
    assert text.count('\n        locator "') == EDGE_COUNT
    assert text.count('\n        source "') == EDGE_COUNT
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "muscle_fiber_edge_ground.py").exists()
    assert not (HERE / "muscle-fiber-edge-grounding.json").exists()
    assert not (HERE / "muscle-fiber-edge-manifest.json").exists()


# ---- 2. the engine binds every property with an authoritative citation -------------

def test_engine_binds_every_property_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for fiber_type, properties in EXPECTED.items():
        q = f"{REL}({fiber_type}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == properties, f"{fiber_type}: bound {set(bound)} != {properties}"
        for prop, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{fiber_type}/{prop} not authoritative"
            assert cite.get("source"), f"{fiber_type}/{prop} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary fiber type abstains, never fabricates ---------------------

def test_unknown_fiber_type_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".musclefiber_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # there is no "Type III" skeletal muscle fiber — the classic types are
            # Type I, IIa, and IIb. A query for a non-existent type must abstain
            # rather than fabricate a property.
            fh.write(f'import "muscle-fiber-edges.adj"\n? {REL}(type_iii_fiber, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded fiber type must abstain"
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
    print(f"\ntest_muscle_fiber_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
