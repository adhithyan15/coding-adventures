"""Property-based testing for `wave` using SplitMix64 PRNG (PHY01, VER15)."""

from __future__ import annotations

import math

from trig import PI

from coding_adventures_wave import Wave

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


def test_pbt_amplitude_bounded_and_finite() -> None:
    rng = SplitMix64(0x57A1_E001_CAFE_BEEF)
    for _ in range(200):
        amp = rng.next_float() * 1000.0
        freq = 0.01 + rng.next_float() * 20_000.0
        phase = (rng.next_float() - 0.5) * 4.0 * PI
        w = Wave(amplitude=amp, frequency=freq, phase=phase)

        for _ in range(16):
            t = (rng.next_float() - 0.5) * 100.0
            y = w.evaluate(t)
            assert math.isfinite(y)
            assert abs(y) <= amp + 1e-12


def test_pbt_periodicity_and_half_period_inversion() -> None:
    rng = SplitMix64(0x1234_5678_ABCD_EF01)
    for _ in range(150):
        amp = 0.1 + rng.next_float() * 100.0
        freq = 0.5 + rng.next_float() * 500.0
        phase = (rng.next_float() - 0.5) * 2.0 * PI
        w = Wave(amplitude=amp, frequency=freq, phase=phase)
        period = w.period()
        t = (rng.next_float() - 0.5) * 5.0

        y0 = w.evaluate(t)
        y_half = w.evaluate(t + 0.5 * period)
        y_full = w.evaluate(t + period)

        tol = 1e-9 * max(1.0, amp)
        assert abs(y0 - y_full) <= tol
        assert abs(y0 + y_half) <= tol


def test_pbt_angular_frequency_identity_and_zero_amplitude() -> None:
    rng = SplitMix64(0xDEAD_BEEF_4242_1337)
    for _ in range(100):
        freq = 0.001 + rng.next_float() * 10_000.0
        phase = (rng.next_float() - 0.5) * 100.0
        zero = Wave(amplitude=0.0, frequency=freq, phase=phase)
        t = (rng.next_float() - 0.5) * 1e6
        result = zero.evaluate(t)
        assert result == 0.0
        assert math.copysign(1.0, result) == 1.0

        w = Wave(amplitude=1.0, frequency=freq, phase=phase)
        assert math.isclose(w.angular_frequency() * w.period(), 2.0 * PI, rel_tol=1e-12)
