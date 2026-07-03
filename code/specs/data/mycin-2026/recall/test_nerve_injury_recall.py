#!/usr/bin/env python3
"""test_nerve_injury_recall.py — the ADJ-only peripheral-nerve-lesion recall library (MYCIN-2026 NERVE).

A new pure-ADJ recall domain (high-yield clinical-neuroanatomy board content): a named
peripheral/cranial nerve and the classic clinical sign its injury produces.
`nerve-injury-edges.adj` is the SOLE source of truth — facts + byte-provenance inline,
no Python gate, no JSON, no manifest. This test pins the property that matters: the
native adj-lang engine, given a query .adj that only `import`s the library and asks
binding queries (`nerve-injury-recall.query.adj` — the shape an LLM produces), binds
the grounded sign for each nerve, each carrying an AUTHORITATIVE citation with a real
source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_nerve_injury_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "nerve-injury-edges.adj"
QUERY = HERE / "nerve-injury-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: peripheral/cranial nerve -> the localizing sign the library binds.
# Each nerve maps to one canonical sign, so each query binds exactly one answer.
EXPECTED = {
    "radial_nerve": "wrist_drop",                          # NBK532993
    "median_nerve": "hand_of_benediction",                 # NBK554458
    "ulnar_nerve": "ulnar_claw_hand",                      # NBK431063
    "long_thoracic_nerve": "winged_scapula",               # NBK535396
    "recurrent_laryngeal_nerve": "vocal_cord_paralysis",   # NBK560832
    "facial_nerve": "facial_paralysis",                    # NBK482290
    "superior_gluteal_nerve": "trendelenburg_gait",        # NBK535408
    "deep_fibular_nerve": "foot_drop",                     # NBK526033
}
REL = "nerve_lesion_sign"
VAR = "Sign"


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
    assert "trust consensus" not in text, "NERVE ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 8
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
    assert not (HERE / "nerve_injury_edge_ground.py").exists()
    assert not (HERE / "nerve-injury-edge-grounding.json").exists()
    assert not (HERE / "nerve-injury-edge-manifest.json").exists()


# ---- 2. the engine binds each sign, each with an authoritative citation ------------

def test_engine_binds_every_sign_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for nerve, sign in EXPECTED.items():
        q = f"{REL}({nerve}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {sign}, f"{nerve}: bound {set(bound)} != {{{sign}}}"
        cite = bound[sign]
        assert cite.get("trust") == "authoritative", f"{nerve}/{sign} not authoritative"
        assert cite.get("source"), f"{nerve}/{sign} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary nerve abstains, never fabricates --------------------------

def test_unknown_nerve_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".nerve_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # femoral_nerve is not in the library (knee-extension deficit) -> must abstain
            fh.write(f'import "nerve-injury-edges.adj"\n? {REL}(femoral_nerve, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded nerve must abstain"
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
    print(f"\ntest_nerve_injury_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
