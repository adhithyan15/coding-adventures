"""Hostile input testing for CWE-20, CWE-190, CWE-400, CWE-674, and CWE-1333."""

from __future__ import annotations

import ast
from collections.abc import Iterator
from fractions import Fraction
from pathlib import Path
from unittest.mock import patch

import pytest

from musical_instruments import (
    MAX_NOTE_DURATION_SECONDS,
    MAX_PARTIALS,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    ADSREnvelope,
    GMProgram,
    HarmonicPartial,
    InstrumentNoteRender,
    InstrumentProfile,
    InstrumentSignal,
    SplitMix64,
    VariationProfile,
    get_gm_program,
    get_instrument,
    render_instrument_note,
)
from musical_instruments.musical_instruments import _safe_repr


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
            HarmonicPartial(bad_val, 1.0)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            HarmonicPartial(1.0, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            HarmonicPartial(1.0, 1.0, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            ADSREnvelope(bad_val, 0.01, 0.5, 0.01)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            ADSREnvelope(0.01, bad_val, 0.5, 0.01)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            ADSREnvelope(0.01, 0.01, bad_val, 0.01)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            ADSREnvelope(0.01, 0.01, 0.5, bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            VariationProfile(pitch_cents_jitter=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            VariationProfile(amplitude_jitter=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            VariationProfile(attack_jitter_ms=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            VariationProfile(harmonic_jitter=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", 0.01, sample_rate_hz=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", 0.01, amplitude=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", 0.01, start_time_seconds=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", 0.01, max_sample_count=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            render_instrument_note("A4", 0.01, max_partial_count=bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            HarmonicPartial(huge_real, 1.0)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            ADSREnvelope(huge_real, 0.01, 0.5, 0.01)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            render_instrument_note("A4", huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            render_instrument_note("A4", 0.01, sample_rate_hz=huge_real)  # type: ignore[arg-type]

    def test_subnormal_sample_rate_and_nyquist_rejected(self) -> None:
        with pytest.raises(ValueError, match="subnormal"):
            render_instrument_note("A4", 0.01, sample_rate_hz=5e-324)
        with pytest.raises(ValueError, match="below Nyquist"):
            render_instrument_note("B100", 0.01, sample_rate_hz=44_100.0)

    def test_invalid_note_names_and_string_fields_rejected(self) -> None:
        with pytest.raises(ValueError):
            render_instrument_note("H4", 0.01)
        with pytest.raises(ValueError):
            render_instrument_note("A" + "0" * 10_000 + "4", 0.01)

        env = ADSREnvelope(0.0, 0.0, 1.0, 0.0)
        partial = HarmonicPartial(1.0, 1.0)
        with pytest.raises(ValueError, match="id must be a non-empty string"):
            InstrumentProfile(123, "Name", "sine", 1.0, (partial,), env)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="display_name must be a non-empty string"):
            InstrumentProfile("id", 123, "sine", 1.0, (partial,), env)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="family must be a string"):
            InstrumentProfile("id", "Name", "sine", 1.0, (partial,), env, family=123)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="name must be a string"):
            GMProgram(1, 123, "piano", "piano_naive")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="family must be a string"):
            GMProgram(1, "Piano", 123, "piano_naive")  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="instrument_id must be a string"):
            GMProgram(1, "Piano", "piano", 123)  # type: ignore[arg-type]

    def test_pitch_jitter_overflow_caught_as_value_error(self) -> None:
        profile = get_instrument("sine")
        var = VariationProfile(pitch_cents_jitter=1e12, seed=42)
        # Seed 42 gives a non-zero pitch_draw so 2^(delta_cents/1200) overflows.
        with pytest.raises(ValueError, match="pitch_cents_jitter overflows"):
            InstrumentSignal(
                fundamental_hz=440.0,
                instrument=profile,
                note_duration_seconds=0.05,
                variation_profile=var,
            )

        # Very large positive jitter without OverflowError that pushes eff_fund to inf
        var2 = VariationProfile(pitch_cents_jitter=1_300_000.0, seed=1)
        with pytest.raises(ValueError, match="effective fundamental frequency"):
            InstrumentSignal(
                fundamental_hz=1e307,
                instrument=profile,
                note_duration_seconds=0.05,
                variation_profile=var2,
            )

    def test_amplitude_sum_and_jitter_overflow_rejected(self) -> None:
        env = ADSREnvelope(0.0, 0.0, 1.0, 0.0)
        with pytest.raises(ValueError, match="partial amplitude sum overflows"):
            InstrumentProfile(
                "ovf",
                "Ovf",
                "additive",
                1.0,
                (HarmonicPartial(1.0, 1.5e308), HarmonicPartial(2.0, 1.5e308)),
                env,
            )

        prof = InstrumentProfile(
            "big", "Big", "additive", 1.0, (HarmonicPartial(1.0, 1.0e308),), env
        )
        with (
            patch.object(SplitMix64, "next_float_signed", return_value=2.0),
            pytest.raises(ValueError, match="amplitude_jitter overflows"),
        ):
            InstrumentSignal(
                100.0,
                prof,
                0.05,
                amplitude=0.0,
                variation_profile=VariationProfile(amplitude_jitter=1.5e308, seed=1),
            )
        with (
            patch.object(SplitMix64, "next_float_signed", return_value=2.0),
            pytest.raises(ValueError, match="harmonic_jitter overflows"),
        ):
            InstrumentSignal(
                100.0,
                prof,
                0.05,
                variation_profile=VariationProfile(harmonic_jitter=1.5e308, seed=1),
            )
        # Real SplitMix64 harmonic_jitter scaling pushing p_amp -> inf
        with pytest.raises(ValueError, match="amplitude must be finite"):
            InstrumentSignal(
                100.0,
                prof,
                0.05,
                variation_profile=VariationProfile(harmonic_jitter=1.0, seed=0),
            )
        # Effective harmonic amplitude sum overflow when partial sum <= 1e308 * 2 -> inf
        prof2 = InstrumentProfile(
            "big2",
            "Big2",
            "additive",
            1.0,
            (HarmonicPartial(1.0, 8e307), HarmonicPartial(2.0, 8e307)),
            env,
        )
        with (
            patch.object(SplitMix64, "next_float_signed", return_value=0.8),
            pytest.raises(
                ValueError, match="effective harmonic amplitude sum overflows"
            ),
        ):
            InstrumentSignal(
                100.0,
                prof2,
                0.05,
                variation_profile=VariationProfile(harmonic_jitter=0.5, seed=0),
            )

    @pytest.mark.parametrize(
        ("field", "match"),
        [
            ("note", "note must be a Note"),
            ("fundamental_hz", "fundamental_hz"),
            ("instrument", "instrument must be an InstrumentProfile"),
            ("signal", "signal must be an InstrumentSignal"),
            ("floating_samples", "floating_samples must be a SampleBuffer"),
            ("pcm_buffer", "pcm_buffer must be a PCMBuffer"),
        ],
    )
    def test_instrument_note_render_rejects_invalid_stage_fields(
        self, field: str, match: str
    ) -> None:
        valid = render_instrument_note("A4", 0.005, sample_rate_hz=8_000)
        kwargs: dict[str, object] = {
            "note": valid.note,
            "fundamental_hz": valid.fundamental_hz,
            "instrument": valid.instrument,
            "signal": valid.signal,
            "floating_samples": valid.floating_samples,
            "pcm_buffer": valid.pcm_buffer,
        }
        kwargs[field] = "invalid"
        with pytest.raises(ValueError, match=match):
            InstrumentNoteRender(**kwargs)  # type: ignore[arg-type]


class TestCwe190HugeIntegersAndSafeRepr:
    @pytest.mark.parametrize(
        "huge_int",
        [10**5000, -(10**5000)],
        ids=["pos-10e5000", "neg-10e5000"],
    )
    def test_huge_int_fails_in_o1_via_safe_repr(self, huge_int: int) -> None:
        assert _safe_repr(huge_int) == "<out-of-range int>"
        with pytest.raises(ValueError, match="<out-of-range int>") as exc1:
            render_instrument_note("A4", 0.01, max_sample_count=huge_int)
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError) as exc2:
            get_gm_program(huge_int)
        assert len(str(exc2.value)) < 160

        with pytest.raises(ValueError, match="<out-of-range int>") as exc3:
            render_instrument_note(huge_int, 0.01)  # type: ignore[arg-type]
        assert len(str(exc3.value)) < 160

        # Huge int seed works in O(1) without raising
        rng = SplitMix64(huge_int)
        assert 0 <= rng.next_u64() < (1 << 64)

    def test_safe_repr_formats_long_strings_and_bad_seed(self) -> None:
        long_str = "x" * 5000
        with pytest.raises(ValueError, match="<str len=5000>") as exc1:
            get_instrument(long_str)
        assert len(str(exc1.value)) < 160

        with pytest.raises(ValueError, match="<str len=5000>") as exc2:
            SplitMix64(long_str)  # type: ignore[arg-type]
        assert len(str(exc2.value)) < 160

        with pytest.raises(ValueError, match="<str len=5000>") as exc3:
            render_instrument_note("A4", long_str)  # type: ignore[arg-type]
        assert len(str(exc3.value)) < 160


class TestCwe400ResourceExhaustionBounds:
    def test_oversized_and_generator_harmonic_profiles_rejected_before_alloc(
        self,
    ) -> None:
        env = ADSREnvelope(0.0, 0.0, 1.0, 0.0)
        partial = HarmonicPartial(1.0, 1.0)

        class LyingSized:
            def __len__(self) -> int:
                return 10**100

            def __iter__(self) -> Iterator[HarmonicPartial]:
                yield partial

        with pytest.raises(ValueError, match="MAX_PARTIALS"):
            InstrumentProfile("p", "P", "additive", 1.0, LyingSized(), env)  # type: ignore[arg-type]

        class HugeLenList(list):
            def __len__(self) -> int:
                return MAX_PARTIALS + 1

        with pytest.raises(ValueError, match="MAX_PARTIALS"):
            InstrumentProfile("p", "P", "additive", 1.0, HugeLenList([partial]), env)

        consumed = 0

        def infinite_partials() -> Iterator[HarmonicPartial]:
            nonlocal consumed
            while True:
                consumed += 1
                yield partial

        with pytest.raises(ValueError, match="MAX_PARTIALS"):
            InstrumentProfile(
                "p", "P", "additive", 1.0, infinite_partials(), env  # type: ignore[arg-type]
            )
        assert consumed == MAX_PARTIALS + 1

        with pytest.raises(ValueError, match="iterable"):
            InstrumentProfile("p", "P", "additive", 1.0, "bad", env)  # type: ignore[arg-type]

    def test_pre_flight_duration_rate_and_sample_caps(self) -> None:
        with pytest.raises(ValueError, match="MAX_NOTE_DURATION_SECONDS"):
            render_instrument_note("A4", MAX_NOTE_DURATION_SECONDS + 0.1)
        with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
            render_instrument_note("A4", 0.01, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
        with pytest.raises(ValueError, match="MAX_SAMPLE_COUNT"):
            render_instrument_note("A4", 0.01, max_sample_count=MAX_SAMPLE_COUNT + 1)
        with pytest.raises(ValueError, match="MAX_PARTIALS"):
            render_instrument_note("A4", 0.01, max_partial_count=MAX_PARTIALS + 1)
        with pytest.raises(ValueError, match="max_partial_count"):
            render_instrument_note(
                "A4", 0.01, instrument="piano_naive", max_partial_count=1
            )
        with pytest.raises(ValueError, match="above max_sample_count"):
            render_instrument_note("A4", 1.0, sample_rate_hz=8000, max_sample_count=10)


class TestCwe1333AndCwe674StaticAstAudit:
    def test_zero_regex_eval_exec_and_zero_recursion(self) -> None:
        pkg_src = Path(__file__).resolve().parents[1] / "src" / "musical_instruments"
        for py_file in pkg_src.rglob("*.py"):
            tree = ast.parse(py_file.read_text(encoding="utf-8"))
            func_defs: dict[str, ast.FunctionDef] = {}
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    assert all(
                        alias.name not in ("re", "regex") for alias in node.names
                    )
                elif isinstance(node, ast.ImportFrom):
                    assert node.module not in ("re", "regex")
                elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                    assert node.func.id not in ("eval", "exec")
                elif isinstance(node, ast.FunctionDef):
                    func_defs[node.name] = node
            for name, fn in func_defs.items():
                for child in ast.walk(fn):
                    if isinstance(child, ast.Call) and isinstance(
                        child.func, ast.Name
                    ):
                        assert child.func.id != name
