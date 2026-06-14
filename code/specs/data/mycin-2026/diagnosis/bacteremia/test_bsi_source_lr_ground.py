#!/usr/bin/env python3
"""test_bsi_source_lr_ground.py — guard the BSI portal-of-entry LR gate (G5b)."""
import json
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bsi_source_lr_ground as g  # noqa: E402

def test_gate():
    if not g.GROUNDING.exists():
        print("test_bsi_source_lr_ground: SKIPPED (no grounding)")
        return
    assert g.build(check=False) == 0
    man = json.loads((HERE / "bsi-source-lr-manifest.json").read_text())["clauses"]
    assert set(man) == {cid for cid, *_ in g.SOURCE_LRS}
    for cid, c in man.items():
        assert c["verdict"] in ("ACCEPT", "FLAG")
        assert isinstance(c["lr"], int) and c["lr"] > 0      # structural LR magnitude
        assert c["evidence"] and c["organism"]
        if c["verdict"] == "ACCEPT":
            assert c["url"] and c["byte_quote"], f"{cid}: grounded association needs a cited source"
    assert g.build(check=True) == 0, "manifest stale vs grounding"
    acc = sum(1 for c in man.values() if c["verdict"] == "ACCEPT")
    print(f"test_bsi_source_lr_ground: PASS ({acc} grounded source LRs, cited; --check)")

if __name__ == "__main__":
    test_gate()
