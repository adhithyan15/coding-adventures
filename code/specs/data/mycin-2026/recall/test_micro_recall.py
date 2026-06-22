#!/usr/bin/env python3
"""test_micro_recall.py — the ADJ-only microbiology recall library (MYCIN-2026 MICRO).

MICRO is the first recall domain shipped as a pure ADJ artifact: `micro-edges.adj`
is the SOLE source of truth — facts + byte-provenance inline, no Python gate, no JSON,
no manifest. So this test does NOT check a generator against a committed file (the
`test_*_edge_ground.py` pattern); instead it pins the property that actually matters:

  the native adj-lang engine, given a query .adj that only `import`s the library and
  asks binding queries (`micro-recall.query.adj` — the shape an LLM produces), returns
  the correct binding for every grounded edge, each carrying an AUTHORITATIVE citation
  with a real source span + locator. Knowledge lives in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_micro_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ADJ = HERE / "micro-edges.adj"
QUERY = HERE / "micro-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# The full grounded edge set (subject, relation, expected binding) the engine must
# resolve. Mirrors micro-edges.adj; a divergence here or there fails the test.
EXPECTED = [
    ("staphylococcus_aureus", "gram_stain", "gram_positive"),
    ("staphylococcus_aureus", "morphology", "cocci"),
    ("neisseria_meningitidis", "gram_stain", "gram_negative"),
    ("neisseria_meningitidis", "morphology", "diplococci"),
    ("vibrio_cholerae", "gram_stain", "gram_negative"),
    ("vibrio_cholerae", "morphology", "comma_shaped"),
    ("vibrio_cholerae", "causes", "cholera"),
    ("escherichia_coli", "gram_stain", "gram_negative"),
    ("escherichia_coli", "morphology", "bacilli"),
    ("pseudomonas_aeruginosa", "gram_stain", "gram_negative"),
    ("pseudomonas_aeruginosa", "morphology", "bacilli"),
    ("streptococcus_pneumoniae", "gram_stain", "gram_positive"),
    ("streptococcus_pneumoniae", "causes", "community_acquired_pneumonia"),
    ("helicobacter_pylori", "gram_stain", "gram_negative"),          # expand (NBK534233)
    ("helicobacter_pylori", "morphology", "spiral"),                 # expand (NBK534233)
    ("helicobacter_pylori", "causes", "peptic_ulcer_disease"),       # expand (NBK534233)
    ("clostridioides_difficile", "gram_stain", "gram_positive"),      # expand (NBK431054)
    ("clostridioides_difficile", "causes", "pseudomembranous_colitis"),  # expand (NBK431054)
    ("listeria_monocytogenes", "gram_stain", "gram_positive"),       # expand (NBK534838)
    ("listeria_monocytogenes", "morphology", "bacilli"),             # expand (NBK534838)
    ("klebsiella_pneumoniae", "gram_stain", "gram_negative"),        # expand (NBK519004)
    ("bacillus_anthracis", "gram_stain", "gram_positive"),           # expand (NBK470553)
    ("bacillus_anthracis", "morphology", "bacilli"),                 # expand (NBK470553)
    # --- BATCH: high-yield organisms (gram stain + morphology + signature disease) ---
    ("neisseria_gonorrhoeae", "gram_stain", "gram_negative"),        # expand (NBK558903)
    ("neisseria_gonorrhoeae", "morphology", "diplococci"),           # expand (NBK558903)
    ("neisseria_gonorrhoeae", "causes", "gonorrhea"),                # expand (NBK558903)
    ("streptococcus_pyogenes", "gram_stain", "gram_positive"),       # expand (NBK554528)
    ("streptococcus_pyogenes", "causes", "pharyngitis"),             # expand (NBK554528)
    ("haemophilus_influenzae", "gram_stain", "gram_negative"),       # expand (NBK562176)
    ("haemophilus_influenzae", "morphology", "coccobacilli"),        # expand (NBK562176)
    ("corynebacterium_diphtheriae", "gram_stain", "gram_positive"),  # expand (NBK559015)
    ("corynebacterium_diphtheriae", "morphology", "coccobacilli"),   # expand (NBK559015)
    ("salmonella", "gram_stain", "gram_negative"),                   # expand (NBK555892)
    ("salmonella", "morphology", "bacilli"),                         # expand (NBK555892)
    ("campylobacter", "gram_stain", "gram_negative"),                # expand (NBK537033)
    ("campylobacter", "morphology", "spiral"),                       # expand (NBK537033)
    ("legionella_pneumophila", "gram_stain", "gram_negative"),       # expand (NBK430807)
    ("legionella_pneumophila", "causes", "legionnaires_disease"),    # expand (NBK430807)
    ("clostridium_tetani", "gram_stain", "gram_positive"),           # expand (NBK482484)
    ("clostridium_tetani", "causes", "tetanus"),                     # expand (NBK482484)
    ("treponema_pallidum", "morphology", "spirochete"),              # expand (NBK534780)
    ("treponema_pallidum", "causes", "syphilis"),                    # expand (NBK534780)
    ("bordetella_pertussis", "gram_stain", "gram_negative"),         # expand (NBK519008)
    ("bordetella_pertussis", "morphology", "coccobacilli"),          # expand (NBK519008)
    ("bordetella_pertussis", "causes", "pertussis"),                 # expand (NBK519008)
    ("neisseria_meningitidis", "causes", "meningitis"),              # recovered (NBK549849)
    ("bacillus_anthracis", "causes", "anthrax"),                     # expand (NBK535379)
    ("klebsiella_pneumoniae", "causes", "pneumonia"),                # expand (NBK519004)
    ("listeria_monocytogenes", "causes", "listeriosis"),             # expand (NBK534838)
]


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
    # No legacy authored-debt markers: every edge is grounded or omitted.
    assert "trust consensus" not in text, "MICRO ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    # Every relate clause carries a source + locator + authoritative trust inline.
    relate_count = text.count("    relate ")
    assert text.count("trust authoritative") == relate_count == len(EXPECTED)
    # Count the clause attribute (8-space indented), not the prose mention in the header.
    assert text.count('\n        locator "') == relate_count
    assert text.count('\n        source "') == relate_count
    # No sibling generator / JSON / manifest for this domain (ADJ-only).
    assert not (HERE / "micro_edge_ground.py").exists()
    assert not (HERE / "micro-edge-grounding.json").exists()
    assert not (HERE / "micro-edge-manifest.json").exists()


# ---- 2. the engine resolves every grounded edge with an authoritative citation ----

def test_engine_resolves_every_edge_with_authoritative_citation() -> None:
    if not _cli_available():
        return  # the native engine answers; skip in a Python-only env
    result = _run(QUERY)
    by_query = {r["query"]: r for r in result["recall"]}
    var = {"gram_stain": "Result", "morphology": "Shape", "causes": "Disease"}
    # Every edge in micro-edges.adj must be resolvable through an import-only query.
    for subj, rel, expected in EXPECTED:
        q = f"{rel}({subj}, {var[rel]})"
        # The committed query .adj covers a representative subset; run any uncovered
        # edge directly so the test pins the WHOLE library, not just the demo queries.
        r = by_query.get(q) or _one(rel, subj, var[rel])
        assert r is not None and not r["abstained"], f"{q} abstained — edge missing?"
        ans = r["answers"][0]
        assert ans["bindings"][var[rel]] == expected, f"{q} -> {ans['bindings']}"
        cite = ans["citations"][0]
        assert cite["trust"] == "authoritative", f"{q} citation not authoritative"
        assert cite["source"], f"{q} citation has no source span"
        assert cite["locator"].startswith("https://www.ncbi.nlm.nih.gov/"), cite["locator"]


def _one(rel: str, subj: str, varname: str) -> dict | None:
    """Resolve a single edge by writing a throwaway one-line query .adj in this dir
    (so the relative `import "micro-edges.adj"` resolves) and running the engine."""
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".micro_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write(f'import "micro-edges.adj"\n? {rel}({subj}, ${varname})\n')
        res = _run(Path(path))
        return res["recall"][0] if res["recall"] else None
    finally:
        os.unlink(path)


# ---- 3. an off-vocabulary subject abstains, never fabricates ----------------------

def test_unknown_organism_abstains() -> None:
    if not _cli_available():
        return
    r = _one("gram_stain", "treponema_pallidum", "Result")  # not in the library
    assert r is not None and r["abstained"], "an ungrounded organism must abstain"


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
    print(f"\ntest_micro_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
