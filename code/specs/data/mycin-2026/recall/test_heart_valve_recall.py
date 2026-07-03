#!/usr/bin/env python3
"""test_heart_valve_recall.py — the ADJ-only heart-valve→auscultation-site recall
library (MYCIN-2026 HEARTVALVE).

A pure-ADJ recall domain (high-yield cardiac-exam board table): the chest-wall
site where each cardiac valve is best auscultated. `heart-valve-edges.adj` is the
SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON,
no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`heart-valve-recall.query.adj` — the shape an LLM produces), binds the
grounded location for each valve, each carrying an AUTHORITATIVE citation with a
real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_heart_valve_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "heart-valve-edges.adj"
QUERY = HERE / "heart-valve-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: cardiac valve -> the auscultation site the library binds.
EXPECTED = {
    "aortic_valve": "right_second_intercostal_space",                    # NBK541010
    "pulmonic_valve": "left_second_intercostal_space",                   # NBK541010
    "tricuspid_valve": "fourth_left_intercostal_space",                  # NBK541010
    "mitral_valve": "fifth_intercostal_space_midclavicular_line",        # NBK541010
}
REL = "best_heard_at"
VAR = "Location"


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
    assert "trust consensus" not in text, "HEARTVALVE ships no authored-debt; ground or omit"
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
    assert not (HERE / "heart_valve_edge_ground.py").exists()
    assert not (HERE / "heart-valve-edge-grounding.json").exists()
    assert not (HERE / "heart-valve-edge-manifest.json").exists()


# ---- 2. the engine binds each location with an authoritative citation --------------

def test_engine_binds_every_location_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for valve, location in EXPECTED.items():
        q = f"{REL}({valve}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {location}, f"{valve}: bound {set(bound)} != {{{location}}}"
        cite = bound[location]
        assert cite.get("trust") == "authoritative", f"{valve}/{location} not authoritative"
        assert cite.get("source"), f"{valve}/{location} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary valve abstains, never fabricates --------------------------

def test_unknown_valve_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".heartvalve_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the eustachian valve (valve of the inferior vena cava) is a real
            # cardiac valve but is not one of the four auscultated valves, so it
            # is deliberately not in this library — the engine must abstain.
            fh.write(f'import "heart-valve-edges.adj"\n? {REL}(eustachian_valve, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded valve must abstain"
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
    print(f"\ntest_heart_valve_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
