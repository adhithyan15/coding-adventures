"""Unit tests for mel scale, mel filterbank, mel spectrogram, and MFCC."""

from __future__ import annotations

import math

from dsp_stft import (
    WindowType,
    hz_to_mel,
    mel_filterbank,
    mel_spectrogram,
    mel_to_hz,
    mfcc,
)


def test_hz_to_mel_roundtrip() -> None:
    for hz in [0.0, 100.0, 440.0, 1000.0, 4000.0, 8000.0]:
        m = hz_to_mel(hz)
        rec_hz = mel_to_hz(m)
        assert math.isclose(hz, rec_hz, rel_tol=1e-5, abs_tol=1e-5)


def test_mel_filterbank_properties() -> None:
    n_mels = 40
    n_fft = 512
    sr = 16000.0
    fb = mel_filterbank(n_mels, n_fft, sr)

    bins = n_fft // 2 + 1
    assert len(fb) == n_mels * bins

    for m in range(n_mels):
        row = fb[m * bins : (m + 1) * bins]
        for v in row:
            assert v >= 0.0
        row_sum = sum(row)
        assert math.isclose(row_sum, 1.0, rel_tol=1e-5) or math.isclose(
            row_sum, 0.0, abs_tol=1e-6
        )


def test_mel_spectrogram_and_mfcc_shapes() -> None:
    n = 1024
    n_fft = 256
    hop = 128
    n_mels = 32
    n_mfcc = 13
    sr = 16000.0

    sig = [math.sin(0.1 * i) for i in range(n)]
    mel = mel_spectrogram(sig, n_fft, hop, n_mels, sr, WindowType.HANN)
    num_frames = 1 + (n - n_fft) // hop
    assert len(mel) == num_frames * n_mels

    mfcc_feats = mfcc(sig, n_fft, hop, n_mels, n_mfcc, sr, WindowType.HANN)
    assert len(mfcc_feats) == num_frames * n_mfcc
    assert all(math.isfinite(v) for v in mfcc_feats)
