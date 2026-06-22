#!/usr/bin/env python3
"""test_gi_recall.py — the ADJ-only GI biopsy/histology library (MYCIN-2026 GI).

The tenth recall domain shipped as a pure ADJ artifact (sixth Tier-2 organ-system domain,
after RHEUM, ONCO, HISTO, CARDIO, NEURO). `gi-edges.adj` is the SOLE source of truth —
facts + byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only `import`s
the library and asks binding queries (`gi-recall.query.adj` — the shape an LLM produces),
binds the grounded diagnosis for each biopsy finding, each carrying an AUTHORITATIVE
citation with a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_gi_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "gi-edges.adj"
QUERY = HERE / "gi-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: biopsy finding -> the diagnosis the library binds for it.
EXPECTED = {
    "villous_atrophy": "celiac_disease",
    "transmural_inflammation": "crohn_disease",
    "goblet_cells": "barrett_esophagus",
    "absence_of_ganglion_cells": "hirschsprung_disease",
    "eosinophils_15_per_hpf": "eosinophilic_esophagitis",
    "crypt_abscesses": "ulcerative_colitis",            # primary-source backfill (NBK470312)
    "onion_skin_fibrosis": "primary_sclerosing_cholangitis",  # NBK537181 (PSC abbrev defined on page)
    # --- BATCH: high-yield GI/hepatobiliary biopsy findings ---
    "signet_ring_cells": "gastric_cancer",                # expand (NBK459142)
    "florid_duct_lesion": "primary_biliary_cholangitis",  # expand (NBK459209)
    "cowdry_a_inclusions": "herpes_esophagitis",          # expand (NBK442012)
    "pseudomembranes": "pseudomembranous_colitis",        # expand (NBK470319)
}
REL = "biopsy_finding_in"
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
    assert "trust consensus" not in text, "GI ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 7
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "gi_edge_ground.py").exists()
    assert not (HERE / "gi-edge-grounding.json").exists()
    assert not (HERE / "gi-edge-manifest.json").exists()


# ---- 2. the engine binds each diagnosis, each with an authoritative citation ----

def test_engine_binds_every_diagnosis_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for finding, disease in EXPECTED.items():
        q = f"{REL}({finding}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {disease}, f"{finding}: bound {set(bound)} != {{{disease}}}"
        cite = bound[disease]
        assert cite.get("trust") == "authoritative", f"{finding}/{disease} not authoritative"
        assert cite.get("source"), f"{finding}/{disease} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary finding abstains, never fabricates -----------------------

def test_unknown_finding_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".gi_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # cobblestone_mucosa is not in the library → must abstain
            fh.write(f'import "gi-edges.adj"\n? {REL}(cobblestone_mucosa, ${VAR})\n')
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
    print(f"\ntest_gi_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
