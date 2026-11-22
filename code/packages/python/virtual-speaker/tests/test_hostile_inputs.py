"""Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
from dataclasses import dataclass
from fractions import Fraction
from numbers import Real
from pathlib import Path

import pytest

from virtual_speaker import MAX_SPEAKER_GAIN, LinearSpeakerSignal


@dataclass(frozen=True)
class _FixedSignal:
    val: object

    def value_at(self, time_seconds: Real) -> float:
        del time_seconds
        return self.val  # type: ignore[return-value]


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
    def test_rejects_bool_non_finite_and_non_real(self, bad_val: object) -> None:
        with pytest.raises(ValueError):
            LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain=bad_val)  # type: ignore[arg-type]

        speaker = LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain=1.0)
        with pytest.raises(ValueError):
            speaker.value_at(bad_val)  # type: ignore[arg-type]

        bad_speaker = LinearSpeakerSignal(_FixedSignal(bad_val), speaker_gain=1.0)
        with pytest.raises(ValueError):
            bad_speaker.value_at(0.0)

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain=huge_real)  # type: ignore[arg-type]

        speaker = LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain=1.0)
        with pytest.raises(ValueError, match="overflows float range"):
            speaker.value_at(huge_real)  # type: ignore[arg-type]

        bad_speaker = LinearSpeakerSignal(_FixedSignal(huge_real), speaker_gain=1.0)
        with pytest.raises(ValueError, match="overflows float range"):
            bad_speaker.value_at(0.0)

    def test_output_multiplication_overflow_rejected(self) -> None:
        speaker = LinearSpeakerSignal(_FixedSignal(1e305), speaker_gain=1e6)
        with pytest.raises(ValueError, match="must be finite"):
            speaker.value_at(0.0)

    def test_invalid_analog_signal_rejected(self) -> None:
        class _NonCallableValueAt:
            value_at = 42

        with pytest.raises(ValueError, match="value_at"):
            LinearSpeakerSignal(_NonCallableValueAt())  # type: ignore[arg-type]


class TestCwe190SafeRepr:
    def test_huge_string_and_small_string_repr_bounded(self) -> None:
        with pytest.raises(ValueError, match="<str len=5000>") as exc:
            LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain="x" * 5000)  # type: ignore[arg-type]
        assert len(str(exc.value)) < 160

        with pytest.raises(ValueError, match="'bad'"):
            LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain="bad")  # type: ignore[arg-type]


class TestCwe400SpeakerGainBounds:
    @pytest.mark.parametrize(
        "bad_gain",
        [MAX_SPEAKER_GAIN + 0.1, -(MAX_SPEAKER_GAIN + 0.1)],
    )
    def test_rejects_excessive_gain(self, bad_gain: float) -> None:
        with pytest.raises(ValueError, match="MAX_SPEAKER_GAIN"):
            LinearSpeakerSignal(_FixedSignal(0.0), speaker_gain=bad_gain)


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "virtual_speaker"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"
