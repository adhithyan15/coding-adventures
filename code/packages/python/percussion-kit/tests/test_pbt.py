"""Property-based testing for percussion_kit using SplitMix64 (MUS07, VER15)."""

from __future__ import annotations

import math

from percussion_kit import (
    PercussionHitEvent,
    all_standard_hit_ids,
    apply_hihat_choke,
    render_percussion_hit,
    render_percussion_sequence,
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


def test_pbt_single_hits_determinism_and_bounds() -> None:
    rng = SplitMix64(0x9E37_0001_CAFE_BEEF)
    hit_ids = all_standard_hit_ids()
    for _ in range(25):
        hit_id = hit_ids[rng.bounded(len(hit_ids))]
        dur = 0.005 + rng.next_float() * 0.04
        amp = 0.05 + rng.next_float() * 0.95

        r1 = render_percussion_hit(
            hit_id, dur, sample_rate_hz=8_000, amplitude=amp
        )
        r2 = render_percussion_hit(
            hit_id, dur, sample_rate_hz=8_000, amplitude=amp
        )

        assert r1.floating_samples.samples == r2.floating_samples.samples
        assert r1.pcm_buffer.samples == r2.pcm_buffer.samples
        assert r1.pcm_buffer.sample_count() == r1.floating_samples.sample_count()
        assert all(
            math.isfinite(s) and -1.0 <= s <= 1.0
            for s in r1.floating_samples.samples
        )
        assert all(-32_768 <= s <= 32_767 for s in r1.pcm_buffer.samples)


def test_pbt_sequence_rendering_and_choke_invariants() -> None:
    rng = SplitMix64(0xBADC_0FFE_1234_5678)
    hit_ids = all_standard_hit_ids()
    for _ in range(15):
        event_count = 1 + rng.bounded(6)
        events = [
            PercussionHitEvent(
                hit_id=hit_ids[rng.bounded(len(hit_ids))],
                start_seconds=rng.next_float() * 0.2,
                duration_seconds=0.005 + rng.next_float() * 0.03,
                velocity=0.1 + rng.next_float() * 0.9,
            )
            for _ in range(event_count)
        ]

        s1 = render_percussion_sequence(events, sample_rate_hz=8_000)
        s2 = render_percussion_sequence(events, sample_rate_hz=8_000)

        assert s1.floating_samples.samples == s2.floating_samples.samples
        assert s1.pcm_buffer.samples == s2.pcm_buffer.samples
        assert all(
            math.isfinite(x) and -1.0 <= x <= 1.0
            for x in s1.floating_samples.samples
        )
        assert all(-32_768 <= x <= 32_767 for x in s1.pcm_buffer.samples)


def test_pbt_apply_hihat_choke_monotonic_attenuation() -> None:
    rng = SplitMix64(0x1357_2468_AAAA_BBBB)
    rate = 8_000.0
    for _ in range(20):
        n = 50 + rng.bounded(200)
        samples = [0.2 + rng.next_float() * 0.8 for _ in range(n)]
        choke_t = rng.next_float() * (n / rate)
        ramp = 0.002 + rng.next_float() * 0.02

        choked = apply_hihat_choke(
            samples,
            hit_start_seconds=0.0,
            choke_start_seconds=choke_t,
            sample_rate_hz=rate,
            choke_ramp_seconds=ramp,
        )
        assert len(choked) == n
        for orig, out in zip(samples, choked, strict=True):
            assert 0.0 <= out <= orig + 1e-12
