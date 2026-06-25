#!/usr/bin/env python3
"""test_gi_hormone_recall.py — the ADJ-only gastrointestinal-hormone→action
recall library (MYCIN-2026 GIHORMONE).

A pure-ADJ recall domain (high-yield GI-physiology board table): the principal
physiological action each gut hormone performs. `gi-hormone-edges.adj` is the
SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON,
no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`gi-hormone-recall.query.adj` — the shape an LLM produces), binds the
grounded action for each hormone, each carrying an AUTHORITATIVE citation with a
real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_gi_hormone_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "gi-hormone-edges.adj"
QUERY = HERE / "gi-hormone-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: gut hormone -> the action the library binds.
EXPECTED = {
    "gastrin": "stimulates_gastric_acid_secretion_by_parietal_cells",  # NBK534822
    "secretin": "stimulates_pancreatic_bicarbonate_secretion",         # NBK537116
    "cholecystokinin": "stimulates_gallbladder_wall_contraction",      # NBK542254
    "somatostatin": "inhibits_secretion",                              # NBK538327
}
REL = "has_action"
VAR = "Action"


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
    assert "trust consensus" not in text, "GIHORMONE ships no authored-debt; ground or omit"
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
    assert not (HERE / "gi_hormone_edge_ground.py").exists()
    assert not (HERE / "gi-hormone-edge-grounding.json").exists()
    assert not (HERE / "gi-hormone-edge-manifest.json").exists()


# ---- 2. the engine binds each action with an authoritative citation ----------------

def test_engine_binds_every_action_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for hormone, action in EXPECTED.items():
        q = f"{REL}({hormone}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {action}, f"{hormone}: bound {set(bound)} != {{{action}}}"
        cite = bound[action]
        assert cite.get("trust") == "authoritative", f"{hormone}/{action} not authoritative"
        assert cite.get("source"), f"{hormone}/{action} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary hormone abstains, never fabricates -----------------------

def test_unknown_hormone_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".gihormone_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # motilin is a real gut hormone (migrating motor complex) but is
            # deliberately not in the library, so the engine must abstain rather
            # than fabricate.
            fh.write(f'import "gi-hormone-edges.adj"\n? {REL}(motilin, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded hormone must abstain"
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
    print(f"\ntest_gi_hormone_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
