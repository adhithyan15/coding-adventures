"""Property-based testing for note_audio using SplitMix64 PRNG (MUS01, VER15)."""

from __future__ import annotations

import math

from note_audio import (
    PCM16_MAX,
    PCM16_MIN,
    render_note_to_sound_chain,
    to_wav_bytes,
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
    "C#3",
    "D3",
    "Eb3",
    "E3",
    "F3",
    "F#3",
    "G3",
    "Ab3",
    "A3",
    "Bb3",
    "B3",
    "C4",
    "D4",
    "E4",
    "F4",
    "G4",
    "A4",
    "B4",
    "C5",
)


def test_pbt_note_audio_chain_and_wav_invariants() -> None:
    rng = SplitMix64(0xA440_CAFE_BABE_0001)
    for _ in range(30):
        note = NOTES[rng.bounded(len(NOTES))]
        dur = 0.002 + rng.next_float() * 0.02
        amp = 0.05 + rng.next_float() * 0.9
        gain = 0.25 + rng.next_float() * 3.0
        vref = 0.5 + rng.next_float() * 4.0
        start = (rng.next_float() - 0.5) * 2.0

        chain = render_note_to_sound_chain(
            note,
            duration_seconds=dur,
            sample_rate_hz=44_100.0,
            amplitude=amp,
            start_time_seconds=start,
            full_scale_voltage=vref,
            speaker_gain=gain,
        )

        n = chain.floating_samples.sample_count()
        assert chain.pcm_buffer.sample_count() == n
        assert chain.pcm_buffer.start_time_seconds == start
        assert all(PCM16_MIN <= s <= PCM16_MAX for s in chain.pcm_buffer.samples)

        if n > 0:
            t_mid = start + (0.5 / 44_100.0)
            dac_v = chain.dac_signal.value_at(t_mid)
            spk_p = chain.speaker_signal.value_at(t_mid)
            assert math.isclose(spk_p, dac_v * gain, rel_tol=1e-12, abs_tol=1e-12)
        assert chain.dac_signal.value_at(start - 0.01) == 0.0

        wav_bytes = to_wav_bytes(chain.pcm_buffer)
        assert wav_bytes[:4] == b"RIFF"
        assert len(wav_bytes) == 44 + n * 2
