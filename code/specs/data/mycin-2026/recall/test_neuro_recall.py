#!/usr/bin/env python3
"""test_neuro_recall.py — the ADJ-only neuroanatomical-localization library (MYCIN-2026 NEURO).

The ninth recall domain shipped as a pure ADJ artifact (fifth Tier-2 organ-system domain,
after RHEUM, ONCO, HISTO, CARDIO). `neuro-edges.adj` is the SOLE source of truth — facts +
byte-provenance inline, no Python gate, no JSON, no manifest. This test pins the property
that matters: the native adj-lang engine, given a query .adj that only `import`s the
library and asks binding queries (`neuro-recall.query.adj` — the shape an LLM produces),
binds the grounded deficit for each lesion site, each carrying an AUTHORITATIVE citation
with a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_neuro_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "neuro-edges.adj"
QUERY = HERE / "neuro-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: lesion site -> the deficit the library binds for it.
EXPECTED = {
    "wernicke_area": "fluent_aphasia",
    "broca_area": "nonfluent_aphasia",
    "arcuate_fasciculus": "conduction_aphasia",
    "substantia_nigra": "parkinson_disease",
    "subthalamic_nucleus": "hemiballismus",
    "caudate_nucleus": "huntington_disease",
    "midline_cerebellum": "imbalance",              # expand (NBK562317)
    "cerebellar_hemisphere": "incoordination",      # expand (NBK562317)
    # --- BATCH: high-yield neuroanatomy localizations ---
    "mammillary_bodies": "wernicke_korsakoff_syndrome",   # expand (NBK430729)
    "amygdala": "kluver_bucy_syndrome",                   # expand (NBK544221)
    "hippocampus": "anterograde_amnesia",                 # expand (NBK537247)
    "medial_longitudinal_fasciculus": "internuclear_ophthalmoplegia",  # expand (NBK441970)
    "internal_capsule": "pure_motor_hemiparesis",         # expand (NBK563216)
    "dominant_parietal_lobe": "gerstmann_syndrome",       # expand (NBK519528)
    "nondominant_parietal_lobe": "hemispatial_neglect",   # expand (NBK537247)
    "frontal_lobe": "personality_change",                 # expand (NBK532981)
    "posterior_columns": "loss_of_proprioception",        # expand (NBK507888)
}
REL = "lesion_causes"
VAR = "Deficit"


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
    assert "trust consensus" not in text, "NEURO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 6
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    assert not (HERE / "neuro_edge_ground.py").exists()
    assert not (HERE / "neuro-edge-grounding.json").exists()
    assert not (HERE / "neuro-edge-manifest.json").exists()


# ---- 2. the engine binds each deficit, each with an authoritative citation ----

def test_engine_binds_every_deficit_with_authoritative_citation() -> None:
    if not _cli_available():
        return
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    for site, deficit in EXPECTED.items():
        q = f"{REL}({site}, {VAR})"
        r = by_query.get(q)
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        bound = {}
        for a in r["answers"]:
            bound[a["bindings"][VAR]] = (a.get("citations") or [{}])[0]
        assert set(bound) == {deficit}, f"{site}: bound {set(bound)} != {{{deficit}}}"
        cite = bound[deficit]
        assert cite.get("trust") == "authoritative", f"{site}/{deficit} not authoritative"
        assert cite.get("source"), f"{site}/{deficit} has no source span"
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary lesion site abstains, never fabricates -------------------

def test_unknown_site_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".neuro_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # pineal_gland is not in the library → must abstain
            fh.write(f'import "neuro-edges.adj"\n? {REL}(pineal_gland, ${VAR})\n')
        res = _run(Path(path))
        r = res["recall"][0] if res["recall"] else None
        assert r is not None and r["abstained"], "an ungrounded lesion site must abstain"
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
    print(f"\ntest_neuro_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
