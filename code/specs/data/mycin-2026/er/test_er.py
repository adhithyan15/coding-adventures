#!/usr/bin/env python3
"""test_er.py - guard the ER spine: transcribe abstraction + grounded triage.

Runs with NO audio and NO model: the transcribe passthrough is exercised on text,
and triage is checked directly against the grounded rules (red-flag precedence,
diagnosis acuity, undifferentiated default). The full spine is smoke-run only when
a local decomposer backend + the CLI are available. CI runs the no-dependency part.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MYCIN = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(MYCIN / "warm"))
import transcribe as tr  # noqa: E402
import triage as triage_mod  # noqa: E402


def test_transcribe_passthrough_text() -> None:
    assert tr.transcribe("  72M fever neck stiffness  ") == "72M fever neck stiffness"
    assert not tr.is_audio_path("72M fever")  # plain prose is not an audio path
    assert not tr.is_audio_path("nonexistent.wav")  # a non-existent file is not audio


def test_triage_red_flag_overrides_diagnosis() -> None:
    """An active seizure escalates to resuscitation even with a non-emergent leader."""
    t = triage_mod.triage("viral_meningitis", "determinate", ["seizure(present)", "fever(present)"])
    assert t["acuity"] == 1 and t["label"] == "resuscitation", t
    assert t["rule"] == "red_flag:seizure(present)"


def test_triage_bacterial_is_emergent_with_time_target() -> None:
    t = triage_mod.triage("bacterial_meningitis", "determinate",
                          ["csf_glucose(low)", "csf_neutrophilic_pleocytosis(high)"])
    assert t["acuity"] == 2 and t["time_target_min"] == 60, t
    assert any("door-to-antibiotic" in a or "antibiotics within 60" in a
               for a in t["immediate_actions"]), t["immediate_actions"]


def test_triage_viral_is_urgent() -> None:
    t = triage_mod.triage("viral_meningitis", "determinate", ["csf_lymphocytic_pleocytosis(high)"])
    assert t["acuity"] == 3 and t["label"] == "urgent", t


def test_triage_insufficient_evidence_defaults_urgent_not_low() -> None:
    """An undifferentiated presentation errs toward urgent, never under-triaged."""
    t = triage_mod.triage(None, "insufficient_evidence", [])
    assert t["acuity"] == 3 and t["rule"] == "insufficient_evidence", t
    # And a leader with insufficient evidence is NOT given its diagnosis acuity.
    t2 = triage_mod.triage("bacterial_meningitis", "insufficient_evidence", [])
    assert t2["rule"] == "insufficient_evidence", t2


def main() -> int:
    test_transcribe_passthrough_text()
    test_triage_red_flag_overrides_diagnosis()
    test_triage_bacterial_is_emergent_with_time_target()
    test_triage_viral_is_urgent()
    test_triage_insufficient_evidence_defaults_urgent_not_low()
    print("test_er: PASS (transcribe passthrough; red-flag precedence; bacterial=emergent; "
          "viral=urgent; undifferentiated=urgent-not-low; no audio/model required)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
