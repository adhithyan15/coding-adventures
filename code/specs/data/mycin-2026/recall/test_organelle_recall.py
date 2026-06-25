#!/usr/bin/env python3
"""test_organelle_recall.py — the ADJ-only cell-organelle→function recall library
(MYCIN-2026 ORGANELLE).

A new pure-ADJ recall domain (foundational high-yield cell-biology/biochem board
table): the principal cellular function each organelle performs. `organelle-edges.adj`
is the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding queries
(`organelle-recall.query.adj` — the shape an LLM produces), binds the grounded
function for each organelle, each carrying an AUTHORITATIVE citation with a real
source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_organelle_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "organelle-edges.adj"
QUERY = HERE / "organelle-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: organelle -> the function the library binds.
EXPECTED = {
    "mitochondria": "atp_production",                              # NBK553175
    "golgi_apparatus": "protein_modification_and_sorting",        # NBK9838
    "lysosome": "intracellular_degradation",                      # NBK9953
    "peroxisome": "fatty_acid_oxidation",                         # NBK560676
    "ribosome": "protein_synthesis",                              # NBK9849
    "nucleus": "genetic_material_storage",                        # NBK9845
}
REL = "performs"
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
    assert "trust consensus" not in text, "ORGANELLE ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "organelle_edge_ground.py").exists()
    assert not (HERE / "organelle-edge-grounding.json").exists()
    assert not (HERE / "organelle-edge-manifest.json").exists()


# ---- 2. the engine binds each function with an authoritative citation --------------

def test_engine_binds_every_function_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for organelle, function in EXPECTED.items():
        q = f"{REL}({organelle}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {function}, f"{organelle}: bound {set(bound)} != {{{function}}}"
        cite = bound[function]
        assert cite.get("trust") == "authoritative", f"{organelle}/{function} not authoritative"
        assert cite.get("source"), f"{organelle}/{function} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary organelle abstains, never fabricates ----------------------

def test_unknown_organelle_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".organelle_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the centrosome is a real organelle (organizes microtubules / mitotic
            # spindle) but is deliberately not in the library, so the engine must
            # abstain rather than fabricate.
            fh.write(f'import "organelle-edges.adj"\n? {REL}(centrosome, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded organelle must abstain"
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
    print(f"\ntest_organelle_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
