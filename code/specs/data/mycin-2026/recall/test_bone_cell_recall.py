#!/usr/bin/env python3
"""test_bone_cell_recall.py — the ADJ-only bone-cell→function recall library
(MYCIN-2026 BONECELL).

A pure-ADJ recall domain (high-yield histology/physiology board table): the
principal function each bone (and related cartilage) cell type performs.
`bone-cell-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`bone-cell-recall.query.adj` — the shape an LLM
produces), binds the grounded function for each cell, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_bone_cell_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "bone-cell-edges.adj"
QUERY = HERE / "bone-cell-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: bone cell -> the function the library binds.
EXPECTED = {
    "osteoblast": "bone_formation",                  # NBK557792
    "osteoclast": "bone_resorption",                 # NBK554489
    "osteocyte": "regulates_bone_remodeling",        # NBK441968
    "chondrocyte": "produces_cartilage_matrix",      # NBK557576
}
REL = "has_function"
VAR = "Function"


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
    assert "trust consensus" not in text, "BONECELL ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 4
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "bone_cell_edge_ground.py").exists()
    assert not (HERE / "bone-cell-edge-grounding.json").exists()
    assert not (HERE / "bone-cell-edge-manifest.json").exists()


# ---- 2. the engine binds each function with an authoritative citation --------------

def test_engine_binds_every_function_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for cell, function in EXPECTED.items():
        q = f"{REL}({cell}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {function}, f"{cell}: bound {set(bound)} != {{{function}}}"
        cite = bound[function]
        assert cite.get("trust") == "authoritative", f"{cell}/{function} not authoritative"
        assert cite.get("source"), f"{cell}/{function} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary cell abstains, never fabricates --------------------------

def test_unknown_cell_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".bonecell_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # fibroblast is a real connective-tissue cell but is deliberately not
            # in this bone-cell library, so the engine must abstain rather than
            # fabricate.
            fh.write(f'import "bone-cell-edges.adj"\n? {REL}(fibroblast, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded cell must abstain"
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
    print(f"\ntest_bone_cell_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
