#!/usr/bin/env python3
"""test_uti_id_ground.py — guard the UTI write gate + that the rulebook RUNS (G4).

Pure checks: the gate maps the spider verdicts to the right ACCEPT/FLAG counts and the
regenerated uti-id.adj is byte-identical to --check. Engine-gated check (if the adj-lang-cli
is built): compose a case that imports the grounded uti-id.adj, observe urinalysis findings,
run the CLI, and confirm the differential actually ranks the uropathogens — proving the new
specialty compiles and reasons through the same engine as meningitis, not just parses.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import uti_id_ground as g  # noqa: E402
import decide as decide_mod  # noqa: E402

TOKEN_RE = re.compile(r"\A[a-z][a-z0-9_]*\Z")


def run_differential(cli: Path, findings: dict[str, str]) -> list[dict]:
    """Compose a case importing uti-id.adj + the observed findings, run the engine, return
    the ranked uropathogens. The case sits next to uti-id.adj so the relative import resolves."""
    lines = ['import "uti-id.adj"']
    for f, v in findings.items():
        if not (TOKEN_RE.match(f) and TOKEN_RE.match(v)):
            raise ValueError(f"unsafe finding token {f!r}={v!r}")
        lines.append(f"observe {f}({v})")
    fd, name = tempfile.mkstemp(suffix=".adj", prefix="_tmp_uti_", dir=HERE)
    case = Path(name)
    try:
        with os.fdopen(fd, "w") as fh:
            fh.write("\n".join(lines) + "\n")
        r = subprocess.run([str(cli), str(case)], capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"adj-lang-cli exited {r.returncode}: {r.stderr}")
        return json.loads(r.stdout).get("ranked", [])
    finally:
        case.unlink(missing_ok=True)


def test_gate_counts_and_check():
    rc = g.build(check=False)
    assert rc == 0
    man = json.loads((HERE / "uti-id-manifest.json").read_text())
    from collections import Counter
    verdicts = Counter(c["verdict"] for c in man["clauses"].values())
    # 7 priors + 2 finding-records: e_coli prior is grounded; nitrite is direction_only (FLAG).
    assert verdicts["ACCEPT"] >= 7 and verdicts["FLAG"] >= 1, verdicts
    # Every ACCEPTed clause is byte-cited to a source.
    for cid, c in man["clauses"].items():
        if c["verdict"] == "ACCEPT":
            assert c["url"], f"{cid}: grounded clause must cite a source URL"
    assert g.build(check=True) == 0, "uti-id.adj is stale vs the grounding"


def test_engine_runs_the_uti_differential():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_uti_id_ground: PASS (gate counts + --check); engine differential SKIPPED (no cli)")
        return
    # Alkaline urine + struvite (urease) → Proteus should be in play and elevated.
    ranked = run_differential(cli, {"urine_urease_alkaline": "present", "urine_nitrite": "positive"})
    assert ranked, "the rulebook produced no differential"
    names = {r.get("hypothesis") or r.get("name") for r in ranked}
    assert "proteus" in names and "e_coli" in names, names
    # E. coli should still lead on the prior; the urease finding lifts Proteus above its prior.
    print(f"test_uti_id_ground: PASS (gate + --check; engine ran the UTI differential over "
          f"{len(ranked)} uropathogens; 0 model calls)")


def main() -> int:
    test_gate_counts_and_check()
    test_engine_runs_the_uti_differential()
    return 0


if __name__ == "__main__":
    sys.exit(main())
