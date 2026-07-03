#!/usr/bin/env python3
"""test_dural_sinus_recall.py — the ADJ-only dural-venous-sinus→drainage recall
library (MYCIN-2026 DURALSINUS).

A pure-ADJ recall domain (high-yield neuroanatomy board table): the structure
each major dural venous sinus drains into along the cerebral venous outflow path.
`dural-sinus-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`dural-sinus-recall.query.adj` — the shape an
LLM produces), binds the grounded destination for each sinus, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ;
the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_dural_sinus_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "dural-sinus-edges.adj"
QUERY = HERE / "dural-sinus-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: dural venous sinus -> the destination the library binds.
EXPECTED = {
    "superior_sagittal_sinus": "confluence_of_sinuses",                 # NBK546615
    "straight_sinus": "confluence_of_sinuses",                          # NBK482257
    "transverse_sinus": "sigmoid_sinus",                               # NBK546605
    "sigmoid_sinus": "internal_jugular_vein",                          # NBK482257
    "cavernous_sinus": "superior_and_inferior_petrosal_sinuses",       # NBK459244
}
REL = "drains_into"
VAR = "Destination"


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
    assert "trust consensus" not in text, "DURALSINUS ships no authored-debt; ground or omit"
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
    assert not (HERE / "dural_sinus_edge_ground.py").exists()
    assert not (HERE / "dural-sinus-edge-grounding.json").exists()
    assert not (HERE / "dural-sinus-edge-manifest.json").exists()


# ---- 2. the engine binds each destination with an authoritative citation -----------

def test_engine_binds_every_destination_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for sinus, destination in EXPECTED.items():
        q = f"{REL}({sinus}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {destination}, f"{sinus}: bound {set(bound)} != {{{destination}}}"
        cite = bound[destination]
        assert cite.get("trust") == "authoritative", f"{sinus}/{destination} not authoritative"
        assert cite.get("source"), f"{sinus}/{destination} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary sinus abstains, never fabricates --------------------------

def test_unknown_sinus_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".duralsinus_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the inferior sagittal sinus is a real dural venous sinus (it is even
            # named inside one source span as a tributary) but is deliberately not
            # a subject in this library, so the engine must abstain rather than
            # fabricate.
            fh.write(f'import "dural-sinus-edges.adj"\n? {REL}(inferior_sagittal_sinus, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded sinus must abstain"
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
    print(f"\ntest_dural_sinus_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
