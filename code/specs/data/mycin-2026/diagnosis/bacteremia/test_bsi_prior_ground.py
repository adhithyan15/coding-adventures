#!/usr/bin/env python3
"""test_bsi_prior_ground.py — guard the BSI prior write gate (G5)."""
import json
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bsi_prior_ground as g  # noqa: E402

def test_gate():
    if not g.GROUNDING.exists():
        print("test_bsi_prior_ground: SKIPPED (no grounding)")
        return
    assert g.build(check=False) == 0
    man = json.loads((HERE / "bsi-prior-manifest.json").read_text())["clauses"]
    assert {c["organism"] for c in man.values()} >= {"s_aureus", "enteric_gnb", "candida"}
    for cid, c in man.items():
        assert c["verdict"] in ("ACCEPT", "FLAG")
        assert 0.0 <= c["value"] <= 1.0, (cid, c["value"])   # a prior is a probability
        if c["verdict"] == "ACCEPT":
            assert c["url"] and c["byte_quote"], f"{cid}: grounded prior needs a cited source"
    assert g.build(check=True) == 0, "manifest stale vs grounding"
    acc = sum(1 for c in man.values() if c["verdict"] == "ACCEPT")
    print(f"test_bsi_prior_ground: PASS ({acc} grounded priors, all probabilities, cited)")

if __name__ == "__main__":
    test_gate()
