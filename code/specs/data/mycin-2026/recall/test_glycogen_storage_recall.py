#!/usr/bin/env python3
"""test_glycogen_storage_recall.py — the ADJ-only glycogen-storage-disease→
deficient-enzyme recall library (MYCIN-2026 GLYCOGEN).

A pure-ADJ recall domain (high-yield biochemistry board table): the enzyme whose
deficiency causes each glycogen storage disease. `glycogen-storage-edges.adj` is
the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`glycogen-storage-recall.query.adj` — the shape an LLM produces), binds
the grounded enzyme for each disease, each carrying an AUTHORITATIVE citation
with a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_glycogen_storage_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "glycogen-storage-edges.adj"
QUERY = HERE / "glycogen-storage-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: GSD -> the deficient enzyme the library binds.
EXPECTED = {
    "gsd_type_i_von_gierke": "glucose_6_phosphatase",            # NBK554417
    "gsd_type_ii_pompe": "acid_alpha_glucosidase",               # NBK470558
    "gsd_type_iii_cori": "glycogen_debranching_enzyme",          # NBK554417
    "gsd_type_vi_hers": "liver_glycogen_phosphorylase",          # NBK5941
}
REL = "deficient_in"
VAR = "Enzyme"


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
    assert "trust consensus" not in text, "GLYCOGEN ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 4
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "glycogen_storage_edge_ground.py").exists()
    assert not (HERE / "glycogen-storage-edge-grounding.json").exists()
    assert not (HERE / "glycogen-storage-edge-manifest.json").exists()


# ---- 2. the engine binds each enzyme with an authoritative citation ----------------

def test_engine_binds_every_enzyme_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for disease, enzyme in EXPECTED.items():
        q = f"{REL}({disease}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {enzyme}, f"{disease}: bound {set(bound)} != {{{enzyme}}}"
        cite = bound[enzyme]
        assert cite.get("trust") == "authoritative", f"{disease}/{enzyme} not authoritative"
        assert cite.get("source"), f"{disease}/{enzyme} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary disease abstains, never fabricates -----------------------

def test_unknown_disease_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".glycogen_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # GSD V / McArdle is a real GSD but is deliberately NOT in the library
            # (its enzyme co-naming was deferred), so the engine must abstain
            # rather than fabricate.
            fh.write(f'import "glycogen-storage-edges.adj"\n? {REL}(gsd_type_v_mcardle, ${VAR})\n')
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
    print(f"\ntest_glycogen_storage_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
