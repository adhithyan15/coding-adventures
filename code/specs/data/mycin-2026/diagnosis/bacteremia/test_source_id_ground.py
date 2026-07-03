#!/usr/bin/env python3
"""test_source_id_ground.py — guard the G5c source-id regeneration gate."""
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import source_id_ground as g  # noqa: E402

def test_regen():
    if not (g.PRIOR_MANIFEST.exists() and g.SRCLR_MANIFEST.exists()):
        print("test_source_id_ground: SKIPPED (no manifests)")
        return
    assert g.build(check=False) == 0
    adj = (HERE / "source-id.adj").read_text()
    # The grounded prior VALUES now appear in the rulebook at trust authoritative.
    assert "prior 0.2 for s_aureus" in adj and "prior 0.291 for enteric_gnb" in adj
    assert "trust authoritative" in adj
    # No conflict markers / well-formed; closed vocab queries present for all 9 organisms.
    assert adj.count("? ") >= 9 and "rulebook source_id {" in adj
    # Flagged/pending clauses are carried + marked, not dropped.
    assert "[FLAG:" in adj and "pending grounding" in adj
    assert g.build(check=True) == 0, "source-id.adj is stale vs the manifests"
    print("test_source_id_ground: PASS (grounded priors used at trust authoritative; "
          "flagged/pending clauses carried + marked; --check up to date)")

if __name__ == "__main__":
    test_regen()
