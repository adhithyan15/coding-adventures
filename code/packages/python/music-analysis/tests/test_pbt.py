"""Property-based testing for music analysis using SplitMix64 PRNG (MUS08, VER15)."""

from __future__ import annotations

import math

from note_frequency import note_to_frequency

from music_analysis import (
    frequency_to_note,
    rms_energy,
    spectral_centroid,
    spectral_rolloff,
    yin_pitch,
    zero_crossing_rate,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:
    class SplitMix64:  # type: ignore[no-redef]
        def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
            self.state: int = seed & 0xFFFFFFFFFFFFFFFF

        def next_u64(self) -> int:
            self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
            z = self.state
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
            return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

        def next_float(self) -> float:
            return (self.next_u64() >> 11) * (1.0 / (1 << 53))


def test_pbt_frequency_to_note_invertibility() -> None:
    prng = SplitMix64(0xFEEDFACE11223344)
    for _ in range(50):
        # Sample random frequency in [50 Hz, 4000 Hz]
        hz = 50.0 + prng.next_float() * 3950.0
        match = frequency_to_note(hz)

        # Parse the note back via note_frequency
        nominal_hz = note_to_frequency(match.note)

        # Reconstruct frequency including cents: f = nominal * 2^(cents / 1200)
        reconstructed_hz = nominal_hz * (2.0 ** (match.cents / 1200.0))
        assert math.isclose(hz, reconstructed_hz, rel_tol=1e-3, abs_tol=1e-2)


def test_pbt_feature_value_invariants() -> None:
    prng = SplitMix64(0xDEADBEEFCAFEBABE)
    sr = 44100.0
    for _ in range(20):
        length = int(prng.next_float() * 200) + 20
        samples = [(prng.next_float() * 2.0) - 1.0 for _ in range(length)]

        rms = rms_energy(samples)
        assert isinstance(rms, float)
        assert rms >= 0.0

        zcr = zero_crossing_rate(samples)
        assert isinstance(zcr, float)
        assert 0.0 <= zcr <= 1.0

        # Positive spectrum
        spec = [prng.next_float() * 10.0 for _ in range(length)]
        centroid = spectral_centroid(spec, sr)
        assert 0.0 <= centroid <= (sr / 2.0)


def test_pbt_rolloff_monotonicity() -> None:
    prng = SplitMix64(0x55AA55AA33CC33CC)
    sr = 44100.0
    for _ in range(20):
        k_bins = int(prng.next_float() * 100) + 10
        spec = [prng.next_float() * 5.0 for _ in range(k_bins)]

        r50 = spectral_rolloff(spec, sr, rolloff_percent=0.50)
        r85 = spectral_rolloff(spec, sr, rolloff_percent=0.85)
        r95 = spectral_rolloff(spec, sr, rolloff_percent=0.95)

        assert r50 <= r85 + 1e-6
        assert r85 <= r95 + 1e-6


def test_pbt_determinism() -> None:
    prng = SplitMix64(0x1357924680BDFE)
    sr = 44100.0
    sig = [(prng.next_float() * 2.0) - 1.0 for _ in range(1024)]

    p1 = yin_pitch(sig, sr)
    p2 = yin_pitch(sig, sr)
    assert p1 == p2

    r1 = rms_energy(sig)
    r2 = rms_energy(sig)
    assert r1 == r2
