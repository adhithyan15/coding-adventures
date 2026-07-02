#!/usr/bin/env python3
"""test_lung_volume_recall.py — the ADJ-only lung-volume/capacity→definition
recall library (MYCIN-2026 LUNGVOLUME).

A pure-ADJ recall domain (classic high-yield respiratory-physiology board table):
the definition of each static lung volume/capacity. `lung-volume-edges.adj` is
the SOLE source of truth — facts + byte-provenance inline, no Python gate, no
JSON, no manifest. This test pins the property that matters: the native adj-lang
engine, given a query .adj that only `import`s the library and asks binding
queries (`lung-volume-recall.query.adj` — the shape an LLM produces), binds the
grounded definition for each volume, each carrying an AUTHORITATIVE citation with
a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_lung_volume_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "lung-volume-edges.adj"
QUERY = HERE / "lung-volume-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: lung volume/capacity -> the definition the library binds.
EXPECTED = {
    "tidal_volume": "air_per_respiratory_cycle",                                     # NBK482502
    "residual_volume": "air_remaining_after_maximal_forceful_expiration",            # NBK493170
    "inspiratory_reserve_volume": "air_inhaled_after_normal_tidal",                  # NBK560526
    "vital_capacity": "max_air_exhaled_after_max_inspiration",                       # NBK541099
    "functional_residual_capacity": "air_remaining_after_normal_passive_exhalation", # NBK500007
}
REL = "is_defined_as"
VAR = "Definition"


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
    assert "trust consensus" not in text, "LUNGVOLUME ships no authored-debt; ground or omit"
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
    assert not (HERE / "lung_volume_edge_ground.py").exists()
    assert not (HERE / "lung-volume-edge-grounding.json").exists()
    assert not (HERE / "lung-volume-edge-manifest.json").exists()


# ---- 2. the engine binds each definition with an authoritative citation ------------

def test_engine_binds_every_definition_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for volume, definition in EXPECTED.items():
        q = f"{REL}({volume}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {definition}, f"{volume}: bound {set(bound)} != {{{definition}}}"
        cite = bound[definition]
        assert cite.get("trust") == "authoritative", f"{volume}/{definition} not authoritative"
        assert cite.get("source"), f"{volume}/{definition} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary volume abstains, never fabricates -------------------------

def test_unknown_volume_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".lungvolume_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # expiratory_reserve_volume is a real lung volume but was DEFERRED from
            # this library (its only self-contained span names the term by
            # abbreviation alone), so the engine must abstain rather than fabricate.
            fh.write(f'import "lung-volume-edges.adj"\n? {REL}(expiratory_reserve_volume, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded volume must abstain"
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
    print(f"\ntest_lung_volume_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
