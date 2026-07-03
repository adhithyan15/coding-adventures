#!/usr/bin/env python3
"""test_exocrine_gland_recall.py — the ADJ-only exocrine-gland→characteristic
recall library (MYCIN-2026 EXOCRINE-GLAND).

A pure-ADJ recall domain (high-yield histology/physiology board table): the
distinguishing feature of each major head/skin exocrine gland — the three paired
salivary glands (by secretion type) and the two sweat glands (by function /
distribution). `exocrine-gland-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`exocrine-gland-recall.query.adj`
— the shape an LLM produces), binds the grounded characteristic for each gland,
each carrying an AUTHORITATIVE citation with a real source span + locator.
Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_exocrine_gland_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "exocrine-gland-edges.adj"
QUERY = HERE / "exocrine-gland-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: exocrine gland -> the characteristic the library binds.
EXPECTED = {
    "parotid_gland": "serous",                                # NBK551688
    "sublingual_gland": "mucous",                             # NBK551688
    "submandibular_gland": "mixed_predominantly_serous",      # NBK551688
    "eccrine_gland": "thermoregulatory_sweat",                # NBK482278
    "apocrine_gland": "axillary_anogenital",                  # NBK482199
}
REL = "characterized_by"
VAR = "Characteristic"


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
    assert "trust consensus" not in text, "EXOCRINE-GLAND ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 5
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "exocrine_gland_edge_ground.py").exists()
    assert not (HERE / "exocrine-gland-edge-grounding.json").exists()
    assert not (HERE / "exocrine-gland-edge-manifest.json").exists()


# ---- 2. the engine binds each characteristic with an authoritative citation ----------

def test_engine_binds_every_characteristic_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for gland, characteristic in EXPECTED.items():
        q = f"{REL}({gland}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {characteristic}, f"{gland}: bound {set(bound)} != {{{characteristic}}}"
        cite = bound[characteristic]
        assert cite.get("trust") == "authoritative", f"{gland}/{characteristic} not authoritative"
        assert cite.get("source"), f"{gland}/{characteristic} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary gland abstains, never fabricates ----------------------------

def test_unknown_gland_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".exocrine_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # "sebaceous_gland" is a real exocrine gland of the skin (it secretes
            # sebum), but it is NOT one of the salivary/sweat glands this library
            # models, so it is deliberately absent — the engine must abstain rather
            # than fabricate a characteristic.
            fh.write(f'import "exocrine-gland-edges.adj"\n? {REL}(sebaceous_gland, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded gland must abstain"
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
    print(f"\ntest_exocrine_gland_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
