#!/usr/bin/env python3
"""test_ecg_recall.py — the ADJ-only electrocardiogram recall library (MYCIN-2026 ECG).

A new pure-ADJ recall domain (core cardiology/IM board content): the classic ECG
waveform finding and the diagnosis it points to. `ecg-edges.adj` is the SOLE source
of truth — facts + byte-provenance inline, no Python gate, no JSON, no manifest. This
test pins the property that matters: the native adj-lang engine, given a query .adj
that only `import`s the library and asks binding queries (`ecg-recall.query.adj` — the
shape an LLM produces), binds the grounded diagnosis for each ECG finding, each
carrying an AUTHORITATIVE citation with a real source span + locator. Knowledge lives
in ADJ; the engine answers.

If the native CLI isn't built (a Python-only CI lane), the engine-backed checks skip —
the file-shape checks (every clause grounded, no authored-debt) still run.

Run:  python3 test_ecg_recall.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlsplit

HERE = Path(__file__).resolve().parent
ADJ = HERE / "ecg-edges.adj"
QUERY = HERE / "ecg-recall.query.adj"
CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"

# Every grounded edge: ECG finding -> the diagnosis the library binds for it.
EXPECTED = {
    "delta_wave": "wolff_parkinson_white",                # NBK554437
    "sawtooth_waves": "atrial_flutter",                   # NBK482421
    "absent_p_waves": "atrial_fibrillation",              # NBK526072
    "peaked_t_waves": "hyperkalemia",                     # NBK538264
    "prolonged_qt_interval": "long_qt_syndrome",          # NBK441860
    "st_segment_elevation": "myocardial_infarction",      # NBK532281
    "epsilon_wave": "arrhythmogenic_right_ventricular_cardiomyopathy",  # NBK470378
    "electrical_alternans": "cardiac_tamponade",          # NBK534229
}
REL = "ecg_finding_indicates"
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
    assert "trust consensus" not in text, "ECG ships no authored-debt; ground or omit"
    assert "[FLAG:" not in text
    edge_count = len(EXPECTED)  # 8
    assert text.count("    relate ") == edge_count
    assert text.count("trust authoritative") == edge_count
    assert text.count('\n        locator "') == edge_count
    assert text.count('\n        source "') == edge_count
    # every locator is an NCBI primary source
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("locator "):
            locator_url = line.split('"')[1] if '"' in line else line
            assert urlsplit(locator_url).hostname == "www.ncbi.nlm.nih.gov", f"non-NCBI locator: {line}"
    assert not (HERE / "ecg_edge_ground.py").exists()
    assert not (HERE / "ecg-edge-grounding.json").exists()
    assert not (HERE / "ecg-edge-manifest.json").exists()


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
        assert cite.get("locator", "").startswith("https://www.ncbi.nlm.nih.gov/")


# ---- 3. an off-vocabulary finding abstains, never fabricates -----------------------

def test_unknown_finding_abstains() -> None:
    if not _cli_available():
        return
    import os
    import tempfile
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".ecg_q_", dir=HERE)
    try:
        with os.fdopen(fd, "w") as fh:
            # u_waves is not in the library (deferred — see header) -> must abstain
            fh.write(f'import "ecg-edges.adj"\n? {REL}(u_waves, ${VAR})\n')
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
    print(f"\ntest_ecg_recall: {len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(_run_all())
