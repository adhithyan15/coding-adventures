"""Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-1333 (VER15)."""

from __future__ import annotations

import ast
import itertools
import sys
from collections.abc import Iterator
from fractions import Fraction
from pathlib import Path

import pytest

from oscillator import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    SampleBuffer,
    SineOscillator,
    SquareOscillator,
    UniformSampler,
    nyquist_frequency,
    sample_count_for_duration,
    time_at_sample,
)


class _LyingSizedIterable:
    """Iterable whose __len__ lies (returns 1) while yielding > MAX_SAMPLES."""

    def __len__(self) -> int:
        return 1

    def __iter__(self) -> Iterator[float]:
        return itertools.repeat(0.0)


class TestCwe20TypeAndFloatValidation:
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
    def test_rejects_bool_non_finite_and_non_real_inputs(
        self, bad_val: object
    ) -> None:
        with pytest.raises(ValueError):
            SineOscillator(frequency_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SineOscillator(frequency_hz=1.0, amplitude=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SineOscillator(frequency_hz=1.0, phase_cycles=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SineOscillator(frequency_hz=1.0, offset=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SineOscillator(frequency_hz=1.0).value_at(bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SquareOscillator(frequency_hz=1.0, low=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SquareOscillator(frequency_hz=1.0, high=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SquareOscillator(frequency_hz=1.0, duty_cycle=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            SquareOscillator(frequency_hz=1.0).value_at(bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            UniformSampler(sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            nyquist_frequency(bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            SineOscillator(frequency_hz=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            SineOscillator(frequency_hz=1.0).value_at(huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            SquareOscillator(frequency_hz=1.0).value_at(huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            UniformSampler(sample_rate_hz=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            SampleBuffer(samples=(huge_real,), sample_rate_hz=4.0)  # type: ignore[arg-type]

    def test_square_oscillator_huge_phase_does_not_raise_overflow_error(self) -> None:
        sq = SquareOscillator(frequency_hz=1e308, low=-2.0, high=2.0)
        val = sq.value_at(1e308)
        assert val == -2.0

    def test_subnormal_sample_rate_rejected_and_non_finite_time_rejected(self) -> None:
        with pytest.raises(ValueError, match="subnormal"):
            UniformSampler(sample_rate_hz=5e-324)
        with pytest.raises(ValueError, match="subnormal"):
            nyquist_frequency(5e-324)
        with pytest.raises(ValueError, match="non-finite"):
            time_at_sample(10, 1e-300, start_time_seconds=sys.float_info.max)

    def test_uniform_sampler_rejects_invalid_signal(self) -> None:
        sampler = UniformSampler(sample_rate_hz=44_100.0)
        with pytest.raises(ValueError, match="value_at"):
            sampler.sample(object(), duration_seconds=0.1)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="value_at"):
            sampler.samples(None, sample_count=4)  # type: ignore[arg-type]

    def test_uniform_sampler_samples_validates_eagerly(self) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        sig = SineOscillator(frequency_hz=1.0)
        with pytest.raises(ValueError):
            sampler.samples(sig, sample_count=-1)


class TestCwe190IntegerBoundsAndSafeRepr:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_fails_in_o1_without_int_max_str_digits(
        self, huge_int: int
    ) -> None:
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            time_at_sample(huge_int, 44_100.0)
        assert len(str(exc1.value)) < 160

        buf = SampleBuffer(samples=(0.0, 1.0), sample_rate_hz=2.0)
        with pytest.raises(ValueError, match="<out-of-range int>") as exc2:
            buf.time_at(huge_int)
        assert len(str(exc2.value)) < 160

        sampler = UniformSampler(sample_rate_hz=4.0)
        sig = SineOscillator(frequency_hz=1.0)
        with pytest.raises(ValueError, match="<out-of-range int>") as exc3:
            sampler.sample_count(sig, huge_int)
        assert len(str(exc3.value)) < 160

    def test_huge_string_and_small_string_repr_bounded(self) -> None:
        huge_str = "x" * 10_000
        with pytest.raises(ValueError, match="<str len=10000>") as exc_info:
            SineOscillator(frequency_hz=huge_str)  # type: ignore[arg-type]
        assert len(str(exc_info.value)) < 160

        with pytest.raises(ValueError, match="'abc'"):
            SineOscillator(frequency_hz="abc")  # type: ignore[arg-type]


class TestCwe400ResourceExhaustionAndGeneratorDos:
    def test_pre_flight_rate_duration_and_count_bounds(self) -> None:
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            UniformSampler(sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
        with pytest.raises(ValueError, match="MAX_DURATION_SECONDS"):
            sample_count_for_duration(MAX_DURATION_SECONDS + 0.01, 44_100.0)
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            UniformSampler(44_100.0).sample_count(
                SineOscillator(440.0), sample_count=MAX_SAMPLES + 1
            )

    def test_sample_buffer_rejects_string_bytes_and_non_iterables(self) -> None:
        for bad in ("0.0", b"\x00\x01", bytearray(b"\x00"), 123, None):
            with pytest.raises(ValueError, match="iterable"):
                SampleBuffer(samples=bad, sample_rate_hz=4.0)  # type: ignore[arg-type]

    def test_sample_buffer_aborts_infinite_generator_without_oom(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import oscillator.oscillator as osc_mod

        monkeypatch.setattr(osc_mod, "MAX_SAMPLES", 16)
        yielded = 0

        def _gen() -> Iterator[float]:
            nonlocal yielded
            while True:
                yielded += 1
                yield 0.0

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            SampleBuffer(samples=_gen(), sample_rate_hz=4.0)  # type: ignore[arg-type]
        assert yielded == 17

    def test_sample_buffer_aborts_lying_sized_iterable(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import oscillator.oscillator as osc_mod

        monkeypatch.setattr(osc_mod, "MAX_SAMPLES", 8)
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            SampleBuffer(
                samples=_LyingSizedIterable(),  # type: ignore[arg-type]
                sample_rate_hz=4.0,
            )

    def test_sample_buffer_rejects_huge_len_overflow_without_overflow_error(
        self,
    ) -> None:
        class _OverflowLenSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[float]:
                return iter(())

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            SampleBuffer(
                samples=_OverflowLenSized(),  # type: ignore[arg-type]
                sample_rate_hz=4.0,
            )


class TestCwe1333ZeroRegexImports:
    def test_no_regex_imports_in_source(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "oscillator"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(alias.name != "re" for alias in node.names)
                elif isinstance(node, ast.ImportFrom):
                    assert node.module != "re"
