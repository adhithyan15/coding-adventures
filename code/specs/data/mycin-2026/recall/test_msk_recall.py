#!/usr/bin/env python3
"""test_msk_recall.py — the ADJ-only musculoskeletal special-test recall library (MYCIN-2026 MSK).

A new pure-ADJ recall domain (high-yield orthopedics/sports-medicine board content): the
named physical-exam special test and the diagnosis it points to. `msk-edges.adj` is the
SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON, no
manifest. This test pins the property that matters: the native adj-lang engine, given a
query .adj that only `import`s the library and asks binding queries (`msk-recall.query.adj`
— the shape an LLM produces), binds the grounded diagnosis for each special test, each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_msk_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "msk-edges.adj"
QUERY = HERE / "msk-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: special test -> the diagnosis the library binds for it.
# Two distinct tests may share a diagnosis (Phalen + Tinel -> carpal tunnel); each
# test still binds exactly one diagnosis, so the single-answer property holds.
EXPECTED = {
    "mcmurray_test": "meniscal_tear",                 # NBK470549
    "lachman_test": "acl_tear",                       # NBK554415
    "finkelstein_test": "de_quervain_tenosynovitis",  # NBK539768
    "phalen_test": "carpal_tunnel_syndrome",          # NBK555934
    "tinel_sign": "carpal_tunnel_syndrome",           # NBK555934
    "spurling_test": "cervical_radiculopathy",        # NBK493152
    "trendelenburg_sign": "hip_abductor_weakness",    # NBK555987
}
REL = "special_test_indicates"
VAR = "Diagnosis"


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
    assert "trust consensus" not in text, "MSK ships no authored-debt; ground or omit"
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
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "msk_edge_ground.py").exists()
    assert not (HERE / "msk-edge-grounding.json").exists()
    assert not (HERE / "msk-edge-manifest.json").exists()


# ---- 2. the engine binds each diagnosis, each with an authoritative citation ----

def test_engine_binds_every_diagnosis_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for test, diagnosis in EXPECTED.items():
        q = f"{REL}({test}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {diagnosis}, f"{test}: bound {set(bound)} != {{{diagnosis}}}"
        cite = bound[diagnosis]
        assert cite.get("trust") == "authoritative", f"{test}/{diagnosis} not authoritative"
        assert cite.get("source"), f"{test}/{diagnosis} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary test abstains, never fabricates ---------------------------

def test_unknown_test_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".msk_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # thompson_test is not in the library (deferred — see header) -> must abstain
            fh.write(f'import "msk-edges.adj"\n? {REL}(thompson_test, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded test must abstain"
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
    print(f"\ntest_msk_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
