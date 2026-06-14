#!/usr/bin/env python3
"""test_ci_ground.py — guard the CC-3 contraindication/interaction rule gate."""
import json
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ci_ground as g  # noqa: E402

def test_gate_emits_grounded_rules():
    if not g.GROUNDING.exists():
        print("test_ci_ground: SKIPPED (no grounding file)")
        return
    assert g.build(check=False) == 0
    man = json.loads((HERE / "treatment-constraints.json").read_text())
    rules = man["rules"]
    # Every structural rule is present with a gate verdict; grounded ones carry a source.
    assert set(rules) == set(g.EFFECTS)
    for cid, r in rules.items():
        assert r["verdict"] in ("ACCEPT", "FLAG")
        if r["verdict"] == "ACCEPT":
            assert r["url"] and r["byte_quote"], f"{cid}: grounded rule needs a cited byte-quote"
    assert g.build(check=True) == 0, "treatment-constraints.json is stale vs the grounding"
    print(f"test_ci_ground: PASS ({sum(1 for r in rules.values() if r['verdict']=='ACCEPT')} grounded, "
          f"{sum(1 for r in rules.values() if r['verdict']=='FLAG')} flagged)")

if __name__ == "__main__":
    test_gate_emits_grounded_rules()
