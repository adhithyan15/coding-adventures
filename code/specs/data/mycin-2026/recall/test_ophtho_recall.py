#!/usr/bin/env python3
"""test_ophtho_recall.py — the ADJ-only ophthalmology recall library (MYCIN-2026 OPHTHO).

A new pure-ADJ recall domain (high-yield ophthalmology/IM board content): the eye
finding (fundoscopic sign, pupillary reflex, corneal/lens finding) and the diagnosis it
points to. `ophtho-edges.adj` is the SOLE source of truth — facts + byte-provenance
inline, no Python gate, no JSON, no manifest. This test pins the property that matters:
the native adj-lang engine, given a query .adj that only `import`s the library and asks
binding queries (`ophtho-recall.query.adj` — the shape an LLM produces), binds the
grounded diagnosis for each eye finding, each carrying an AUTHORITATIVE citation with a
real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_ophtho_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "ophtho-edges.adj"
QUERY = HERE / "ophtho-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: eye finding -> the diagnosis the library binds for it.
EXPECTED = {
    "cherry_red_spot": "central_retinal_artery_occlusion",   # NBK539841
    "roth_spots": "infective_endocarditis",                  # PMC7417078
    "leukocoria": "retinoblastoma",                          # NBK545276
    "optic_disc_cupping": "glaucoma",                        # NBK441887
    "kayser_fleischer_rings": "wilson_disease",              # NBK459187
    "papilledema": "increased_intracranial_pressure",       # NBK538295
    "retinal_neovascularization": "proliferative_diabetic_retinopathy",  # NBK278967
    "superior_lens_dislocation": "marfan_syndrome",          # NBK578193
    # --- write-once / use-many: four findings from ONE NBK525980 span ---
    "cotton_wool_spots": "hypertensive_retinopathy",         # NBK525980
    "flame_shaped_hemorrhages": "hypertensive_retinopathy",  # NBK525980 (same span)
    "arteriovenous_crossing_changes": "hypertensive_retinopathy",  # NBK525980 (same span)
    "macular_star": "hypertensive_retinopathy",              # NBK525980 (same span)
}
# Every edge cites a primary NCBI authority; we accept StatPearls/Bookshelf and
# PubMed Central (the Roth-spots edge has no StatPearls span and lives on PMC).
_AUTH_HOSTS = (
    "https://www.ncbi.nlm.nih.gov/",
    "https://pmc.ncbi.nlm.nih.gov/",
)
REL = "eye_finding_indicates"
VAR = "Diagnosis"


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
    assert "trust consensus" not in text, "OPHTHO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 12
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    # every locator is a recognized NCBI primary host
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            assert any(h in line for h in _AUTH_HOSTS), f"non-NCBI locator: {line}"
    assert not (HERE / "ophtho_edge_ground.py").exists()
    assert not (HERE / "ophtho-edge-grounding.json").exists()
    assert not (HERE / "ophtho-edge-manifest.json").exists()


# ---- 2. the engine binds each diagnosis, each with an authoritative citation ----

def test_engine_binds_every_diagnosis_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for finding, diagnosis in EXPECTED.items():
        q = f"{REL}({finding}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {diagnosis}, f"{finding}: bound {set(bound)} != {{{diagnosis}}}"
        cite = bound[diagnosis]
        assert cite.get("trust") == "authoritative", f"{finding}/{diagnosis} not authoritative"
        assert cite.get("source"), f"{finding}/{diagnosis} has no source span"
        assert cite.get("locator", "").startswith(_AUTH_HOSTS), f"{finding} locator not a primary host"


# ---- 3. an off-vocabulary finding abstains, never fabricates -----------------------

def test_unknown_finding_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".ophtho_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # hollenhorst_plaque is not in the library (deferred — see header) -> must abstain
            fh.write(f'import "ophtho-edges.adj"\n? {REL}(hollenhorst_plaque, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded finding must abstain"
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
    print(f"\ntest_ophtho_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
