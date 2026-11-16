"""Unit tests for inverse STFT and COLA/NOLA reconstruction."""

from __future__ import annotations

import math

from dsp_stft import WindowType, istft, stft


def _assert_close(a: list[float], b: list[float], tol: float = 1e-3) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_istft_cola_hann_half_hop_roundtrip() -> None:
    # Hann window at hop = n_fft // 2 satisfies COLA exactly
    n = 1024
    n_fft = 256
    hop = 128
    sig = [math.sin(0.1 * i) + 0.5 * math.cos(0.3 * i) for i in range(n)]

    spec = stft(sig, n_fft, hop, WindowType.HANN)
    reconstructed = istft(spec, n_fft, hop, WindowType.HANN, output_length=n)

    # In strict-mode framing, end samples beyond last frame might not be fully
    # covered, but the interior region [n_fft // 2 .. n - n_fft // 2] reconstructs.
    # For full length when frames tile, check middle:
    valid_start = n_fft // 2
    valid_end = n - n_fft // 2
    _assert_close(
        sig[valid_start:valid_end],
        reconstructed[valid_start:valid_end],
        tol=1e-3,
    )


def test_istft_default_output_length() -> None:
    n = 512
    n_fft = 128
    hop = 64
    sig = [float(i) * 0.01 for i in range(n)]
    spec = stft(sig, n_fft, hop, WindowType.HAMMING)
    reconstructed = istft(spec, n_fft, hop, WindowType.HAMMING)
    assert len(reconstructed) > 0
