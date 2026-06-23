#!/usr/bin/env python3
"""test_tox_recall.py — the ADJ-only toxidrome recall library (MYCIN-2026 TOX).

A new pure-ADJ recall domain (core emergency-medicine/pharmacology board content):
the classic toxidrome and the agent class that causes it. `tox-edges.adj` is the SOLE
source of truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest.
This test pins the property that matters: the native adj-lang engine, given a query .adj
that only `import`s the library and asks binding queries (`tox-recall.query.adj` — the
shape an LLM produces), binds the grounded agent for each toxidrome, each carrying an
AUTHORITATIVE citation with a real source span + locator. Knowledge lives in ADJ; the
engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_tox_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "tox-edges.adj"
QUERY = HERE / "tox-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: toxidrome -> the agent class the library binds for it.
EXPECTED = {
    "cholinergic_toxidrome": "organophosphates",                  # NBK539783
    "anticholinergic_toxidrome": "tricyclic_antidepressants",     # NBK534798
    "sympathomimetic_toxidrome": "cocaine_amphetamines",          # NBK430757
    "opioid_toxidrome": "opioids",                                # NBK470415
    "serotonin_syndrome": "monoamine_oxidase_inhibitors",         # NBK482377
    "sedative_hypnotic_toxidrome": "benzodiazepines",             # NBK482238
}
REL = "toxidrome_caused_by"
VAR = "Agent"


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
    assert "trust consensus" not in text, "TOX ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert "https://www.ncbi.nlm.nih.gov/" in line, f"non-NCBI locator: {line}"
    assert not (HERE / "tox_edge_ground.py").exists()
    assert not (HERE / "tox-edge-grounding.json").exists()
    assert not (HERE / "tox-edge-manifest.json").exists()


# ---- 2. the engine binds each agent, each with an authoritative citation ----------

def test_engine_binds_every_agent_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for toxidrome, agent in EXPECTED.items():
        q = f"{REL}({toxidrome}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {agent}, f"{toxidrome}: bound {set(bound)} != {{{agent}}}"
        cite = bound[agent]
        assert cite.get("trust") == "authoritative", f"{toxidrome}/{agent} not authoritative"
        assert cite.get("source"), f"{toxidrome}/{agent} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary toxidrome abstains, never fabricates ----------------------

def test_unknown_toxidrome_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".tox_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # salicylate_toxicity is not in the library (held — see header) -> must abstain
            fh.write(f'import "tox-edges.adj"\n? {REL}(salicylate_toxicity, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded toxidrome must abstain"
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
    print(f"\ntest_tox_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
