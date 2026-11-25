"""Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
from fractions import Fraction
from pathlib import Path

import pytest

from note_audio import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    NoteEvent,
    RenderedNote,
    render_note_to_sound_chain,
)


class TestCwe20TypeAndNumericValidation:
    @pytest.mark.parametrize(
        "bad_val",
        [
            True,
            False,
            float("nan"),
            float("inf"),
            float("-inf"),
            "1.0",
            None,
            complex(1.0, 0.0),
        ],
    )
    def test_rejects_bool_non_finite_and_non_real_params(
        self, bad_val: object
    ) -> None:
        with pytest.raises(ValueError):
            NoteEvent("A4", duration_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            NoteEvent("A4", duration_seconds=0.01, amplitude=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            NoteEvent("A4", duration_seconds=0.01, start_time_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, sample_rate_hz=bad_val  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, phase_cycles=bad_val  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, full_scale_voltage=bad_val  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, speaker_gain=bad_val  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, max_sample_count=bad_val  # type: ignore[arg-type]
            )

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            NoteEvent("A4", duration_seconds=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, sample_rate_hz=huge_real  # type: ignore[arg-type]
            )
        with pytest.raises(ValueError, match="overflows float range"):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, speaker_gain=huge_real  # type: ignore[arg-type]
            )

    def test_subnormal_sample_rate_and_nyquist_rejected(self) -> None:
        with pytest.raises(ValueError, match="subnormal"):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, sample_rate_hz=5e-324
            )
        with pytest.raises(ValueError, match="below Nyquist"):
            render_note_to_sound_chain(
                "B100", duration_seconds=0.01, sample_rate_hz=44_100.0
            )


class TestCwe190HugeIntegersAndSafeRepr:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_fails_in_o1_via_safe_repr(self, huge_int: int) -> None:
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, max_sample_count=huge_int
            )
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            render_note_to_sound_chain(huge_int, duration_seconds=0.01)  # type: ignore[arg-type]
        assert len(str(exc2.value)) < 160

    def test_safe_repr_formats_long_strings_and_short_strings(self) -> None:
        with pytest.raises(ValueError, match="<str len=5000>") as exc:
            NoteEvent("A4", duration_seconds="x" * 5000)  # type: ignore[arg-type]
        assert len(str(exc.value)) < 160

        with pytest.raises(ValueError, match="'bad'"):
            NoteEvent("A4", duration_seconds="bad")  # type: ignore[arg-type]


class TestCwe400ResourceExhaustionBounds:
    def test_pre_flight_bounds_enforced(self) -> None:
        with pytest.raises(ValueError, match="MAX_DURATION_SECONDS"):
            NoteEvent("A4", duration_seconds=MAX_DURATION_SECONDS + 0.01)
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0
            )
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            render_note_to_sound_chain(
                "A4", duration_seconds=0.01, max_sample_count=MAX_SAMPLES + 1
            )
        with pytest.raises(ValueError, match="above max_sample_count"):
            render_note_to_sound_chain(
                "A4", duration_seconds=1.0, sample_rate_hz=44_100.0, max_sample_count=10
            )

    def test_oversized_note_string_rejected(self) -> None:
        huge_note = "A" + "0" * 100_000 + "4"
        with pytest.raises(ValueError, match="exceeds maximum"):
            NoteEvent(huge_note, duration_seconds=0.01)

    @pytest.mark.parametrize(
        ("field", "match"),
        [
            ("note_event", "note_event must be a NoteEvent"),
            ("oscillator", "oscillator must be a SineOscillator"),
            ("floating_samples", "floating_samples must be a SampleBuffer"),
            ("pcm_buffer", "pcm_buffer must be a PCMBuffer"),
            ("dac_signal", "dac_signal must be a ZeroOrderHoldDACSignal"),
            ("speaker_signal", "speaker_signal must be a LinearSpeakerSignal"),
        ],
    )
    def test_rendered_note_rejects_invalid_stage_fields(
        self, field: str, match: str
    ) -> None:
        valid = render_note_to_sound_chain("A4", duration_seconds=0.001)
        kwargs: dict[str, object] = {
            "note_event": valid.note_event,
            "frequency_hz": valid.frequency_hz,
            "oscillator": valid.oscillator,
            "floating_samples": valid.floating_samples,
            "pcm_buffer": valid.pcm_buffer,
            "dac_signal": valid.dac_signal,
            "speaker_signal": valid.speaker_signal,
        }
        kwargs[field] = "invalid"
        with pytest.raises(ValueError, match=match):
            RenderedNote(**kwargs)  # type: ignore[arg-type]


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "note_audio"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"
