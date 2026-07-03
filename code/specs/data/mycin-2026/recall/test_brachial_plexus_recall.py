#!/usr/bin/env python3
"""test_brachial_plexus_recall.py — the ADJ-only brachial-plexus origin recall library (MYCIN-2026 BRACHIAL).

A new pure-ADJ recall domain (high-yield anatomy board content): which cord (or
root) of the brachial plexus each terminal nerve arises from. `brachial-plexus-edges.adj`
is the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding queries
(`brachial-plexus-recall.query.adj` — the shape an LLM produces), binds the grounded
origin for each nerve, each carrying an AUTHORITATIVE citation with a real source
span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_brachial_plexus_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "brachial-plexus-edges.adj"
QUERY = HERE / "brachial-plexus-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: terminal nerve -> the cord/root the library binds.
EXPECTED = {
    "musculocutaneous_nerve": "lateral_cord",                # NBK534199
    "axillary_nerve": "posterior_cord",                      # NBK493212
    "radial_nerve": "posterior_cord",                        # NBK534840
    "ulnar_nerve": "medial_cord",                            # NBK499892
    "long_thoracic_nerve": "roots",                          # NBK535396
    "median_nerve": "lateral_and_medial_cords",              # NBK448084
}
REL = "brachial_plexus_origin"
VAR = "Origin"


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
    assert "trust consensus" not in text, "BRACHIAL ships no authored-debt; ground or omit"
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
    assert not (HERE / "brachial_plexus_edge_ground.py").exists()
    assert not (HERE / "brachial-plexus-edge-grounding.json").exists()
    assert not (HERE / "brachial-plexus-edge-manifest.json").exists()


# ---- 2. the engine binds each origin with an authoritative citation ----------------

def test_engine_binds_every_origin_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for nerve, origin in EXPECTED.items():
        q = f"{REL}({nerve}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {origin}, f"{nerve}: bound {set(bound)} != {{{origin}}}"
        cite = bound[origin]
        assert cite.get("trust") == "authoritative", f"{nerve}/{origin} not authoritative"
        assert cite.get("source"), f"{nerve}/{origin} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary nerve abstains, never fabricates --------------------------

def test_unknown_nerve_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".brachial_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # phrenic_nerve is a real nerve but arises from the CERVICAL plexus, not the
            # brachial plexus — it is deliberately not in the library, so must abstain.
            fh.write(f'import "brachial-plexus-edges.adj"\n? {REL}(phrenic_nerve, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded nerve must abstain"
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
    print(f"\ntest_brachial_plexus_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
