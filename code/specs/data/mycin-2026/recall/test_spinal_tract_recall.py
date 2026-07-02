#!/usr/bin/env python3
"""test_spinal_tract_recall.py — the ADJ-only spinal-cord-tract→function recall
library (MYCIN-2026 SPINAL-TRACT).

A new pure-ADJ recall domain (classic high-yield neuroanatomy board table): the
sensory/motor function each ascending or descending spinal cord tract carries.
`spinal-tract-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`spinal-tract-recall.query.adj` — the shape an
LLM produces), binds the grounded function for each tract, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ;
the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_spinal_tract_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "spinal-tract-edges.adj"
QUERY = HERE / "spinal-tract-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: tract -> the function the library binds.
EXPECTED = {
    "lateral_corticospinal_tract": "voluntary_motor_control",                 # NBK534818
    "dorsal_column": "fine_touch_vibration_proprioception",                   # NBK507888
    "spinothalamic_tract": "pain_and_temperature",                            # NBK507824
    "dorsal_spinocerebellar_tract": "unconscious_proprioception",             # NBK556013
    "rubrospinal_tract": "flexor_muscles",                                    # NBK554542
    "anterior_corticospinal_tract": "axial_muscles",                          # NBK546614
}
REL = "carries"
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
    assert "trust consensus" not in text, "SPINAL-TRACT ships no authored-debt; ground or omit"
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
    assert not (HERE / "spinal_tract_edge_ground.py").exists()
    assert not (HERE / "spinal-tract-edge-grounding.json").exists()
    assert not (HERE / "spinal-tract-edge-manifest.json").exists()


# ---- 2. the engine binds each function with an authoritative citation --------------

def test_engine_binds_every_function_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for tract, function in EXPECTED.items():
        q = f"{REL}({tract}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {function}, f"{tract}: bound {set(bound)} != {{{function}}}"
        cite = bound[function]
        assert cite.get("trust") == "authoritative", f"{tract}/{function} not authoritative"
        assert cite.get("source"), f"{tract}/{function} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary tract abstains, never fabricates --------------------------

def test_unknown_tract_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".spinal_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the tectospinal tract is a real spinal cord tract (coordinates head
            # and eye movements toward stimuli) but is deliberately not in the
            # library, so the engine must abstain rather than fabricate.
            fh.write(f'import "spinal-tract-edges.adj"\n? {REL}(tectospinal_tract, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded tract must abstain"
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
    print(f"\ntest_spinal_tract_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
