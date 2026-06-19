#!/usr/bin/env python3
"""test_resp_recall.py — the ADJ-only occupational-lung-disease library (MYCIN-2026 RESP).

The twelfth recall domain shipped as a pure ADJ artifact (eighth Tier-2 organ-system
domain, after RHEUM, ONCO, HISTO, CARDIO, NEURO, GI, DERM). `resp-edges.adj` is the SOLE
source of truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest.
This test pins the property that matters: the native adj-lang engine, given a query .adj
that only `import`s the library and asks binding queries (`resp-recall.query.adj` — the
shape an LLM produces), binds the grounded pneumoconiosis for each exposure, each carrying
an AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ; the
engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_resp_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "resp-edges.adj"
QUERY = HERE / "resp-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: occupational exposure -> the pneumoconiosis the library binds.
EXPECTED = {
    "silica": "silicosis",
    "asbestos": "asbestosis",
    "beryllium": "berylliosis",
    "cotton_dust": "byssinosis",
    "coal_dust": "coal_workers_pneumoconiosis",
}
# Every edge cites a primary authority; we accept any of the recognized authoritative
# hosts (NCBI StatPearls/Bookshelf and CDC/NIOSH), not a single hardcoded domain — the
# primary-source-first policy grounds an edge wherever the clean both-endpoints span lives.
_AUTH_HOSTS = ("https://www.ncbi.nlm.nih.gov/", "https://www.cdc.gov/")
REL = "inhalation_causes"
VAR = "Disease"


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
    assert "trust consensus" not in text, "RESP ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 4
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "resp_edge_ground.py").exists()
    assert not (HERE / "resp-edge-grounding.json").exists()
    assert not (HERE / "resp-edge-manifest.json").exists()


# ---- 2. the engine binds each pneumoconiosis, each with an authoritative citation ----

def test_engine_binds_every_disease_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for exposure, disease in EXPECTED.items():
        q = f"{REL}({exposure}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {disease}, f"{exposure}: bound {set(bound)} != {{{disease}}}"
        cite = bound[disease]
        assert cite.get("trust") == "authoritative", f"{exposure}/{disease} not authoritative"
        assert cite.get("source"), f"{exposure}/{disease} has no source span"
        assert cite.get("locator", "").startswith(_AUTH_HOSTS), f"{exposure} locator not a primary host"


# ---- 3. an off-vocabulary exposure abstains, never fabricates ----------------------

def test_unknown_exposure_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".resp_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # tobacco_smoke is not in the library → must abstain
            fh.write(f'import "resp-edges.adj"\n? {REL}(tobacco_smoke, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded exposure must abstain"
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
    print(f"\ntest_resp_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
