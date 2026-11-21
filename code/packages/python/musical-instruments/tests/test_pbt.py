"""Property-based testing for musical_instruments using SplitMix64 (MUS05, VER15)."""

from __future__ import annotations

import math

from musical_instruments import (
    ADSREnvelope,
    HarmonicPartial,
    InstrumentProfile,
    VariationProfile,
    all_instruments,
    render_instrument_note,
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


NOTES: tuple[str, ...] = (
    "C3",
    "D3",
    "E3",
    "G3",
    "A3",
    "C4",
    "D4",
    "E4",
    "G4",
    "A4",
    "C5",
)


def test_pbt_preset_instruments_determinism_and_bounds() -> None:
    rng = SplitMix64(0x5A17_BEEF_1234_0001)
    presets = all_instruments()
    for _ in range(25):
        profile = presets[rng.bounded(len(presets))]
        note = NOTES[rng.bounded(len(NOTES))]
        dur = 0.005 + rng.next_float() * 0.03
        amp = 0.05 + rng.next_float() * 0.95
        seed_val = int(rng.next_u64() & 0x7FFF_FFFF)

        var = VariationProfile(
            pitch_cents_jitter=15.0,
            amplitude_jitter=0.15,
            attack_jitter_ms=3.0,
            seed=seed_val,
            harmonic_jitter=0.1,
        )

        r1 = render_instrument_note(
            note,
            dur,
            instrument=profile,
            sample_rate_hz=8_000,
            amplitude=amp,
            variation_profile=var,
        )
        r2 = render_instrument_note(
            note,
            dur,
            instrument=profile,
            sample_rate_hz=8_000,
            amplitude=amp,
            variation_profile=var,
        )

        assert r1.floating_samples.samples == r2.floating_samples.samples
        assert r1.pcm_buffer.samples == r2.pcm_buffer.samples
        assert r1.pcm_buffer.sample_count() == r1.floating_samples.sample_count()
        assert all(
            math.isfinite(s) and -1.0 <= s <= 1.0
            for s in r1.floating_samples.samples
        )
        assert all(-32_768 <= s <= 32_767 for s in r1.pcm_buffer.samples)
        if profile.synthesis_kind == "silence":
            assert all(s == 0.0 for s in r1.floating_samples.samples)
            assert all(s == 0 for s in r1.pcm_buffer.samples)


def test_pbt_custom_additive_nyquist_and_envelope_invariants() -> None:
    rng = SplitMix64(0xDEAD_C0DE_5555_0002)
    for _ in range(15):
        partial_count = 1 + rng.bounded(6)
        partials = tuple(
            HarmonicPartial(
                frequency_multiplier=float(k + 1),
                amplitude=0.1 + rng.next_float() * 0.9,
                phase_cycles=rng.next_float(),
            )
            for k in range(partial_count)
        )
        env = ADSREnvelope(
            attack_seconds=rng.next_float() * 0.01,
            decay_seconds=rng.next_float() * 0.01,
            sustain_level=0.2 + rng.next_float() * 0.8,
            release_seconds=rng.next_float() * 0.01,
        )
        profile = InstrumentProfile(
            id="pbt_custom",
            display_name="PBT Custom",
            synthesis_kind="additive",
            gain=0.1 + rng.next_float() * 0.9,
            harmonic_profile=partials,
            envelope_profile=env,
        )
        res = render_instrument_note(
            "A4",
            0.01 + rng.next_float() * 0.02,
            instrument=profile,
            sample_rate_hz=8_000,
        )
        assert all(
            math.isfinite(s) and -1.0 <= s <= 1.0
            for s in res.floating_samples.samples
        )
        assert res.signal.value_at(env.rendered_duration_seconds(0.05) + 1.0) == 0.0
