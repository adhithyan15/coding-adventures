"""Property-based testing for music_machine using SplitMix64 (MUS01/03/04, VER15)."""

from __future__ import annotations

import math

from music_machine import (
    PortableScore,
    PortableScoreBuilder,
    parse_portable_score,
    parse_score,
    render_portable_score_to_pcm,
    render_score_to_pcm,
)
from music_machine.score import (
    parse_score as parse_simple_score,
)
from music_machine.score import (
    render_score as render_simple_score,
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


NOTES: tuple[str, ...] = ("C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5")
DURATIONS: tuple[str, ...] = ("s", "e", "q")
HITS: tuple[str, ...] = ("kick", "snare", "closed_hihat")


def test_pbt_portable_score_roundtrip_and_render_determinism() -> None:
    rng = SplitMix64(0xCAFE_BABE_2026_0001)
    for idx in range(15):
        builder = PortableScoreBuilder(
            title=f"PBT_{idx}",
            ppq=120,
            sample_rate_hz=4000,
        )
        builder.add_tempo(0, 120 + rng.bounded(120))
        builder.add_meter(0, "4/4")
        builder.add_instrument("lead", kind="sine", gain=0.4)
        builder.add_instrument("drums", kind="drumkit", kit="standard", gain=0.4)
        builder.add_track("mel", instrument_id="lead")
        builder.add_track("perc", instrument_id="drums")

        tick = 0
        for _ in range(4):
            dur = 30 + rng.bounded(60)
            vel = round(0.2 + rng.next_float() * 0.7, 4)
            if rng.bounded(2) == 0:
                note = NOTES[rng.bounded(len(NOTES))]
                builder.add_note("mel", tick, dur, note, velocity=vel)
            else:
                chord = (
                    NOTES[rng.bounded(len(NOTES))],
                    NOTES[rng.bounded(len(NOTES))],
                )
                builder.add_chord("mel", tick, dur, chord, velocity=vel)
            hit = HITS[rng.bounded(len(HITS))]
            builder.add_hit("perc", tick, dur, hit, velocity=vel)
            tick += dur

        score = builder.build()
        # Text & JSON canonical round-trip invariants
        from_text = parse_portable_score(score.to_text())
        from_json = PortableScore.from_json(score.to_json())
        assert from_text == score
        assert from_json == score

        r1 = render_portable_score_to_pcm(score)
        r2 = render_portable_score_to_pcm(from_json)
        assert r1.pcm_buffer.samples == r2.pcm_buffer.samples
        assert all(-32_768 <= s <= 32_767 for s in r1.pcm_buffer.samples)


def test_pbt_text_score_and_simple_score_determinism() -> None:
    rng = SplitMix64(0x1234_5678_ABCD_EF01)
    for _ in range(15):
        tokens = [
            f"{NOTES[rng.bounded(len(NOTES))]}/{DURATIONS[rng.bounded(len(DURATIONS))]}"
            for _ in range(4)
        ]
        tempo = 240 + rng.bounded(240)
        text = f"tempo: {tempo}\nsample_rate: 4000\n{' '.join(tokens)}\n"
        s = parse_score(text)
        r1 = render_score_to_pcm(s)
        r2 = render_score_to_pcm(s)
        assert r1.pcm_buffer.samples == r2.pcm_buffer.samples
        assert all(-32_768 <= x <= 32_767 for x in r1.pcm_buffer.samples)

        simple_lines = [f"tempo: {tempo}"] + [
            f"{NOTES[rng.bounded(len(NOTES))]} {0.25 + rng.next_float() * 0.5:.3f}"
            for _ in range(4)
        ]
        simple_score = parse_simple_score("\n".join(simple_lines))
        samples = render_simple_score(simple_score, sample_rate_hz=200)
        assert all(math.isfinite(v) and -1.0 <= v <= 1.0 for v in samples)
