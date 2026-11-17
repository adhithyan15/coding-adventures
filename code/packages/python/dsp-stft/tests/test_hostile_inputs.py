"""Hostile input and security invariant tests for dsp_stft.

Covers:
- CWE-400: MAX_FFT_LEN, MAX_SAMPLES, MAX_FRAMES pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- Frequency range validation: 0 <= f_min < f_max <= sample_rate / 2.
- CWE-674: Zero recursion.
- CWE-1333: Zero regular expression imports.
- Empty signals, empty spectrograms, misaligned spectrogram buffers.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_stft import (
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_SAMPLES,
    EmptySignalError,
    InvalidParamError,
    InvalidSpectrogramError,
    SignalTooShortError,
    WindowType,
    build_window,
    hz_to_mel,
    istft,
    log_spectrogram,
    mel_filterbank,
    mel_spectrogram,
    mel_to_hz,
    mfcc,
    parse_window_type,
    stft,
)


def test_empty_and_short_inputs_rejected() -> None:
    with pytest.raises(EmptySignalError):
        stft([], 64, 32)
    with pytest.raises(InvalidSpectrogramError):
        istft([], 64, 32)
    with pytest.raises(SignalTooShortError):
        stft([1.0] * 30, 64, 32)


def test_invalid_fft_parameters() -> None:
    with pytest.raises(InvalidParamError):
        stft([1.0] * 64, 0, 32)
    with pytest.raises(InvalidParamError):
        stft([1.0] * 64, 64, 0)
    with pytest.raises(InvalidParamError):
        istft([1.0] * 66, 0, 32)
    with pytest.raises(InvalidParamError):
        istft([1.0] * 66, 64, 0)
    with pytest.raises(InvalidParamError):
        istft([1.0] * 66, 64, 32, output_length=0)


def test_misaligned_spectrogram_rejected() -> None:
    # n_fft=64 -> bins=33 -> frame_floats=66
    # Buffer of 100 floats is not divisible by 66
    with pytest.raises(InvalidSpectrogramError):
        istft([0.0] * 100, 64, 32)


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidParamError, match="non-finite"):
            stft([1.0, bad] + [0.0] * 62, 64, 32)
        with pytest.raises(InvalidSpectrogramError, match="non-finite"):
            istft([bad] * 66, 64, 32)
        with pytest.raises(InvalidParamError, match="finite"):
            hz_to_mel(bad)
        with pytest.raises(InvalidParamError, match="finite"):
            mel_to_hz(bad)
        with pytest.raises(InvalidParamError, match="positive and finite"):
            mel_spectrogram([1.0] * 64, 32, 16, 10, bad)
        with pytest.raises(InvalidParamError, match="positive and finite"):
            mel_spectrogram([1.0] * 64, 32, 16, 10, 0.0)
        with pytest.raises(InvalidParamError, match="positive and finite"):
            log_spectrogram([1.0] * 64, 32, 16, eps=bad)
        with pytest.raises(InvalidParamError, match="positive and finite"):
            log_spectrogram([1.0] * 64, 32, 16, eps=0.0)
        with pytest.raises(InvalidParamError, match="positive and finite"):
            log_spectrogram([1.0] * 64, 32, 16, eps=-1.0)


def test_mel_frequency_bounds_rejected() -> None:
    sig = [1.0] * 64
    sr = 16000.0
    with pytest.raises(InvalidParamError, match="frequencies must satisfy"):
        mel_spectrogram(sig, 32, 16, 10, sr, f_min=-1.0, f_max=4000.0)
    with pytest.raises(InvalidParamError, match="frequencies must satisfy"):
        mel_spectrogram(sig, 32, 16, 10, sr, f_min=5000.0, f_max=4000.0)
    with pytest.raises(InvalidParamError, match="frequencies must satisfy"):
        mel_spectrogram(sig, 32, 16, 10, sr, f_min=0.0, f_max=9000.0)  # > 8000 Nyquist
    with pytest.raises(InvalidParamError, match="f_min and f_max must be finite"):
        mel_spectrogram(sig, 32, 16, 10, sr, f_min=float("nan"), f_max=4000.0)

    # Empty filterbank returned for degenerate parameters
    assert mel_filterbank(0, 64, 16000.0) == []
    assert mel_filterbank(10, 0, 16000.0) == []
    assert mel_filterbank(10, 64, -1.0) == []
    assert mel_filterbank(10, 64, 16000.0, f_min=-5.0) == []
    assert mel_filterbank(10, 64, 16000.0, f_min=float("nan")) == []


def test_mfcc_coefficients_bound() -> None:
    sig = [1.0] * 128
    with pytest.raises(InvalidParamError, match="cannot keep more DCT"):
        mfcc(sig, 64, 32, n_mels=10, n_mfcc=11, sample_rate=16000.0)
    with pytest.raises(InvalidParamError, match="must be > 0"):
        mfcc(sig, 64, 32, n_mels=10, n_mfcc=0, sample_rate=16000.0)


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    huge_samples = FakeHugeSequence(MAX_SAMPLES + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        stft(huge_samples, 64, 32)
    with pytest.raises(InvalidParamError, match="exceeds MAX_FFT_LEN"):
        stft([1.0] * 64, MAX_FFT_LEN + 1, 32)
    with pytest.raises(InvalidParamError, match="exceeds MAX_FFT_LEN"):
        build_window(WindowType.HANN, MAX_FFT_LEN + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        istft([0.0] * 66, 64, 32, output_length=MAX_SAMPLES + 1)

    # Frame count bound
    huge_frames_seq = FakeHugeSequence(66 * (MAX_FRAMES + 1))
    with pytest.raises(InvalidParamError, match="exceeds MAX_FRAMES"):
        istft(huge_frames_seq, 64, 32)

    with pytest.raises(InvalidParamError, match="exceeds MAX_FRAMES"):
        stft(FakeHugeSequence((MAX_FRAMES + 2) * 1 + 64), 64, 1)

    with pytest.raises(InvalidParamError, match="n_mels must be > 0"):
        mel_spectrogram([1.0] * 64, 32, 16, 0, 16000.0)

    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        mel_spectrogram([1.0] * 64, 32, 16, MAX_SAMPLES + 1, 16000.0)


def test_window_parsing_invalid() -> None:
    with pytest.raises(InvalidParamError, match="unsupported window type"):
        parse_window_type("unknown_window")
    with pytest.raises(InvalidParamError, match="n_fft must be > 0"):
        build_window(WindowType.HANN, 0)


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_stft"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
