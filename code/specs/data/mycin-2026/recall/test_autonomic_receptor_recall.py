#!/usr/bin/env python3
"""test_autonomic_receptor_recall.py — the ADJ-only autonomic-receptor→effect
recall library (MYCIN-2026 AUTONOMIC).

A pure-ADJ recall domain (high-yield autonomic-pharmacology board table): the
principal physiological effect each adrenergic / muscarinic receptor subtype
mediates. `autonomic-receptor-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`autonomic-receptor-recall.query.adj`
— the shape an LLM produces), binds the grounded effect for each receptor, each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge
lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_autonomic_receptor_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "autonomic-receptor-edges.adj"
QUERY = HERE / "autonomic-receptor-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: autonomic receptor -> the effect the library binds.
EXPECTED = {
    "alpha_1": "vasoconstriction",                            # NBK551698
    "alpha_2": "decreases_norepinephrine_release",            # NBK459124
    "beta_1": "increases_heart_rate_and_contractility",       # NBK532904
    "beta_2": "smooth_muscle_relaxation",                     # NBK542249
    "m2": "decreases_heart_rate",                             # NBK555909
    "m3": "smooth_muscle_contraction",                        # NBK555909
}
REL = "mediates"
VAR = "Effect"


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
    assert "trust consensus" not in text, "AUTONOMIC ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "autonomic_receptor_edge_ground.py").exists()
    assert not (HERE / "autonomic-receptor-edge-grounding.json").exists()
    assert not (HERE / "autonomic-receptor-edge-manifest.json").exists()


# ---- 2. the engine binds each effect with an authoritative citation ----------------

def test_engine_binds_every_effect_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for receptor, effect in EXPECTED.items():
        q = f"{REL}({receptor}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {effect}, f"{receptor}: bound {set(bound)} != {{{effect}}}"
        cite = bound[effect]
        assert cite.get("trust") == "authoritative", f"{receptor}/{effect} not authoritative"
        assert cite.get("source"), f"{receptor}/{effect} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary receptor abstains, never fabricates ----------------------

def test_unknown_receptor_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".autonomic_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # beta_3 is a real adrenergic receptor (lipolysis / bladder relaxation)
            # but is deliberately not in the library, so the engine must abstain
            # rather than fabricate.
            fh.write(f'import "autonomic-receptor-edges.adj"\n? {REL}(beta_3, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded receptor must abstain"
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
    print(f"\ntest_autonomic_receptor_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
