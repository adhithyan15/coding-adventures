"""Property-based testing for virtual_dac using SplitMix64 PRNG (MUS01, VER15)."""

from __future__ import annotations

import math

from pcm_audio import PCM16_MAX, PCM16_MIN, PCMBuffer, PCMFormat

from virtual_dac import ZeroOrderHoldDACSignal, pcm16_to_voltage

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


def test_pbt_pcm16_to_voltage_monotonicity_and_bounds() -> None:
    rng = SplitMix64(0x1357_2468_ABCD_EF01)
    for _ in range(200):
        s1 = PCM16_MIN + rng.bounded(PCM16_MAX - PCM16_MIN + 1)
        s2 = PCM16_MIN + rng.bounded(PCM16_MAX - PCM16_MIN + 1)
        vref = 0.1 + rng.next_float() * 12.0
        fmt = PCMFormat(full_scale_voltage=vref)

        v1 = pcm16_to_voltage(s1, fmt)
        v2 = pcm16_to_voltage(s2, fmt)
        assert -vref - 1e-12 <= v1 <= vref + 1e-12
        assert -vref - 1e-12 <= v2 <= vref + 1e-12
        if s1 < s2:
            assert v1 < v2
        elif s1 == s2:
            assert v1 == v2


def test_pbt_zero_order_hold_piecewise_constant_and_silence() -> None:
    rng = SplitMix64(0xCAFE_BEEF_9988_7766)
    for _ in range(40):
        n = 1 + rng.bounded(32)
        rate = 100.0 + rng.next_float() * 4000.0
        start = (rng.next_float() - 0.5) * 10.0
        samples = tuple(
            PCM16_MIN + rng.bounded(PCM16_MAX - PCM16_MIN + 1) for _ in range(n)
        )
        pcm = PCMBuffer(
            samples=samples,
            pcm_format=PCMFormat(sample_rate_hz=rate),
            start_time_seconds=start,
        )
        dac = ZeroOrderHoldDACSignal(pcm)

        for idx in range(n):
            t_lo = start + (idx + 0.1) / rate
            t_hi = start + (idx + 0.9) / rate
            expected = pcm16_to_voltage(samples[idx], pcm.pcm_format)
            assert math.isclose(dac.value_at(t_lo), expected, rel_tol=1e-12)
            assert math.isclose(dac.value_at(t_hi), expected, rel_tol=1e-12)

        assert dac.value_at(start - 0.001) == 0.0
        assert dac.value_at(start + pcm.duration_seconds() + 0.001) == 0.0
