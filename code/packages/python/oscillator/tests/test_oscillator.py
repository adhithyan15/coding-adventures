from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from oscillator import (
    SampleBuffer,
    SineOscillator,
    SquareOscillator,
    UniformSampler,
    nyquist_frequency,
    sample_count_for_duration,
    time_at_sample,
)

ABS_TOLERANCE = 1e-9


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

    def test_negative_time_uses_portable_fractional_part(self) -> None:
        signal = SquareOscillator(
            frequency_hz=1.0,
            low=0.0,
            high=1.0,
            duty_cycle=0.5,
        )

        assert signal.value_at(-0.25) == 0.0
        assert signal.value_at(-0.75) == 1.0

    def test_phase_cycles_shifts_square_wave(self) -> None:
        signal = SquareOscillator(
            frequency_hz=1.0,
            low=-1.0,
            high=1.0,
            duty_cycle=0.5,
            phase_cycles=0.5,
        )

        assert signal.value_at(0.0) == -1.0
        assert signal.value_at(0.5) == 1.0

    def test_zero_frequency_uses_initial_phase(self) -> None:
        high_signal = SquareOscillator(
            frequency_hz=0.0,
            low=0.0,
            high=1.0,
            duty_cycle=0.5,
            phase_cycles=0.25,
        )
        low_signal = SquareOscillator(
            frequency_hz=0.0,
            low=0.0,
            high=1.0,
            duty_cycle=0.5,
            phase_cycles=0.75,
        )

        assert high_signal.value_at(999.0) == 1.0
        assert low_signal.value_at(999.0) == 0.0


class TestUniformSampler:
    def test_uniform_sampler_parity_vector(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)
        sampler = UniformSampler(sample_rate_hz=4.0)

        buffer = sampler.sample(signal, duration_seconds=1.0)

        assert [buffer.time_at(index) for index in range(buffer.sample_count())] == [
            0.0,
            0.25,
            0.5,
            0.75,
        ]
        assert buffer.samples == pytest.approx(
            (0.0, 1.0, 0.0, -1.0),
            abs=ABS_TOLERANCE,
        )
        assert buffer.sample_count() == 4
        assert buffer.sample_period_seconds() == 0.25
        assert buffer.duration_seconds() == 1.0
        assert sampler.nyquist_frequency() == 2.0

    def test_start_time_offsets_sample_grid(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)
        sampler = UniformSampler(sample_rate_hz=4.0)

        buffer = sampler.sample(
            signal,
            duration_seconds=0.5,
            start_time_seconds=0.25,
        )

        assert buffer.samples == pytest.approx((1.0, 0.0), abs=ABS_TOLERANCE)
        assert buffer.time_at(0) == 0.25
        assert buffer.time_at(1) == 0.5

    def test_explicit_sample_count_matches_streaming_values(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)
        sampler = UniformSampler(sample_rate_hz=4.0)

        buffer = sampler.sample_count(signal, sample_count=3)

        assert buffer.samples == tuple(sampler.samples(signal, sample_count=3))
        assert buffer.samples == pytest.approx((0.0, 1.0, 0.0), abs=ABS_TOLERANCE)

    def test_zero_duration_produces_empty_buffer(self) -> None:
        signal = SineOscillator(frequency_hz=1.0)
        sampler = UniformSampler(sample_rate_hz=44_100.0)

        buffer = sampler.sample(signal, duration_seconds=0.0)

        assert buffer.samples == ()
        assert buffer.sample_count() == 0
        assert buffer.duration_seconds() == 0.0


class TestSampleBuffer:
    def test_sample_buffer_derives_metadata(self) -> None:
        buffer = SampleBuffer(
            samples=(0.0, 1.0, 0.0),
            sample_rate_hz=2.0,
            start_time_seconds=10.0,
        )

        assert buffer.sample_count() == 3
        assert buffer.sample_period_seconds() == 0.5
        assert buffer.duration_seconds() == 1.5
        assert buffer.time_at(2) == 11.0

    def test_sample_buffer_converts_sample_iterables_to_tuple(self) -> None:
        buffer = SampleBuffer(samples=[0, 1, -1], sample_rate_hz=1.0)

        assert buffer.samples == (0.0, 1.0, -1.0)


class TestHelpers:
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


class TestValidation:
    @pytest.mark.parametrize(
        "kwargs",
        [
            {"frequency_hz": -1.0},
            {"frequency_hz": float("nan")},
            {"frequency_hz": float("inf")},
            {"frequency_hz": True},
            {"frequency_hz": 1.0, "amplitude": -1.0},
            {"frequency_hz": 1.0, "phase_cycles": float("nan")},
            {"frequency_hz": 1.0, "offset": float("inf")},
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
            {"frequency_hz": 1.0, "high": float("inf")},
            {"frequency_hz": 1.0, "duty_cycle": 0.0},
            {"frequency_hz": 1.0, "duty_cycle": 1.0},
            {"frequency_hz": 1.0, "duty_cycle": float("nan")},
            {"frequency_hz": 1.0, "phase_cycles": float("inf")},
        ],
    )
    def test_square_rejects_invalid_parameters(self, kwargs: dict[str, float]) -> None:
        with pytest.raises(ValueError):
            SquareOscillator(**kwargs)

    @pytest.mark.parametrize(
        "sample_rate_hz",
        [0.0, -1.0, float("nan"), float("inf"), False],
    )
    def test_sampler_rejects_invalid_sample_rates(
        self,
        sample_rate_hz: float,
    ) -> None:
        with pytest.raises(ValueError):
            UniformSampler(sample_rate_hz=sample_rate_hz)

    @pytest.mark.parametrize(
        ("duration_seconds", "sample_rate_hz"),
        [(-1.0, 4.0), (float("nan"), 4.0), (1.0, 0.0)],
    )
    def test_sample_count_helper_rejects_invalid_inputs(
        self,
        duration_seconds: float,
        sample_rate_hz: float,
    ) -> None:
        with pytest.raises(ValueError):
            sample_count_for_duration(duration_seconds, sample_rate_hz)

    @pytest.mark.parametrize("index", [-1, 1.5, True])
    def test_time_at_sample_rejects_invalid_index(self, index: int) -> None:
        with pytest.raises(ValueError):
            time_at_sample(index, 4.0)

    @pytest.mark.parametrize("sample_count", [-1, 1.5, True])
    def test_sampler_rejects_invalid_explicit_count(
        self,
        sample_count: int,
    ) -> None:
        sampler = UniformSampler(sample_rate_hz=4.0)
        signal = SineOscillator(frequency_hz=1.0)

        with pytest.raises(ValueError):
            tuple(sampler.samples(signal, sample_count=sample_count))

    def test_sample_buffer_rejects_non_finite_samples(self) -> None:
        with pytest.raises(ValueError, match="samples\\[1\\]"):
            SampleBuffer(samples=(0.0, float("nan")), sample_rate_hz=1.0)

    @pytest.mark.parametrize("index", [-1, 2, 1.5, True])
    def test_sample_buffer_time_at_rejects_invalid_index(self, index: int) -> None:
        buffer = SampleBuffer(samples=(0.0, 1.0), sample_rate_hz=2.0)

        with pytest.raises(ValueError):
            buffer.time_at(index)
