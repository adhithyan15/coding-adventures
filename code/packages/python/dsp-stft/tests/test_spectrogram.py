"""Unit tests for magnitude and log spectrograms."""

from __future__ import annotations

import math

from dsp_stft import (
    WindowType,
    log_spectrogram,
    power_spectrogram,
    spectrogram,
)


def test_spectrogram_properties() -> None:
    n = 512
    n_fft = 128
    hop = 64
    sig = [math.sin(0.2 * i) for i in range(n)]

    p_spec = power_spectrogram(sig, n_fft, hop, WindowType.HANN)
    spec = spectrogram(sig, n_fft, hop, WindowType.HANN)
    assert p_spec == spec

    # Power spectrogram must be non-negative
    for v in p_spec:
        assert v >= 0.0


def test_log_spectrogram_properties() -> None:
    n = 256
    n_fft = 64
    hop = 32
    sig = [0.0] * n  # Silent signal

    log_spec = log_spectrogram(sig, n_fft, hop, WindowType.HANN, eps=1e-10)
    expected_val = math.log(1e-10)
    for v in log_spec:
        assert math.isclose(v, expected_val, rel_tol=1e-4)
        assert math.isfinite(v)
