#!/usr/bin/env python3
"""test_pituitary_hormone_recall.py — the ADJ-only pituitary-hormone→target
recall library (MYCIN-2026 PITUITARY).

A pure-ADJ recall domain (high-yield endocrine-physiology board table): the
target organ/effect each major pituitary hormone acts on.
`pituitary-hormone-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the
property that matters: the native adj-lang engine, given a query .adj that only
`import`s the library and asks binding queries (`pituitary-hormone-recall.query.adj`
— the shape an LLM produces), binds the grounded target for each hormone, each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge
lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_pituitary_hormone_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "pituitary-hormone-edges.adj"
QUERY = HERE / "pituitary-hormone-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: pituitary hormone -> the target the library binds.
EXPECTED = {
    "tsh": "thyroid_gland",                      # NBK499898
    "acth": "adrenal_cortex",                    # NBK500031
    "prolactin": "mammary_gland",                # NBK507829
    "adh": "promotes_water_reabsorption",        # NBK526069
    "lh": "stimulates_leydig_cells",             # NBK539692
    "fsh": "stimulates_follicle_growth",         # NBK535442
}
REL = "targets"
VAR = "Target"


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
    assert "trust consensus" not in text, "PITUITARY ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "pituitary_hormone_edge_ground.py").exists()
    assert not (HERE / "pituitary-hormone-edge-grounding.json").exists()
    assert not (HERE / "pituitary-hormone-edge-manifest.json").exists()


# ---- 2. the engine binds each target with an authoritative citation ----------------

def test_engine_binds_every_target_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for hormone, target in EXPECTED.items():
        q = f"{REL}({hormone}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {target}, f"{hormone}: bound {set(bound)} != {{{target}}}"
        cite = bound[target]
        assert cite.get("trust") == "authoritative", f"{hormone}/{target} not authoritative"
        assert cite.get("source"), f"{hormone}/{target} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary hormone abstains, never fabricates ------------------------

def test_unknown_hormone_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".pituitary_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # oxytocin is a real posterior-pituitary hormone but is deliberately
            # not in this library, so the engine must abstain rather than
            # fabricate.
            fh.write(f'import "pituitary-hormone-edges.adj"\n? {REL}(oxytocin, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded hormone must abstain"
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
    print(f"\ntest_pituitary_hormone_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
