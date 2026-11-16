from __future__ import annotations

import math
from collections.abc import Iterator
from dataclasses import FrozenInstanceError

import pytest

from oscillator import (
    INTEGER_TOLERANCE,
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    TWO_PI,
    SampleBuffer,
    SineOscillator,
    SquareOscillator,
    UniformSampler,
    nyquist_frequency,
    sample_count_for_duration,
    time_at_sample,
)

ABS_TOLERANCE = 1e-9


# =============================================================================
# § 1. Pure Python SplitMix64 Deterministic PRNG for PBT / Fuzzing
# =============================================================================


class SplitMix64:
    """Deterministic 64-bit PRNG (Steele et al., 2014) for reproducible PBT."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
        return z ^ (z >> 31)

    def next_float(self) -> float:
        """Return uniform float in [0.0, 1.0)."""
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, low: float, high: float) -> float:
        """Return uniform float in [low, high)."""
        return low + (high - low) * self.next_float()


class TestSineOscillator:
    def test_default_one_hz_parity_vector(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)

        assert signal.value_at(0.00) == pytest.approx(0.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.25) == pytest.approx(1.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.50) == pytest.approx(0.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.75) == pytest.approx(-1.0, abs=ABS_TOLERANCE)
        assert signal.value_at(1.00) == pytest.approx(0.0, abs=ABS_TOLERANCE)

    def test_amplitude_and_offset_parity_vector(self) -> None:
        signal = SineOscillator(frequency_hz=1.0, amplitude=2.0, offset=3.0)

        assert signal.value_at(0.00) == pytest.approx(3.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.25) == pytest.approx(5.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.75) == pytest.approx(1.0, abs=ABS_TOLERANCE)

    def test_phase_cycles_parity_vector(self) -> None:
        signal = SineOscillator(frequency_hz=1.0, phase_cycles=0.25)

        assert signal.value_at(0.00) == pytest.approx(1.0, abs=ABS_TOLERANCE)
        assert signal.value_at(0.25) == pytest.approx(0.0, abs=ABS_TOLERANCE)

    def test_zero_frequency_is_constant_from_initial_phase(self) -> None:
        signal = SineOscillator(
            frequency_hz=0.0,
            amplitude=2.0,
            phase_cycles=0.25,
            offset=3.0,
        )

        assert signal.value_at(0.0) == pytest.approx(5.0, abs=ABS_TOLERANCE)
        assert signal.value_at(123.456) == pytest.approx(5.0, abs=ABS_TOLERANCE)

    def test_negative_time_is_allowed_for_continuous_evaluation(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)

        assert signal.value_at(-0.25) == pytest.approx(-1.0, abs=ABS_TOLERANCE)

    def test_phase_beyond_two_to_the_29_radians_stays_finite(self) -> None:
        # trig.sin returns NaN beyond 2**29 radians; the oscillator reduces its
        # phase in cycles first, so a 440 Hz tone is still a sine after a week.
        signal = SineOscillator(frequency_hz=440.0)
        week = 7 * 24 * 3600.0
        assert 2 * math.pi * 440.0 * week > 2.0**29

        value = signal.value_at(week + 0.25 / 440.0)

        assert math.isfinite(value)
        assert value == pytest.approx(1.0, abs=1e-9)

    def test_phase_in_cycles_is_exactly_periodic(self) -> None:
        signal = SineOscillator(frequency_hz=1.0, phase_cycles=0.125)

        assert signal.value_at(1.0e9) == signal.value_at(0.0)

    def test_sine_is_immutable(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)

        with pytest.raises(FrozenInstanceError):
            signal.frequency_hz = 2.0  # type: ignore[misc]


class TestSquareOscillator:
    def test_square_parity_vector(self) -> None:
        signal = SquareOscillator(
            frequency_hz=2.0,
            low=0.0,
            high=1.0,
            duty_cycle=0.5,
        )

        assert signal.value_at(0.000) == 1.0
        assert signal.value_at(0.125) == 1.0
        assert signal.value_at(0.250) == 0.0
        assert signal.value_at(0.375) == 0.0
        assert signal.value_at(0.500) == 1.0

    def test_custom_duty_cycle_parity_vector(self) -> None:
        signal = SquareOscillator(
            frequency_hz=1.0,
            low=-2.0,
            high=3.0,
            duty_cycle=0.25,
        )

        assert signal.value_at(0.10) == 3.0
        assert signal.value_at(0.25) == -2.0
        assert signal.value_at(0.90) == -2.0

    def test_square_is_immutable(self) -> None:
        signal = SquareOscillator(frequency_hz=1.0)

        with pytest.raises(FrozenInstanceError):
            signal.duty_cycle = 0.8  # type: ignore[misc]


class TestUniformSampler:
    def test_uniform_sampler_matches_signal_points(self) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        signal = SineOscillator(frequency_hz=1.0)

        buffer = sampler.sample(signal, duration_seconds=1.0)

        assert buffer.samples == pytest.approx(
            (0.0, 1.0, 0.0, -1.0),
            abs=ABS_TOLERANCE,
        )
        assert buffer.sample_rate_hz == 4.0
        assert buffer.start_time_seconds == 0.0

    def test_sampler_sample_count_helper(self) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        signal = SineOscillator(frequency_hz=1.0)

        buffer = sampler.sample_count(signal, sample_count=4)

        assert buffer.samples == pytest.approx(
            (0.0, 1.0, 0.0, -1.0),
            abs=ABS_TOLERANCE,
        )

    def test_sampler_samples_generator_yields_values(self) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        signal = SineOscillator(frequency_hz=1.0)

        stream = sampler.samples(signal, sample_count=2)

        assert next(stream) == pytest.approx(0.0, abs=ABS_TOLERANCE)
        assert next(stream) == pytest.approx(1.0, abs=ABS_TOLERANCE)
        with pytest.raises(StopIteration):
            next(stream)

    def test_sampler_nyquist_frequency(self) -> None:
        sampler = UniformSampler(sample_rate_hz=44_100.0)
        assert sampler.nyquist_frequency() == 22_050.0


class TestSampleBuffer:
    def test_buffer_timing_properties(self) -> None:
        buffer = SampleBuffer(samples=(0.0, 1.0, 2.0, 3.0), sample_rate_hz=4.0)

        assert buffer.sample_count() == 4
        assert buffer.sample_period_seconds() == 0.25
        assert buffer.duration_seconds() == 1.0
        assert buffer.time_at(0) == 0.00
        assert buffer.time_at(1) == 0.25
        assert buffer.time_at(2) == 0.50
        assert buffer.time_at(3) == 0.75

    def test_buffer_with_non_zero_start_time(self) -> None:
        buffer = SampleBuffer(
            samples=(0.0, 1.0),
            sample_rate_hz=2.0,
            start_time_seconds=10.0,
        )

        assert buffer.start_time_seconds == 10.0
        assert buffer.time_at(0) == 10.0
        assert buffer.time_at(1) == 10.5

    def test_empty_buffer_is_valid(self) -> None:
        buffer = SampleBuffer(samples=(), sample_rate_hz=44_100.0)

        assert buffer.sample_count() == 0
        assert buffer.duration_seconds() == 0.0

    def test_sample_buffer_is_immutable(self) -> None:
        buffer = SampleBuffer(samples=(0.0,), sample_rate_hz=1.0)

        with pytest.raises(FrozenInstanceError):
            buffer.samples = (1.0,)  # type: ignore[misc]


class TestHelperFunctions:
    @pytest.mark.parametrize(
        ("duration_seconds", "sample_rate_hz", "expected"),
        [
            (1.0, 44_100.0, 44_100),
            (0.5, 48_000.0, 24_000),
            (0.01, 48_000.0, 480),
            (0.0, 44_100.0, 0),
            (1.0 / 3.0, 10.0, 3),
        ],
    )
    def test_sample_count_for_duration(
        self,
        duration_seconds: float,
        sample_rate_hz: float,
        expected: int,
    ) -> None:
        assert sample_count_for_duration(duration_seconds, sample_rate_hz) == expected

    def test_sample_count_tolerates_near_integer_float_products(self) -> None:
        assert sample_count_for_duration(0.1 + 0.2, 1600.0) == 480

    def test_time_at_sample(self) -> None:
        assert time_at_sample(3, 4.0) == 0.75
        assert time_at_sample(3, 4.0, start_time_seconds=10.0) == 10.75

    def test_nyquist_frequency(self) -> None:
        assert nyquist_frequency(44_100.0) == 22_050.0


class TestValidationAndHardening:
    @pytest.mark.parametrize(
        "kwargs",
        [
            {"frequency_hz": -1.0},
            {"frequency_hz": float("nan")},
            {"frequency_hz": float("inf")},
            {"frequency_hz": True},
            {"frequency_hz": 1.0, "amplitude": -1.0},
            {"frequency_hz": 1.0, "amplitude": True},
            {"frequency_hz": 1.0, "phase_cycles": float("nan")},
            {"frequency_hz": 1.0, "phase_cycles": True},
            {"frequency_hz": 1.0, "offset": float("inf")},
            {"frequency_hz": 1.0, "offset": True},
        ],
    )
    def test_sine_rejects_invalid_parameters(self, kwargs: dict[str, float]) -> None:
        with pytest.raises(ValueError):
            SineOscillator(**kwargs)

    @pytest.mark.parametrize(
        "kwargs",
        [
            {"frequency_hz": -1.0},
            {"frequency_hz": 1.0, "low": float("nan")},
            {"frequency_hz": 1.0, "low": True},
            {"frequency_hz": 1.0, "high": float("inf")},
            {"frequency_hz": 1.0, "high": True},
            {"frequency_hz": 1.0, "duty_cycle": 0.0},
            {"frequency_hz": 1.0, "duty_cycle": 1.0},
            {"frequency_hz": 1.0, "duty_cycle": float("nan")},
            {"frequency_hz": 1.0, "duty_cycle": True},
            {"frequency_hz": 1.0, "phase_cycles": float("inf")},
        ],
    )
    def test_square_rejects_invalid_parameters(self, kwargs: dict[str, float]) -> None:
        with pytest.raises(ValueError):
            SquareOscillator(**kwargs)

    @pytest.mark.parametrize(
        "sample_rate_hz",
        [
            0.0,
            -1.0,
            float("nan"),
            float("inf"),
            -float("inf"),
            False,
            True,
            MAX_SAMPLE_RATE_HZ + 1.0,
            1_000_000.0,
        ],
    )
    def test_sampler_rejects_invalid_sample_rates(
        self,
        sample_rate_hz: float,
    ) -> None:
        with pytest.raises(ValueError):
            UniformSampler(sample_rate_hz=sample_rate_hz)

    @pytest.mark.parametrize(
        ("duration_seconds", "sample_rate_hz"),
        [
            (-1.0, 4.0),
            (float("nan"), 4.0),
            (1.0, 0.0),
            (MAX_DURATION_SECONDS + 0.1, 44_100.0),
            (float("inf"), 44_100.0),
            (True, 44_100.0),
            (1.0, True),
        ],
    )
    def test_sample_count_helper_rejects_invalid_inputs(
        self,
        duration_seconds: float,
        sample_rate_hz: float,
    ) -> None:
        with pytest.raises(ValueError):
            sample_count_for_duration(duration_seconds, sample_rate_hz)

    def test_sample_count_helper_rejects_oversized_allocation(
        self, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        import oscillator.oscillator as osc_mod

        monkeypatch.setattr(osc_mod, "MAX_SAMPLES", 100)
        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            sample_count_for_duration(1.0, 1000.0)

    @pytest.mark.parametrize(
        "index",
        [-1, 1.5, True, False, -100, MAX_SAMPLES, MAX_SAMPLES + 1],
    )
    def test_time_at_sample_rejects_invalid_index(self, index: int) -> None:
        with pytest.raises(ValueError):
            time_at_sample(index, 4.0)

    @pytest.mark.parametrize(
        "sample_count",
        [-1, 1.5, True, False, -5, MAX_SAMPLES + 1],
    )
    def test_sampler_rejects_invalid_explicit_count(
        self,
        sample_count: int,
    ) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        signal = SineOscillator(frequency_hz=1.0)

        with pytest.raises(ValueError):
            tuple(sampler.samples(signal, sample_count=sample_count))

        with pytest.raises(ValueError):
            sampler.sample_count(signal, sample_count=sample_count)

    def test_sample_buffer_rejects_non_finite_samples(self) -> None:
        with pytest.raises(ValueError, match="samples\\[1\\]"):
            SampleBuffer(samples=(0.0, float("nan")), sample_rate_hz=1.0)
        with pytest.raises(ValueError, match="samples\\[0\\]"):
            SampleBuffer(samples=(True,), sample_rate_hz=1.0)  # type: ignore[arg-type]

    def test_sample_buffer_rejects_oversized_allocation(self) -> None:
        class FakeOversizedSeq:
            def __len__(self) -> int:
                return MAX_SAMPLES + 1

            def __iter__(self) -> Iterator[object]:
                return iter(())

        with pytest.raises(ValueError, match="MAX_SAMPLES"):
            SampleBuffer(samples=FakeOversizedSeq(), sample_rate_hz=1000.0)  # type: ignore[arg-type]

    @pytest.mark.parametrize("index", [-1, 2, 1.5, True, False, 100])
    def test_sample_buffer_time_at_rejects_invalid_index(self, index: int) -> None:
        buffer = SampleBuffer(samples=(0.0, 1.0), sample_rate_hz=2.0)

        with pytest.raises(ValueError):
            buffer.time_at(index)

    def test_constants_and_tolerances(self) -> None:
        assert MAX_SAMPLE_RATE_HZ == 192_000.0
        assert MAX_DURATION_SECONDS == 60.0
        assert MAX_SAMPLES == 16_777_216
        assert INTEGER_TOLERANCE == 1e-9
        assert pytest.approx(2.0 * math.pi) == TWO_PI


# =============================================================================
# § 2. Property-Based Testing (PBT) using SplitMix64 PRNG
# =============================================================================


def test_splitmix64_pbt_sine_oscillator_bounded_outputs() -> None:
    """PBT: Sine outputs are strictly within [-amplitude, amplitude] + offset."""
    prng = SplitMix64(seed=0xDEAD_BEEF_CAFE_0001)

    for _ in range(50):
        freq = prng.uniform(0.1, 10_000.0)
        amp = prng.uniform(0.01, 100.0)
        phase = prng.uniform(-10.0, 10.0)
        offset = prng.uniform(-50.0, 50.0)

        osc = SineOscillator(
            frequency_hz=freq,
            amplitude=amp,
            phase_cycles=phase,
            offset=offset,
        )

        for _ in range(20):
            t = prng.uniform(-10.0, 60.0)
            val = osc.value_at(t)
            assert math.isfinite(val)
            assert offset - amp - ABS_TOLERANCE <= val <= offset + amp + ABS_TOLERANCE


def test_splitmix64_pbt_square_oscillator_two_state_property() -> None:
    """PBT: Square oscillator output is always exactly high or low."""
    prng = SplitMix64(seed=0xDEAD_BEEF_CAFE_0002)

    for _ in range(50):
        freq = prng.uniform(0.1, 5_000.0)
        low = prng.uniform(-100.0, 0.0)
        high = prng.uniform(0.01, 100.0)
        duty = prng.uniform(0.01, 0.99)
        phase = prng.uniform(-5.0, 5.0)

        osc = SquareOscillator(
            frequency_hz=freq,
            low=low,
            high=high,
            duty_cycle=duty,
            phase_cycles=phase,
        )

        for _ in range(20):
            t = prng.uniform(-10.0, 60.0)
            val = osc.value_at(t)
            assert val in (high, low)


def test_splitmix64_pbt_uniform_sampler_determinism() -> None:
    """PBT: Sampling the same signal twice yields identical samples bit-for-bit."""
    prng = SplitMix64(seed=0xDEAD_BEEF_CAFE_0003)

    for _ in range(25):
        rate = prng.uniform(100.0, 48_000.0)
        dur = prng.uniform(0.001, 0.1)
        freq = prng.uniform(20.0, 1000.0)

        osc = SineOscillator(frequency_hz=freq)
        sampler = UniformSampler(sample_rate_hz=rate)

        buf1 = sampler.sample(osc, duration_seconds=dur)
        buf2 = sampler.sample(osc, duration_seconds=dur)

        assert buf1.samples == buf2.samples
        assert buf1.sample_count() == buf2.sample_count()
        assert buf1.duration_seconds() == buf2.duration_seconds()
