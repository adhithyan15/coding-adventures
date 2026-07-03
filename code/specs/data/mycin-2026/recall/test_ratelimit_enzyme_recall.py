#!/usr/bin/env python3
"""test_ratelimit_enzyme_recall.py — the ADJ-only metabolic-pathway→rate-limiting-enzyme
recall library (MYCIN-2026 RATELIMIT).

A new pure-ADJ recall domain (classic high-yield biochemistry board table): the
rate-limiting (committed-step) enzyme of each major metabolic pathway.
`ratelimit-enzyme-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`ratelimit-enzyme-recall.query.adj` — the shape an
LLM produces), binds the grounded rate-limiting enzyme for each pathway, each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge lives
in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_ratelimit_enzyme_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "ratelimit-enzyme-edges.adj"
QUERY = HERE / "ratelimit-enzyme-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: pathway -> the rate-limiting enzyme the library binds.
EXPECTED = {
    "glycolysis": "phosphofructokinase_1",                 # NBK576428
    "citric_acid_cycle": "isocitrate_dehydrogenase",       # NBK556032
    "cholesterol_synthesis": "hmg_coa_reductase",          # NBK542212
    "fatty_acid_synthesis": "acetyl_coa_carboxylase",      # NBK2413
    "fatty_acid_oxidation": "carnitine_palmitoyltransferase_1",  # NBK556002
    "heme_synthesis": "ala_synthase",                      # NBK537352
    "glycogenesis": "glycogen_synthase",                   # NBK539802
    "glycogenolysis": "glycogen_phosphorylase",            # NBK549820
}
REL = "rate_limiting_enzyme_of"
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
    assert "trust consensus" not in text, "RATELIMIT ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 8
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "ratelimit_enzyme_edge_ground.py").exists()
    assert not (HERE / "ratelimit-enzyme-edge-grounding.json").exists()
    assert not (HERE / "ratelimit-enzyme-edge-manifest.json").exists()


# ---- 2. the engine binds each rate-limiting enzyme with an authoritative citation --

def test_engine_binds_every_enzyme_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for pathway, enzyme in EXPECTED.items():
        q = f"{REL}({pathway}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {enzyme}, f"{pathway}: bound {set(bound)} != {{{enzyme}}}"
        cite = bound[enzyme]
        assert cite.get("trust") == "authoritative", f"{pathway}/{enzyme} not authoritative"
        assert cite.get("source"), f"{pathway}/{enzyme} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary pathway abstains, never fabricates ------------------------

def test_unknown_pathway_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".ratelimit_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # the urea cycle is a real pathway (rate-limiting enzyme = carbamoyl
            # phosphate synthetase I) but is deliberately not in the library, so the
            # engine must abstain rather than fabricate.
            fh.write(f'import "ratelimit-enzyme-edges.adj"\n? {REL}(urea_cycle, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded pathway must abstain"
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
    print(f"\ntest_ratelimit_enzyme_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
