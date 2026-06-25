#!/usr/bin/env python3
"""test_hypothalamic_hormone_recall.py — the ADJ-only
hypothalamic-releasing-hormone→pituitary-hormone recall library
(MYCIN-2026 HYPOTHALAMIC).

A pure-ADJ recall domain (high-yield endocrine-physiology board table): the
anterior-pituitary hormone whose release each hypothalamic releasing hormone
stimulates. `hypothalamic-hormone-edges.adj` is the SOLE source of truth — facts
+ byte-provenance inline, no Python gate, no JSON, no manifest. This test pins
the property that matters: the native adj-lang engine, given a query .adj that
only `import`s the library and asks binding queries
(`hypothalamic-hormone-recall.query.adj` — the shape an LLM produces), binds the
grounded pituitary hormone for each releasing hormone, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in
ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks
skip — the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_hypothalamic_hormone_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "hypothalamic-hormone-edges.adj"
QUERY = HERE / "hypothalamic-hormone-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: hypothalamic releasing hormone -> the pituitary hormone
# the library binds.
EXPECTED = {
    "trh": "tsh",            # NBK499850
    "crh": "acth",           # NBK500031
    "gnrh": "fsh_and_lh",    # NBK535442
    "ghrh": "gh",            # NBK459247
}
REL = "stimulates_release_of"
VAR = "PituitaryHormone"


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
    assert "trust consensus" not in text, "HYPOTHALAMIC ships no authored-debt; ground or omit"
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
    assert not (HERE / "hypothalamic_hormone_edge_ground.py").exists()
    assert not (HERE / "hypothalamic-hormone-edge-grounding.json").exists()
    assert not (HERE / "hypothalamic-hormone-edge-manifest.json").exists()


# ---- 2. the engine binds each pituitary hormone with an authoritative citation -----

def test_engine_binds_every_target_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for releasing, pituitary in EXPECTED.items():
        q = f"{REL}({releasing}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {pituitary}, f"{releasing}: bound {set(bound)} != {{{pituitary}}}"
        cite = bound[pituitary]
        assert cite.get("trust") == "authoritative", f"{releasing}/{pituitary} not authoritative"
        assert cite.get("source"), f"{releasing}/{pituitary} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary releasing hormone abstains, never fabricates -------------

def test_unknown_hormone_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".hypothalamic_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # ghih (somatostatin / growth-hormone-inhibiting hormone) is a real
            # hypothalamic hormone but is inhibitory and deliberately not in this
            # library, so the engine must abstain rather than fabricate.
            fh.write(f'import "hypothalamic-hormone-edges.adj"\n? {REL}(ghih, ${VAR})\n')
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
    print(f"\ntest_hypothalamic_hormone_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
