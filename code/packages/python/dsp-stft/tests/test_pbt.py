"""Property-based testing for dsp_stft using deterministic SplitMix64 PRNG."""

from __future__ import annotations

import math

from dsp_stft import (
    WindowType,
    hz_to_mel,
    istft,
    mel_filterbank,
    mel_to_hz,
    stft,
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


def _next_f64(prng: SplitMix64) -> float:
    return (prng.next_float() * 20.0) - 10.0


def _assert_close(a: list[float], b: list[float], tol: float = 1e-3) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_pbt_hz_mel_bijective() -> None:
    prng = SplitMix64(0x12345678)
    for _ in range(50):
        hz = prng.next_float() * 20000.0  # [0, 20 kHz]
        mel = hz_to_mel(hz)
        rec_hz = mel_to_hz(mel)
        assert math.isclose(hz, rec_hz, rel_tol=1e-5, abs_tol=1e-5)


def test_pbt_stft_istft_cola_reconstruction() -> None:
    prng = SplitMix64(0x87654321)
    n = 512
    n_fft = 128
    hop = 64
    sig = [_next_f64(prng) for _ in range(n)]

    spec = stft(sig, n_fft, hop, WindowType.HANN)
    rec = istft(spec, n_fft, hop, WindowType.HANN, output_length=n)

    # Interior region is fully covered by COLA Hann
    valid_start = n_fft // 2
    valid_end = n - n_fft // 2
    _assert_close(sig[valid_start:valid_end], rec[valid_start:valid_end], tol=1e-3)


def test_pbt_mel_filterbank_partition_of_unity() -> None:
    prng = SplitMix64(0xFEEDFACE)
    for _ in range(5):
        n_mels = int(prng.next_float() * 50) + 10
        n_fft = 256
        sr = 16000.0 + prng.next_float() * 8000.0
        fb = mel_filterbank(n_mels, n_fft, sr)
        bins = n_fft // 2 + 1
        for m in range(n_mels):
            row_sum = sum(fb[m * bins : (m + 1) * bins])
            assert math.isclose(row_sum, 1.0, rel_tol=1e-4) or math.isclose(
                row_sum, 0.0, abs_tol=1e-5
            )
