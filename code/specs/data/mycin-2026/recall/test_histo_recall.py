#!/usr/bin/env python3
"""test_histo_recall.py — the ADJ-only histology-finding library (MYCIN-2026 HISTO).

The seventh recall domain shipped as a pure ADJ artifact (third Tier-2 organ-system
domain, after RHEUM and ONCO). `histo-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the property
that matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`histo-recall.query.adj` — the shape an LLM produces),
binds EVERY grounded condition for each finding (a finding may point to several), each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_histo_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "histo-edges.adj"
QUERY = HERE / "histo-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: finding -> the set of conditions the library binds for it.
EXPECTED = {
    "reed_sternberg_cells": {"hodgkin_lymphoma"},
    "howell_jolly_bodies": {"asplenia", "megaloblastic_anemia"},
    "heinz_bodies": {"g6pd_deficiency", "methemoglobinemia"},
}
REL = "seen_in"
VAR = "Condition"


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
    assert "trust consensus" not in text, "HISTO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = sum(len(v) for v in EXPECTED.values())  # 5
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "histo_edge_ground.py").exists()
    assert not (HERE / "histo-edge-grounding.json").exists()
    assert not (HERE / "histo-edge-manifest.json").exists()


# ---- 2. the engine binds every grounded condition, each with an authoritative citation ----

def test_engine_binds_every_condition_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for finding, conditions in EXPECTED.items():
        q = f"{REL}({finding}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == conditions, f"{finding}: bound {set(bound)} != {conditions}"
        for c, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{finding}/{c} not authoritative"
            assert cite.get("source"), f"{finding}/{c} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary finding abstains, never fabricates ----------------------

def test_unknown_finding_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".histo_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # basophilic_stippling is not in the library → must abstain
            fh.write(f'import "histo-edges.adj"\n? {REL}(basophilic_stippling, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded finding must abstain"
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
    print(f"\ntest_histo_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
