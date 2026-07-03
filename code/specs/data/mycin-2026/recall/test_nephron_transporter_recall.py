#!/usr/bin/env python3
"""test_nephron_transporter_recall.py — the ADJ-only nephron-transporter recall library (MYCIN-2026 NEPHRON).

A new pure-ADJ recall domain (high-yield renal-physiology board content): which
membrane transporter / channel operates at each nephron segment (and so where each
diuretic class acts). `nephron-transporter-edges.adj` is the SOLE source of truth —
facts + byte-provenance inline, no Python gate, no JSON, no manifest. This test pins
the property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`nephron-transporter-recall.query.adj`
— the shape an LLM produces), binds the grounded transporter(s) for each segment,
each carrying an AUTHORITATIVE citation with a real source span + locator.

Like the coronary domain, a segment may carry SEVERAL transporters (the proximal
tubule has SGLT2 + carbonic anhydrase), so a query binds a SET; the test checks the
full set per segment.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_nephron_transporter_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "nephron-transporter-edges.adj"
QUERY = HERE / "nephron-transporter-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Each nephron segment -> the SET of transporters the library grounds it to.
# The proximal convoluted tubule is deliberately multi-transporter.
EXPECTED = {
    "proximal_convoluted_tubule": {"sglt2", "carbonic_anhydrase"},  # NBK576405 / NBK557736
    "thick_ascending_limb": {"nkcc2"},                              # NBK546656
    "distal_convoluted_tubule": {"ncc"},                            # NBK430766
    "collecting_duct": {"aquaporin_2"},                            # NBK470458
    "principal_cell": {"enac"},                                    # NBK549766
    "descending_limb": {"water"},                                 # NBK538339
}
REL = "nephron_segment_transporter"
VAR = "Transporter"
EDGE_COUNT = sum(len(v) for v in EXPECTED.values())  # 7 grounded clauses


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
    assert "trust consensus" not in text, "NEPHRON ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    assert text.count("    relate ") == EDGE_COUNT
    assert text.count("trust authoritative") == EDGE_COUNT
    assert text.count('\n        locator "') == EDGE_COUNT
    assert text.count('\n        source "') == EDGE_COUNT
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "nephron_transporter_edge_ground.py").exists()
    assert not (HERE / "nephron-transporter-edge-grounding.json").exists()
    assert not (HERE / "nephron-transporter-edge-manifest.json").exists()


# ---- 2. the engine binds each transporter set, each with an authoritative citation -

def test_engine_binds_every_transporter_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for segment, transporters in EXPECTED.items():
        q = f"{REL}({segment}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edges missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == transporters, f"{segment}: bound {set(bound)} != {transporters}"
        for transporter, cite in bound.items():
            assert cite.get("trust") == "authoritative", f"{segment}/{transporter} not authoritative"
            assert cite.get("source"), f"{segment}/{transporter} has no source span"
            assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary segment abstains, never fabricates ------------------------

def test_unknown_segment_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".nephron_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # macula_densa is a real nephron structure but is NOT in the library -> must abstain
            fh.write(f'import "nephron-transporter-edges.adj"\n? {REL}(macula_densa, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded segment must abstain"
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
    print(f"\ntest_nephron_transporter_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
