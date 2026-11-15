"""Unit tests for forward STFT."""

from __future__ import annotations

import math

from dsp_stft import WindowType, stft


def test_stft_output_shape() -> None:
    n = 1024
    n_fft = 256
    hop = 128
    signal = [math.sin(0.05 * i) for i in range(n)]
    spec = stft(signal, n_fft, hop, WindowType.HANN)

    num_frames = 1 + (n - n_fft) // hop
    bins = n_fft // 2 + 1
    expected_len = num_frames * bins * 2
    assert len(spec) == expected_len


def test_stft_frequency_localization() -> None:
    # Pure tone at bin 16 out of 256
    n = 512
    n_fft = 256
    hop = 128
    k_target = 16
    signal = [math.sin(2.0 * math.pi * k_target * i / n_fft) for i in range(n)]

    spec = stft(signal, n_fft, hop, WindowType.RECTANGULAR)
    bins = n_fft // 2 + 1
    frame_floats = bins * 2

    # In frame 0, bin 16 should dominate all other bins
    frame_0 = spec[:frame_floats]
    powers = [
        frame_0[2 * k] ** 2 + frame_0[2 * k + 1] ** 2
        for k in range(bins)
    ]
    max_bin = max(range(bins), key=lambda k: powers[k])
    assert max_bin == k_target
