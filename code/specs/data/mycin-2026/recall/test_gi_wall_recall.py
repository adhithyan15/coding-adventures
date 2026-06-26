#!/usr/bin/env python3
"""test_gi_wall_recall.py — the ADJ-only GI-tract-wall-layer→tissue recall library
(MYCIN-2026 GIWALL).

A pure-ADJ recall domain (high-yield histology board table): the characteristic
tissue composing each of the four concentric layers of the gastrointestinal tract
wall (mucosa, submucosa, muscularis externa, serosa). `gi-wall-edges.adj` is the
SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON, no
manifest. This test pins the property that matters: the native adj-lang engine,
given a query .adj that only `import`s the library and asks binding queries
(`gi-wall-recall.query.adj` — the shape an LLM produces), binds the grounded tissue
for each wall layer, each carrying an AUTHORITATIVE citation with a real source span
+ locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_gi_wall_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "gi-wall-edges.adj"
QUERY = HERE / "gi-wall-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: GI-wall layer -> the tissue the library binds.
EXPECTED = {
    "mucosa": "epithelium",                    # NBK537103
    "submucosa": "connective_tissue",          # NBK537103
    "muscularis_externa": "smooth_muscle",     # NBK537103
    "serosa": "mesothelium",                   # NBK459366
}
REL = "composed_of"
VAR = "Tissue"


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
    assert "trust consensus" not in text, "GIWALL ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 4
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "gi_wall_edge_ground.py").exists()
    assert not (HERE / "gi-wall-edge-grounding.json").exists()
    assert not (HERE / "gi-wall-edge-manifest.json").exists()


# ---- 2. the engine binds each tissue with an authoritative citation ----------------

def test_engine_binds_every_tissue_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for layer, tissue in EXPECTED.items():
        q = f"{REL}({layer}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {tissue}, f"{layer}: bound {set(bound)} != {{{tissue}}}"
        cite = bound[tissue]
        assert cite.get("trust") == "authoritative", f"{layer}/{tissue} not authoritative"
        assert cite.get("source"), f"{layer}/{tissue} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary structure abstains, never fabricates ----------------------

def test_unknown_layer_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".giwall_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the lumen is the central cavity of the GI tract, NOT one of the four
            # concentric wall layers this library models, so it is deliberately
            # absent — the engine must abstain rather than fabricate a tissue.
            fh.write(f'import "gi-wall-edges.adj"\n? {REL}(lumen, ${VAR})\n')
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
    print(f"\ntest_gi_wall_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
