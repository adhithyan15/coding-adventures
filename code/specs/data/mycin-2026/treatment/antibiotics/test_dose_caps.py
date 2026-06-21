#!/usr/bin/env python3
"""test_dose_caps.py — guard the ADJ-native conjunctive dose-cap rulebook + runtime (CC-2b).

Pure checks (no engine): the generator emits a parseable, grounded rulebook and `--check`
agrees with the committed file; the runtime's injection guard refuses unsafe risk tokens.
Engine-gated checks (if adj-lang-cli is built): the engine derives the `hepatorenal` compound
risk ONLY when BOTH a hepatic and a renal risk are active (a single risk derives nothing —
faithful to "hepatic impairment alone needs no adjustment"), the grounded FDA byte-quote flows
through to the capped drug, and an unrelated single risk caps nothing — proving the conjunction
reasoning lives in the language, not in Python.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent.parent / "warm"))
import dose_caps as dc  # noqa: E402
import dose_caps_build as dcb  # noqa: E402
import decide as decide_mod  # noqa: E402


# --------------------------------------------------------------------------
# Generator (pure) — the rulebook is generated, grounded, and `--check`-stable.
# --------------------------------------------------------------------------
def test_generator_check_is_up_to_date():
    # The committed dose_caps.adj + manifest must match a fresh regeneration, so the CAS
    # artifact is never hand-edited out of sync with the grounding (mirrors contraindications).
    assert dcb.build(check=True) == 0, "dose_caps.adj is OUT OF DATE — regenerate it"


def test_generated_rulebook_is_grounded_and_parseable():
    text = dcb.ADJ.read_text()
    # The grounded cap fact carries the FDA byte-quote at authoritative trust + a DailyMed URL.
    assert "dose_capped_under(ceftriaxone, hepatorenal)" in text
    assert "hepatic impairment and significant renal impairment" in text
    assert "trust authoritative" in text and "dailymed" in text
    # The compound is defined structurally as TWO distinct component categories.
    assert "compound_first(hepatorenal, hepatic)" in text
    assert "compound_second(hepatorenal, renal)" in text
    # The two GENERIC conjunction rules are present (not a drug-specific Python `if`).
    assert text.count("rule {") == 2 and "derived_risk($C)" in text


# --------------------------------------------------------------------------
# Runtime injection guard (pure) — a risk token is a closed-vocabulary symbol.
# --------------------------------------------------------------------------
@pytest.mark.parametrize("bad", ["renal_moderate)\n? evil(", "RENAL", "a b", "x;y", "", "1risk"])
def test_unsafe_risk_tokens_are_rejected(bad):
    with pytest.raises(ValueError):
        dc.derive_dose_caps(_DUMMY_CLI, {bad})


def test_no_active_risks_short_circuits_without_the_engine():
    # An empty risk set must NOT touch the engine (cli unused) and derive nothing.
    assert dc.derive_dose_caps(_DUMMY_CLI, set()) == (set(), {})


class _DummyCli:
    """A CLI path that would fail loudly if ever invoked — proves the guards run first."""
    def __fspath__(self):  # pragma: no cover - never reached when the guard fires
        raise AssertionError("the engine must not be invoked for a rejected/empty risk set")


_DUMMY_CLI = _DummyCli()


# --------------------------------------------------------------------------
# Engine-gated — the ENGINE derives the conjunctive cap (the whole point).
# --------------------------------------------------------------------------
def _cli_or_skip():
    cli = decide_mod.find_cli()
    if cli is None:
        pytest.skip("adj-lang-cli not built")
    return cli


def test_conjunction_requires_both_categories():
    cli = _cli_or_skip()
    # A single organ risk derives NOTHING — hepatic alone, or renal alone, needs no compound
    # cap (faithful to the label: hepatic impairment alone needs no dose adjustment).
    assert dc.derive_dose_caps(cli, {"hepatic_severe"}) == (set(), {})
    assert dc.derive_dose_caps(cli, {"renal_moderate"}) == (set(), {})
    # Two hepatic risks (still no renal) must NOT fire — the rule needs two DISTINCT categories.
    assert dc.derive_dose_caps(cli, {"hepatic_severe", "hepatic_moderate"}) == (set(), {})


def test_both_categories_derive_hepatorenal_with_grounded_provenance():
    cli = _cli_or_skip()
    risks, caps = dc.derive_dose_caps(cli, {"hepatic_severe", "renal_moderate"})
    assert risks == {"hepatorenal"}, risks
    assert set(caps) == {"ceftriaxone"}, caps
    info = caps["ceftriaxone"]
    assert info["risk"] == "hepatorenal"
    # the grounded FDA byte-quote flows through to the cap.
    assert info["trust"] == "authoritative" and info["source"]
    assert "hepatic impairment and significant renal impairment" in info["source"]
    assert info["locator"] and "dailymed" in info["locator"]


def test_graded_severities_all_map_to_their_category():
    cli = _cli_or_skip()
    # Either hepatic grade + either renal grade triggers the compound — the severity→category
    # membership lives in the rulebook (risk_in_category), so the chart can assert any grade.
    for hep in ("hepatic_severe", "hepatic_moderate"):
        for ren in ("renal_severe", "renal_moderate"):
            risks, caps = dc.derive_dose_caps(cli, {hep, ren})
            assert risks == {"hepatorenal"} and "ceftriaxone" in caps, (hep, ren)


def test_unrelated_single_risk_caps_nothing():
    cli = _cli_or_skip()
    # A non-organ risk token (e.g. an interaction) is not in any category → no compound, no cap.
    assert dc.derive_dose_caps(cli, {"nephrotoxin_interaction"}) == (set(), {})


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
