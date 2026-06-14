#!/usr/bin/env python3
"""Guard the empiric antibiotic selector: the right components fire per profile."""
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import decide as decide_mod  # noqa: E402
sys.path.insert(0, str(HERE))
import select_abx as s  # noqa: E402

def comps(cli, prof):
    return {c["component"] for c in s.select_components(cli, s.PROFILES[prof])}

def main():
    cli = decide_mod.find_cli()
    if cli is None:
        print("test_abx: SKIPPED (no cli)")
        return 0
    young, elderly, allergic = comps(cli, "young"), comps(cli, "elderly"), comps(cli, "allergic")
    assert young == {"give_vancomycin", "give_ceftriaxone", "give_dexamethasone"}, young
    assert "give_ampicillin" in elderly, elderly            # Listeria cover for age>50
    assert "give_ceftriaxone" not in allergic, allergic     # beta-lactam excluded by allergy
    assert "give_betalactam_sparing_alt" in allergic, allergic
    print("test_abx: PASS (standard adult; +ampicillin elderly; beta-lactam excluded on allergy)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
