#!/usr/bin/env python3
"""test_epidermis_layer_recall.py — the ADJ-only epidermal-layer→characteristic
recall library (MYCIN-2026 EPIDERMIS).

A pure-ADJ recall domain (high-yield histology board table): the defining
characteristic of each stratum of the epidermis. `epidermis-layer-edges.adj` is
the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`epidermis-layer-recall.query.adj` — the shape an LLM produces), binds
the grounded characteristic for each layer, each carrying an AUTHORITATIVE
citation with a real source span + locator. Knowledge lives in ADJ; the engine
answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_epidermis_layer_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "epidermis-layer-edges.adj"
QUERY = HERE / "epidermis-layer-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: epidermal layer -> the characteristic the library binds.
EXPECTED = {
    "stratum_basale": "separated_from_dermis_by_basement_membrane",     # NBK470464
    "stratum_spinosum": "prickle_cell_layer",                           # NBK470464
    "stratum_granulosum": "keratohyalin_granules",                      # NBK470464
    "stratum_lucidum": "only_in_thick_skin",                           # NBK470464
    "stratum_corneum": "outermost_layer_keratinocyte_maturation",      # NBK513299
}
REL = "characterized_by"
VAR = "Characteristic"


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
    assert "trust consensus" not in text, "EPIDERMIS ships no authored-debt; ground or omit"
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
    assert not (HERE / "epidermis_layer_edge_ground.py").exists()
    assert not (HERE / "epidermis-layer-edge-grounding.json").exists()
    assert not (HERE / "epidermis-layer-edge-manifest.json").exists()


# ---- 2. the engine binds each characteristic with an authoritative citation ----------

def test_engine_binds_every_characteristic_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for layer, characteristic in EXPECTED.items():
        q = f"{REL}({layer}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {characteristic}, f"{layer}: bound {set(bound)} != {{{characteristic}}}"
        cite = bound[characteristic]
        assert cite.get("trust") == "authoritative", f"{layer}/{characteristic} not authoritative"
        assert cite.get("source"), f"{layer}/{characteristic} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary layer abstains, never fabricates ----------------------------

def test_unknown_layer_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".epidermis_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # "dermis" is a real skin layer, but it is NOT one of the five
            # epidermal strata this library models, so it is deliberately absent —
            # the engine must abstain rather than fabricate a characteristic.
            fh.write(f'import "epidermis-layer-edges.adj"\n? {REL}(dermis, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded layer must abstain"
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
    print(f"\ntest_epidermis_layer_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
