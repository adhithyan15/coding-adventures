#!/usr/bin/env python3
"""test_vitamin_activation_recall.py — the ADJ-only vitamin→active-form recall
library (MYCIN-2026 VITAMIN-ACTIVATION).

A pure-ADJ recall domain (high-yield biochemistry board table): the biologically
active (coenzyme / hormonally active) form each vitamin is converted to.
`vitamin-activation-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`vitamin-activation-recall.query.adj`
— the shape an LLM produces), binds the grounded active form for each vitamin,
each carrying an AUTHORITATIVE citation with a real source span + locator.
Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_vitamin_activation_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "vitamin-activation-edges.adj"
QUERY = HERE / "vitamin-activation-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: vitamin -> the active form the library binds.
EXPECTED = {
    "vitamin_d": "calcitriol",                 # NBK526025
    "folate": "tetrahydrofolate",              # NBK539712
    "vitamin_b6": "pyridoxal_phosphate",       # NBK557436
    "vitamin_a": "retinoic_acid",              # NBK482362
}
REL = "activated_to"
VAR = "ActiveForm"


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
    assert "trust consensus" not in text, "VITAMIN-ACTIVATION ships no authored-debt; ground or omit"
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
    assert not (HERE / "vitamin_activation_edge_ground.py").exists()
    assert not (HERE / "vitamin-activation-edge-grounding.json").exists()
    assert not (HERE / "vitamin-activation-edge-manifest.json").exists()


# ---- 2. the engine binds each active form with an authoritative citation -------------

def test_engine_binds_every_active_form_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for vitamin, active_form in EXPECTED.items():
        q = f"{REL}({vitamin}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {active_form}, f"{vitamin}: bound {set(bound)} != {{{active_form}}}"
        cite = bound[active_form]
        assert cite.get("trust") == "authoritative", f"{vitamin}/{active_form} not authoritative"
        assert cite.get("source"), f"{vitamin}/{active_form} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary vitamin abstains, never fabricates --------------------------

def test_unknown_vitamin_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".vitamin_activation_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # "thiamine" (vitamin B1) IS converted to an active form (thiamine
            # pyrophosphate), but no self-contained span naming both the vitamin
            # and the spelled-out active form cleared the grounding bar, so it was
            # deliberately deferred and is absent here — the engine must abstain
            # rather than fabricate an active form.
            fh.write(f'import "vitamin-activation-edges.adj"\n? {REL}(thiamine, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded vitamin must abstain"
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
    print(f"\ntest_vitamin_activation_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
