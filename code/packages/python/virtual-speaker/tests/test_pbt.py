"""Property-based testing for virtual_speaker using SplitMix64 PRNG (MUS01, VER15)."""

from __future__ import annotations

import math
from dataclasses import dataclass
from numbers import Real

from virtual_speaker import MAX_SPEAKER_GAIN, LinearSpeakerSignal

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


@dataclass(frozen=True)
class _RampSignal:
    slope: float
    intercept: float

    def value_at(self, time_seconds: Real) -> float:
        return self.slope * float(time_seconds) + self.intercept


def test_pbt_linear_speaker_scaling_and_inversion() -> None:
    rng = SplitMix64(0x9876_5432_10FE_DCBA)
    for _ in range(100):
        slope = (rng.next_float() - 0.5) * 10.0
        intercept = (rng.next_float() - 0.5) * 5.0
        gain = (rng.next_float() - 0.5) * 200.0
        assert abs(gain) <= MAX_SPEAKER_GAIN

        sig = _RampSignal(slope=slope, intercept=intercept)
        pos_spk = LinearSpeakerSignal(sig, speaker_gain=gain)
        neg_spk = LinearSpeakerSignal(sig, speaker_gain=-gain)
        zero_spk = LinearSpeakerSignal(sig, speaker_gain=0.0)

        for _ in range(5):
            t = (rng.next_float() - 0.5) * 20.0
            v = sig.value_at(t)
            p_pos = pos_spk.value_at(t)
            p_neg = neg_spk.value_at(t)
            assert math.isclose(p_pos, gain * v, rel_tol=1e-12, abs_tol=1e-12)
            assert math.isclose(p_pos, -p_neg, rel_tol=1e-12, abs_tol=1e-12)
            assert zero_spk.value_at(t) == 0.0
