#!/usr/bin/env python3
"""test_germ_layer_recall.py — the ADJ-only germ-layer→derivative recall library
(MYCIN-2026 GERMLAYER).

A pure-ADJ recall domain (high-yield embryology board table): representative adult
structures each of the three primary embryonic germ layers gives rise to.
`germ-layer-edges.adj` is the SOLE source of truth — facts + byte-provenance inline,
no Python gate, no JSON, no manifest. This test pins the property that matters: the
native adj-lang engine, given a query .adj that only `import`s the library and asks
binding queries (`germ-layer-recall.query.adj` — the shape an LLM produces), binds
the grounded derivatives for each germ layer, each carrying an AUTHORITATIVE
citation with a real source span + locator. Each germ layer maps to TWO derivatives,
so a query binds two answers. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_germ_layer_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "germ-layer-edges.adj"
QUERY = HERE / "germ-layer-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded germ layer -> the SET of derivatives the library binds for it.
EXPECTED = {
    "ectoderm": {"epidermis", "anterior_pituitary"},                  # NBK539836
    "mesoderm": {"kidney", "dermis"},                                 # NBK526024
    "endoderm": {"gastrointestinal_tract", "liver"},                  # NBK554394
}
EDGE_COUNT = sum(len(v) for v in EXPECTED.values())  # 6
REL = "gives_rise_to"
VAR = "Derivative"


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
    assert "trust consensus" not in text, "GERMLAYER ships no authored-debt; ground or omit"
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
    assert not (HERE / "germ_layer_edge_ground.py").exists()
    assert not (HERE / "germ-layer-edge-grounding.json").exists()
    assert not (HERE / "germ-layer-edge-manifest.json").exists()


# ---- 2. the engine binds every derivative with an authoritative citation -----------

def test_engine_binds_every_derivative_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for germ_layer, derivatives in EXPECTED.items():
        q = f"{REL}({germ_layer}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == derivatives, f"{germ_layer}: bound {set(bound)} != {derivatives}"
        for deriv, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{germ_layer}/{deriv} not authoritative"
            assert cite.get("source"), f"{germ_layer}/{deriv} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary germ layer abstains, never fabricates ---------------------

def test_unknown_germ_layer_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".germlayer_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the trophoblast is a real early-embryonic structure but is NOT one of
            # the three primary germ layers this library models, so it is
            # deliberately absent — the engine must abstain rather than guess.
            fh.write(f'import "germ-layer-edges.adj"\n? {REL}(trophoblast, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded germ layer must abstain"
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
    print(f"\ntest_germ_layer_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
