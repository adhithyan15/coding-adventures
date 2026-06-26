#!/usr/bin/env python3
"""test_pharyngeal_arch_recall.py — the ADJ-only pharyngeal-arch→cranial-nerve
recall library (MYCIN-2026 PHARYNGEAL).

A pure-ADJ recall domain (classic high-yield embryology board table): the cranial
nerve that innervates / derives from each pharyngeal (branchial) arch.
`pharyngeal-arch-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`pharyngeal-arch-recall.query.adj` — the shape
an LLM produces), binds the grounded nerve for each arch, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ;
the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_pharyngeal_arch_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "pharyngeal-arch-edges.adj"
QUERY = HERE / "pharyngeal-arch-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: pharyngeal arch -> the cranial nerve the library binds.
EXPECTED = {
    "first_pharyngeal_arch": "trigeminal_nerve",              # NBK507820
    "second_pharyngeal_arch": "facial_nerve",                 # NBK555950
    "third_pharyngeal_arch": "glossopharyngeal_nerve",        # NBK539877
    "fourth_pharyngeal_arch": "superior_laryngeal_nerve",     # NBK532995
    "sixth_pharyngeal_arch": "recurrent_laryngeal_nerve",     # NBK538307
}
REL = "innervated_by"
VAR = "Nerve"


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
    assert "trust consensus" not in text, "PHARYNGEAL ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 5
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "pharyngeal_arch_edge_ground.py").exists()
    assert not (HERE / "pharyngeal-arch-edge-grounding.json").exists()
    assert not (HERE / "pharyngeal-arch-edge-manifest.json").exists()


# ---- 2. the engine binds each nerve with an authoritative citation -----------------

def test_engine_binds_every_nerve_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for arch, nerve in EXPECTED.items():
        q = f"{REL}({arch}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {nerve}, f"{arch}: bound {set(bound)} != {{{nerve}}}"
        cite = bound[nerve]
        assert cite.get("trust") == "authoritative", f"{arch}/{nerve} not authoritative"
        assert cite.get("source"), f"{arch}/{nerve} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary arch abstains, never fabricates --------------------------

def test_unknown_arch_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".pharyngeal_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the fifth pharyngeal arch is rudimentary/absent in humans and is
            # deliberately not in this library, so the engine must abstain rather
            # than fabricate.
            fh.write(f'import "pharyngeal-arch-edges.adj"\n? {REL}(fifth_pharyngeal_arch, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded arch must abstain"
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
    print(f"\ntest_pharyngeal_arch_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
