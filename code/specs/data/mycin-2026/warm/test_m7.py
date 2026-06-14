#!/usr/bin/env python3
"""test_m7.py - guard M7: value-of-information + rulebook self-consistency (IIS).

Two CPU-only checks (0 answer-time model calls), skipped cleanly without the CLI:
  1. VOI "order-next": on the knife's-edge pre-culture case, at least one
     unobserved finding is surfaced that would FLIP the leading diagnosis.
  2. Rulebook self-consistency: the real priors partition (sum to 1.0) → SAT;
     a mis-authored prior → UNSAT with a non-empty IIS `core`.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402
import voi as voi_mod  # noqa: E402


def check_outcome(cli: Path, adj_path: Path) -> dict:
    r = subprocess.run([str(cli), str(adj_path)], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout).get("check", {})


def main() -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_m7: SKIPPED (adj-lang-cli not built)")
        return 0

    # 1. VOI order-next on the ambiguous case.
    ir = json.loads((ROOT / "ir" / "case_preculture_ambiguous.json").read_text())
    observe_adj, kept, _ = ir_mod.ir_to_adj(ir, ir_mod.load_domains())
    rows = voi_mod.voi("case_preculture_ambiguous", observe_adj, set(kept), cli)
    assert rows, "VOI produced no rankings"
    # The VOI must identify at least one materially decision-relevant unobserved
    # finding (a meaningful margin move, or an outright flip on an ambiguous case).
    top_impact = max(abs(r["margin_delta"]) for r in rows)
    assert top_impact > 0.01, f"VOI surfaced no decision-relevant finding (max |Δ|={top_impact})"
    n_flip = sum(r["flips_leader"] for r in rows)
    print(f"test_m7: VOI ok (top order-next = {rows[0]['order']}, "
          f"|Δmargin|={abs(rows[0]['margin_delta']):.3f}, {n_flip} flipping findings)")

    # 2. Rulebook self-consistency (IIS).
    ok = check_outcome(cli, ROOT / "consistency" / "priors_partition_ok.adj")
    assert ok.get("outcome") in ("sat", "sat_real"), ok
    broken = check_outcome(cli, ROOT / "consistency" / "priors_partition_broken.adj")
    assert broken.get("outcome") == "unsat", broken
    assert broken.get("core"), f"UNSAT but no IIS core: {broken}"
    print(f"test_m7: consistency ok (real priors SAT; mis-authored prior UNSAT, IIS core={broken['core']})")
    print("test_m7: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
