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
    # SINGLE-FACTOR grounded caps (vancomycin renal / nephrotoxin) reuse dose_capped_under.
    assert "dose_capped_under(vancomycin, renal_severe)" in text
    assert "dose_capped_under(vancomycin, nephrotoxin_interaction)" in text
    # THREE GENERIC rules now: derived_risk + compound dose_capped + single-factor dose_capped.
    assert text.count("rule {") == 3 and "derived_risk($C)" in text


# --------------------------------------------------------------------------
# Runtime injection guard (pure) — a risk token is a closed-vocabulary symbol.
# --------------------------------------------------------------------------
@pytest.mark.parametrize("bad", ["renal_moderate)\n? evil(", "RENAL", "a b", "x;y", "", "1risk"])
def test_unsafe_risk_tokens_are_rejected(bad):
    with pytest.raises(ValueError):
        dc.derive_dose_caps(_DUMMY_CLI, {bad})


def test_no_active_risks_short_circuits_without_the_engine():
    # An empty risk set must NOT touch the engine (cli unused) and derive nothing.
    assert dc.derive_dose_caps(_DUMMY_CLI, set()) == (set(), [])


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


def _drugs(caps):
    return {c["drug"] for c in caps}


def test_compound_requires_both_categories():
    cli = _cli_or_skip()
    # The COMPOUND (derived_risk) needs two DISTINCT organ categories: a single hepatic risk,
    # or two hepatic risks, derive NO compound (faithful: hepatic alone needs no adjustment).
    risks, caps = dc.derive_dose_caps(cli, {"hepatic_severe"})
    assert risks == set() and caps == [], (risks, caps)  # hepatic has no single-factor cap either
    risks2, _ = dc.derive_dose_caps(cli, {"hepatic_severe", "hepatic_moderate"})
    assert risks2 == set(), risks2  # two hepatic, no renal → no hepatorenal


def test_both_categories_derive_hepatorenal_with_grounded_provenance():
    cli = _cli_or_skip()
    risks, caps = dc.derive_dose_caps(cli, {"hepatic_severe", "renal_moderate"})
    assert risks == {"hepatorenal"}, risks
    # caps is a list; the compound ceftriaxone cap AND the single-factor vancomycin renal cap.
    cef = next((c for c in caps if c["drug"] == "ceftriaxone"), None)
    assert cef and cef["risk"] == "hepatorenal"
    # the grounded FDA byte-quote flows through to the cap.
    assert cef["trust"] == "authoritative" and cef["source"]
    assert "hepatic impairment and significant renal impairment" in cef["source"]
    assert cef["locator"] and "dailymed" in cef["locator"]


def test_graded_severities_all_map_to_their_category():
    cli = _cli_or_skip()
    # Either hepatic grade + either renal grade triggers the compound — the severity→category
    # membership lives in the rulebook (risk_in_category), so the chart can assert any grade.
    for hep in ("hepatic_severe", "hepatic_moderate"):
        for ren in ("renal_severe", "renal_moderate"):
            risks, caps = dc.derive_dose_caps(cli, {hep, ren})
            assert risks == {"hepatorenal"} and "ceftriaxone" in _drugs(caps), (hep, ren)


def test_single_factor_renal_cap_is_grounded():
    cli = _cli_or_skip()
    # CC-2c: a single renal risk caps vancomycin (no conjunction needed), carrying the FDA quote.
    for ren in ("renal_severe", "renal_moderate"):
        risks, caps = dc.derive_dose_caps(cli, {ren})
        assert risks == set(), (ren, risks)  # single factor → no COMPOUND derived
        van = next((c for c in caps if c["drug"] == "vancomycin"), None)
        assert van and van["risk"] == ren and van["trust"] == "authoritative", (ren, caps)
        assert "renal" in van["source"].lower()


def test_single_factor_nephrotoxin_cap_is_grounded():
    cli = _cli_or_skip()
    # CC-2c: a concomitant nephrotoxin caps vancomycin, carrying the PRECAUTIONS byte-quote.
    risks, caps = dc.derive_dose_caps(cli, {"nephrotoxin_interaction"})
    assert risks == set()
    van = next((c for c in caps if c["drug"] == "vancomycin"), None)
    assert van and van["risk"] == "nephrotoxin_interaction" and van["trust"] == "authoritative", caps
    assert "nephrotoxicity" in van["source"].lower()


def test_single_factor_cefepime_renal_cap_is_grounded():
    cli = _cli_or_skip()
    # write-once-use-many: a SECOND drug (cefepime) is capped under renal_severe via the same
    # single-factor substrate — pure data (a grounding record + a row), no engine change.
    risks, caps = dc.derive_dose_caps(cli, {"renal_severe"})
    assert risks == set()
    cef = next((c for c in caps if c["drug"] == "cefepime"), None)
    assert cef and cef["risk"] == "renal_severe" and cef["trust"] == "authoritative", caps
    assert "renal" in cef["source"].lower()
    # vancomycin is ALSO capped under renal_severe — the substrate serves many drugs at once.
    assert any(c["drug"] == "vancomycin" for c in caps), caps


def test_renal_severe_caps_every_renally_cleared_drug():
    cli = _cli_or_skip()
    # Dose-penalty grounding is COMPLETE: every formulary drug with a renal_severe ceiling
    # penalty now has an engine-queryable, FDA-grounded single-factor cap. A renal_severe chart
    # derives the cap for all of them, each at authoritative trust.
    _, caps = dc.derive_dose_caps(cli, {"renal_severe"})
    capped = {c["drug"] for c in caps if c["risk"] == "renal_severe"}
    assert capped == {"vancomycin", "cefepime", "meropenem", "aztreonam", "tmp_smx"}, capped
    assert all(c["trust"] == "authoritative" and c["source"]
               for c in caps if c["risk"] == "renal_severe"), caps


def test_unknown_single_risk_caps_nothing():
    cli = _cli_or_skip()
    # A risk token with no dose_capped_under fact and no category → no compound, no cap.
    assert dc.derive_dose_caps(cli, {"qt_prolongation"}) == (set(), [])


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
