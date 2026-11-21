"""Property-based testing for oscillator using SplitMix64 PRNG (OSC00, VER15)."""

from __future__ import annotations

import math

from oscillator import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    SineOscillator,
    SquareOscillator,
    UniformSampler,
    nyquist_frequency,
    sample_count_for_duration,
    time_at_sample,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:

    class SplitMix64:  # type: ignore[no-redef]
        """Minimal fallback SplitMix64 PRNG for isolated test environments."""

        def __init__(self, seed: int = 0) -> None:
            self._state = seed & 0xFFFF_FFFF_FFFF_FFFF

        def next_u64(self) -> int:
            self._state = (self._state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
            z = self._state
            z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
            z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
            return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

        def next_float(self) -> float:
            return (self.next_u64() >> 11) * (1.0 / (1 << 53))

        def bounded(self, n: int) -> int:
            threshold = (-n) % n
            while True:
                x = self.next_u64()
                if x >= threshold:
                    return x % n


def test_pbt_sine_oscillator_periodicity_and_range() -> None:
    rng = SplitMix64(0xA1B2_C3D4_E5F6_0718)
    for _ in range(100):
        freq = 1.0 + rng.next_float() * 5000.0
        amp = rng.next_float() * 10.0
        phase = (rng.next_float() - 0.5) * 2.0
        offset = (rng.next_float() - 0.5) * 20.0
        osc = SineOscillator(
            frequency_hz=freq, amplitude=amp, phase_cycles=phase, offset=offset
        )

        period = 1.0 / freq
        for _ in range(10):
            t = (rng.next_float() - 0.5) * 10.0
            v0 = osc.value_at(t)
            v1 = osc.value_at(t + period)
            assert math.isfinite(v0) and math.isfinite(v1)
            assert offset - amp - 1e-9 <= v0 <= offset + amp + 1e-9
            assert math.isclose(v0, v1, rel_tol=1e-7, abs_tol=1e-7)


def test_pbt_square_oscillator_two_state_and_duty_cycle() -> None:
    rng = SplitMix64(0x1122_3344_5566_7788)
    for _ in range(100):
        freq = 1.0 + rng.next_float() * 2000.0
        low = -1.0 - rng.next_float() * 5.0
        high = 1.0 + rng.next_float() * 5.0
        duty = 0.05 + rng.next_float() * 0.90
        osc = SquareOscillator(
            frequency_hz=freq, low=low, high=high, duty_cycle=duty
        )

        # Sample strictly inside high and low regions of cycle 0
        t_high = (0.5 * duty) / freq
        t_low = (duty + 0.5 * (1.0 - duty)) / freq
        assert osc.value_at(t_high) == high
        assert osc.value_at(t_low) == low


def test_pbt_uniform_sampler_grid_consistency() -> None:
    rng = SplitMix64(0xCAFE_BABE_1234_ABCD)
    for _ in range(50):
        rate = 100.0 + rng.next_float() * 48_000.0
        dur = 0.001 + rng.next_float() * 0.02
        start = (rng.next_float() - 0.5) * 5.0
        freq = 10.0 + rng.next_float() * 1000.0

        osc = SineOscillator(frequency_hz=freq)
        sampler = UniformSampler(sample_rate_hz=rate)
        buf = sampler.sample(osc, duration_seconds=dur, start_time_seconds=start)

        expected_count = sample_count_for_duration(dur, rate)
        assert buf.sample_count() == expected_count
        assert math.isclose(sampler.nyquist_frequency(), nyquist_frequency(rate))
        assert math.isclose(buf.sample_period_seconds(), 1.0 / rate)

        for idx in range(buf.sample_count()):
            t = time_at_sample(idx, rate, start)
            assert math.isclose(buf.time_at(idx), t, rel_tol=1e-12, abs_tol=1e-12)
            assert buf.samples[idx] == osc.value_at(t)


def test_pbt_hostile_parameter_fuzz_fails_closed() -> None:
    rng = SplitMix64(0xFEED_FACE_9988_7766)
    for _ in range(250):
        raw_rate = (rng.next_float() - 0.2) * (MAX_SAMPLE_RATE_HZ * 1.5)
        raw_dur = (rng.next_float() - 0.2) * (MAX_DURATION_SECONDS * 1.5)
        try:
            count = sample_count_for_duration(raw_dur, raw_rate)
            assert 0 <= count <= int(MAX_DURATION_SECONDS * MAX_SAMPLE_RATE_HZ)
        except ValueError:
            pass
