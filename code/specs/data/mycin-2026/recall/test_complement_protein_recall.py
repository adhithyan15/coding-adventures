#!/usr/bin/env python3
"""test_complement_protein_recall.py — the ADJ-only complement-protein→function
recall library (MYCIN-2026 COMPLEMENT).

A pure-ADJ recall domain (high-yield immunology board table): the effector
function of each major complement component. `complement-protein-edges.adj` is
the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`complement-protein-recall.query.adj` — the shape an LLM produces),
binds the grounded function for each component, each carrying an AUTHORITATIVE
citation with a real source span + locator. Knowledge lives in ADJ; the engine
answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_complement_protein_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "complement-protein-edges.adj"
QUERY = HERE / "complement-protein-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: complement component -> the function the library binds.
EXPECTED = {
    "c3b": "opsonization",                            # NBK534215
    "c5a": "neutrophil_chemotactic_agent",            # NBK544229
    "membrane_attack_complex": "causes_cell_lysis",   # NBK544229
    "c3a": "anaphylatoxin",                           # NBK544229
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
    assert "trust consensus" not in text, "COMPLEMENT ships no authored-debt; ground or omit"
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
    assert not (HERE / "complement_protein_edge_ground.py").exists()
    assert not (HERE / "complement-protein-edge-grounding.json").exists()
    assert not (HERE / "complement-protein-edge-manifest.json").exists()


# ---- 2. the engine binds each function with an authoritative citation ----------------

def test_engine_binds_every_function_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for component, function in EXPECTED.items():
        q = f"{REL}({component}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {function}, f"{component}: bound {set(bound)} != {{{function}}}"
        cite = bound[function]
        assert cite.get("trust") == "authoritative", f"{component}/{function} not authoritative"
        assert cite.get("source"), f"{component}/{function} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary component abstains, never fabricates ------------------------

def test_unknown_component_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".complement_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # "C4b" is a real complement fragment, but its function is not modeled
            # by this library (no self-contained grounded span was committed for
            # it), so it is deliberately absent — the engine must abstain rather
            # than fabricate a function.
            fh.write(f'import "complement-protein-edges.adj"\n? {REL}(c4b, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded component must abstain"
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
    print(f"\ntest_complement_protein_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
