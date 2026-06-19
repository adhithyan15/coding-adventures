#!/usr/bin/env python3
"""test_rheum_recall.py — the ADJ-only rheumatology autoantibody library (MYCIN-2026 RHEUM).

The fifth recall domain shipped as a pure ADJ artifact, and the first Tier-2 organ-system
pathology domain. `rheum-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that matters:
the native adj-lang engine, given a query .adj that only `import`s the library and asks
binding queries (`rheum-recall.query.adj` — the shape an LLM produces), binds EVERY
grounded antibody for each disease (a disease has several serologic markers), each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_rheum_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "rheum-edges.adj"
QUERY = HERE / "rheum-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: disease -> the set of autoantibodies the library binds for it.
EXPECTED = {
    "systemic_lupus_erythematosus": {"anti_dsdna", "anti_smith", "anti_ribosomal_p"},
    "granulomatosis_with_polyangiitis": {"pr3"},
    "systemic_sclerosis": {"anti_scl70", "anti_centromere"},
    "rheumatoid_arthritis": {"anti_ccp"},                 # primary-source backfill (NBK441999)
    "sjogren_syndrome": {"anti_ro", "anti_la"},           # primary-source backfill (NBK431049)
}
REL = "associated_autoantibody"
VAR = "Antibody"


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
    assert "trust consensus" not in text, "RHEUM ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = sum(len(v) for v in EXPECTED.values())  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "rheum_edge_ground.py").exists()
    assert not (HERE / "rheum-edge-grounding.json").exists()
    assert not (HERE / "rheum-edge-manifest.json").exists()


# ---- 2. the engine binds every grounded antibody, each with an authoritative citation ----

def test_engine_binds_every_antibody_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for disease, antibodies in EXPECTED.items():
        q = f"{REL}({disease}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            ab = a["bindings"][VAR]
            cite = (a.get("citations") or [{}])[0]
            bound[ab] = cite
        # Every grounded antibody for this disease must be bound...
        assert set(bound) == antibodies, f"{disease}: bound {set(bound)} != {antibodies}"
        # ...each citing an authoritative NCBI span.
        for ab, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{disease}/{ab} not authoritative"
            assert cite.get("source"), f"{disease}/{ab} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary disease abstains, never fabricates ----------------------

def test_unknown_disease_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".rheum_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # dermatomyositis is not in the library → must abstain
            fh.write(f'import "rheum-edges.adj"\n? {REL}(dermatomyositis, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded disease must abstain"
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
    print(f"\ntest_rheum_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
