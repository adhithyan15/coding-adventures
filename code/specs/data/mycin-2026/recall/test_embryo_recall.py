#!/usr/bin/env python3
"""test_embryo_recall.py — the ADJ-only embryology recall library (MYCIN-2026 EMBRYO).

A new pure-ADJ recall domain (high-yield embryology board content): the adult
structure and the embryonic precursor it derives from. `embryo-edges.adj` is the SOLE
source of truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest.
This test pins the property that matters: the native adj-lang engine, given a query .adj
that only `import`s the library and asks binding queries (`embryo-recall.query.adj` — the
shape an LLM produces), binds the grounded origin for each structure, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ; the
engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_embryo_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "embryo-edges.adj"
QUERY = HERE / "embryo-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: adult structure -> the embryonic origin the library binds.
# Several structures share an origin (four neural-crest derivatives); each structure
# still binds exactly one origin, so the single-answer property holds.
EXPECTED = {
    "adrenal_medulla": "neural_crest",                # NBK539836
    "melanocytes": "neural_crest",                    # NBK547700
    "schwann_cells": "neural_crest",                  # NBK544316
    "parafollicular_c_cells": "neural_crest",         # NBK519054
    "posterior_pituitary": "neuroectoderm",           # NBK526130
    "diaphragm": "septum_transversum",                # NBK560497
    # --- write-once / use-many: both from ONE NBK557724 span ---
    "thymus": "third_pharyngeal_pouch",               # NBK557724
    "inferior_parathyroid": "third_pharyngeal_pouch",  # NBK557724 (same span)
}
REL = "embryonic_origin"
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
    assert "trust consensus" not in text, "EMBRYO ships no authored-debt; ground or omit"
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
    assert not (HERE / "embryo_edge_ground.py").exists()
    assert not (HERE / "embryo-edge-grounding.json").exists()
    assert not (HERE / "embryo-edge-manifest.json").exists()


# ---- 2. the engine binds each origin, each with an authoritative citation ----------

def test_engine_binds_every_origin_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for structure, origin in EXPECTED.items():
        q = f"{REL}({structure}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {origin}, f"{structure}: bound {set(bound)} != {{{origin}}}"
        cite = bound[origin]
        assert cite.get("trust") == "authoritative", f"{structure}/{origin} not authoritative"
        assert cite.get("source"), f"{structure}/{origin} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary structure abstains, never fabricates ----------------------

def test_unknown_structure_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".embryo_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # anterior_pituitary is not in the library (oral ectoderm/Rathke pouch) -> must abstain
            fh.write(f'import "embryo-edges.adj"\n? {REL}(anterior_pituitary, ${VAR})\n')
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
    print(f"\ntest_embryo_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
