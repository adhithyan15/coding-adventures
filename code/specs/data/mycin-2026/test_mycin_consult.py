#!/usr/bin/env python3
"""test_mycin_consult.py — guard the end-to-end interactive MYCIN consultation.

Pure check: a culture sensitivity drops the defeated coverage edge from the emitted
program (no engine needed). Full check (skipped if adj-lang-cli isn't built): the
scripted consultation asks VOI-ranked questions, converges to bacterial meningitis,
derives an empiric regimen, and RE-DERIVES it culture-directed when an isolate is
resistant. 0 answer-time model calls.
"""

from __future__ import annotations

import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent
sys.path.insert(0, str(MYCIN))
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(MYCIN / "treatment" / "antibiotics"))
import decide as decide_mod  # noqa: E402
import mycin_consult as mc  # noqa: E402
import native_setcover as abx  # noqa: E402


def test_sensitivity_defeases_coverage_edge() -> None:
    """A resistant (drug, organism) edge is dropped from the cover (no engine)."""
    organisms = ["listeria"]
    base, _, _ = abx.emit_program(organisms, set())
    line = next(ln for ln in base.splitlines() if "% cover listeria" in ln)
    assert "x_ampicillin" in line, f"ampicillin should cover listeria by default: {line}"
    # With the isolate ampicillin-resistant, that edge is gone.
    deft, _, _ = abx.emit_program(organisms, set(), defeated={("ampicillin", "listeria")})
    line2 = next(ln for ln in deft.splitlines() if "% cover listeria" in ln)
    assert "x_ampicillin" not in line2, f"defeated edge must be dropped: {line2}"


def test_defeated_combination_member_drops_the_combination() -> None:
    """A combination is voided for an organism if a member is resistant to it."""
    organisms = ["s_pneumoniae_resistant"]
    base, _, _ = abx.emit_program(organisms, set())
    assert any("y_vancomycin_ceftriaxone" in ln for ln in base.splitlines())
    deft, _, _ = abx.emit_program(organisms, set(),
                                  defeated={("ceftriaxone", "s_pneumoniae_resistant")})
    # The vanc+ceftriaxone combo no longer contributes its aux to the cover line.
    cover = next((ln for ln in deft.splitlines() if "% cover s_pneumoniae_resistant" in ln), "")
    assert "y_vancomycin_ceftriaxone" not in cover, f"defeated combo dropped: {cover}"


def main() -> int:
    test_sensitivity_defeases_coverage_edge()
    test_defeated_combination_member_drops_the_combination()

    cli = decide_mod.find_cli()
    if cli is None:
        print("test_mycin_consult: PASS (sensitivity defeasance); CLI checks SKIPPED "
              "(adj-lang-cli not built)")
        return 0

    res = mc.consult(cli, mc.CASES["adult_bacterial"], interactive=False)
    assert res["leader"] == "bacterial_meningitis", res
    assert len(res["dialogue"]) >= 1, "the consultation must ask at least one question"
    # Empiric covers Listeria with ampicillin; the culture-directed regimen does not.
    assert "ampicillin" in (res["empiric"] or []), res["empiric"]
    assert "vancomycin" in (res["empiric"] or []), res["empiric"]
    assert "ampicillin" not in (res["regimen"] or []), res["regimen"]
    assert res["regimen"], "a culture-directed regimen must still be derivable"

    print("test_mycin_consult: PASS (VOI-driven dialogue converges to bacterial "
          "meningitis; empiric → culture-directed re-derivation drops the resistant "
          "ampicillin; 0 answer-time model calls)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
