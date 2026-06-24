#!/usr/bin/env python3
"""test_lysosomal_storage_recall.py — the ADJ-only lysosomal-storage-disease→
accumulated-substrate recall library (MYCIN-2026 LYSOSOMAL).

A new pure-ADJ recall domain (classic high-yield biochemistry board table): the
substrate that accumulates in each lysosomal storage disease.
`lysosomal-storage-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that
matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`lysosomal-storage-recall.query.adj` — the shape
an LLM produces), binds the grounded substrate for each disease, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ;
the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_lysosomal_storage_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "lysosomal-storage-edges.adj"
QUERY = HERE / "lysosomal-storage-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: storage disease -> the substrate the library binds.
EXPECTED = {
    "tay_sachs": "gm2_ganglioside",                        # NBK564432
    "gaucher": "glucocerebroside",                         # NBK448080
    "niemann_pick": "sphingomyelin",                       # NBK556129
    "fabry": "globotriaosylceramide",                      # NBK435996
    "hurler": "glycosaminoglycans",                        # NBK532261
    "metachromatic_leukodystrophy": "sulfatides",          # NBK560744
    "pompe": "glycogen",                                   # NBK470558
}
REL = "accumulates"
VAR = "Substrate"


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
    assert "trust consensus" not in text, "LYSOSOMAL ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 7
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "lysosomal_storage_edge_ground.py").exists()
    assert not (HERE / "lysosomal-storage-edge-grounding.json").exists()
    assert not (HERE / "lysosomal-storage-edge-manifest.json").exists()


# ---- 2. the engine binds each substrate with an authoritative citation -------------

def test_engine_binds_every_substrate_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for disease, substrate in EXPECTED.items():
        q = f"{REL}({disease}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {substrate}, f"{disease}: bound {set(bound)} != {{{substrate}}}"
        cite = bound[substrate]
        assert cite.get("trust") == "authoritative", f"{disease}/{substrate} not authoritative"
        assert cite.get("source"), f"{disease}/{substrate} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary disease abstains, never fabricates ------------------------

def test_unknown_disease_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".lysosomal_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # cystinosis is a real lysosomal storage disease (cystine accumulates) but
            # is deliberately not in the library, so the engine must abstain rather
            # than fabricate.
            fh.write(f'import "lysosomal-storage-edges.adj"\n? {REL}(cystinosis, ${VAR})\n')
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
    print(f"\ntest_lysosomal_storage_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
