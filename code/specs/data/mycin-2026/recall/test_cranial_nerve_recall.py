#!/usr/bin/env python3
"""test_cranial_nerve_recall.py — the ADJ-only cranial-nerve function recall library (MYCIN-2026 CRANIAL).

A new pure-ADJ recall domain (foundational clinical-neuroanatomy board content): a named
cranial nerve and its primary function. `cranial-nerve-edges.adj` is the SOLE source of
truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest. This test
pins the property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`cranial-nerve-recall.query.adj` — the
shape an LLM produces), binds the grounded function for each nerve, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ; the
engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_cranial_nerve_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "cranial-nerve-edges.adj"
QUERY = HERE / "cranial-nerve-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: cranial nerve -> the primary function the library binds.
# Each nerve maps to one canonical function, so each query binds exactly one answer.
EXPECTED = {
    "olfactory_nerve": "smell",                                          # NBK556051
    "optic_nerve": "vision",                                             # NBK507907
    "trochlear_nerve": "superior_oblique_muscle",                        # NBK537244
    "abducens_nerve": "lateral_rectus_muscle",                           # NBK430711
    "vagus_nerve": "parasympathetic_innervation_of_viscera",             # NBK553141
    "accessory_nerve": "sternocleidomastoid_and_trapezius",              # NBK507722
    "hypoglossal_nerve": "tongue_muscles",                               # NBK532869
}
REL = "cranial_nerve_function"
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
    assert "trust consensus" not in text, "CRANIAL ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 7
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "cranial_nerve_edge_ground.py").exists()
    assert not (HERE / "cranial-nerve-edge-grounding.json").exists()
    assert not (HERE / "cranial-nerve-edge-manifest.json").exists()


# ---- 2. the engine binds each function, each with an authoritative citation --------

def test_engine_binds_every_function_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for nerve, function in EXPECTED.items():
        q = f"{REL}({nerve}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {function}, f"{nerve}: bound {set(bound)} != {{{function}}}"
        cite = bound[function]
        assert cite.get("trust") == "authoritative", f"{nerve}/{function} not authoritative"
        assert cite.get("source"), f"{nerve}/{function} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary cranial nerve abstains, never fabricates ------------------

def test_unknown_nerve_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".cranial_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # vestibulocochlear_nerve is not in the library (hearing/balance) -> must abstain
            fh.write(f'import "cranial-nerve-edges.adj"\n? {REL}(vestibulocochlear_nerve, ${VAR})\n')
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
    print(f"\ntest_cranial_nerve_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
