#!/usr/bin/env python3
"""test_cardio_recall.py — the ADJ-only auscultation library (MYCIN-2026 CARDIO).

The eighth recall domain shipped as a pure ADJ artifact (fourth Tier-2 organ-system
domain, after RHEUM, ONCO, HISTO). `cardio-edges.adj` is the SOLE source of truth — facts
+ byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only `import`s
the library and asks binding queries (`cardio-recall.query.adj` — the shape an LLM
produces), binds the grounded lesion for each murmur, each carrying an AUTHORITATIVE
citation with a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_cardio_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "cardio-edges.adj"
QUERY = HERE / "cardio-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: murmur -> the lesion the library binds for it.
EXPECTED = {
    "holosystolic_apical": "mitral_regurgitation",
    "crescendo_decrescendo_systolic": "aortic_stenosis",
    "late_systolic_click": "mitral_valve_prolapse",
    "holosystolic_left_lower_sternal": "tricuspid_regurgitation",
    "decrescendo_diastolic": "aortic_regurgitation",
    "diastolic_apical": "mitral_stenosis",
}
REL = "murmur_indicates"
VAR = "Lesion"


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
    assert "trust consensus" not in text, "CARDIO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "cardio_edge_ground.py").exists()
    assert not (HERE / "cardio-edge-grounding.json").exists()
    assert not (HERE / "cardio-edge-manifest.json").exists()


# ---- 2. the engine binds each lesion, each with an authoritative citation ----

def test_engine_binds_every_lesion_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for murmur, lesion in EXPECTED.items():
        q = f"{REL}({murmur}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {lesion}, f"{murmur}: bound {set(bound)} != {{{lesion}}}"
        cite = bound[lesion]
        assert cite.get("trust") == "authoritative", f"{murmur}/{lesion} not authoritative"
        assert cite.get("source"), f"{murmur}/{lesion} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary murmur abstains, never fabricates ----------------------

def test_unknown_murmur_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".cardio_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # continuous_machinery (PDA) is not in the library → must abstain
            fh.write(f'import "cardio-edges.adj"\n? {REL}(continuous_machinery, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded murmur must abstain"
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
    print(f"\ntest_cardio_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
