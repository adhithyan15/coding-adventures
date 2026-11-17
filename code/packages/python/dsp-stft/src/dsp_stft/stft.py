"""Short-Time Fourier Transform (STFT) forward transform."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_fft.rfft import rfft_scalar

from dsp_stft.errors import (
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_SAMPLES,
    EmptySignalError,
    FftError,
    InvalidParamError,
    SignalTooShortError,
    validate_floats,
)
from dsp_stft.windows import WindowType, build_window


def stft(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    window: WindowType | str = WindowType.HANN,
) -> list[float]:
    """Compute the short-time Fourier transform of signal in strict framing mode.

    Returns a flattened row-major [num_frames, n_fft // 2 + 1, 2] list of floats
    holding interleaved complex spectra [re0, im0, re1, im1, ...].
    """
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if n_fft <= 0:
        raise InvalidParamError(f"n_fft must be > 0; got {n_fft}")
    if hop_length <= 0:
        raise InvalidParamError(f"hop_length must be > 0; got {hop_length}")

    n = len(signal)
    if n > MAX_SAMPLES:
        raise InvalidParamError(
            f"signal length {n} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    if n_fft > MAX_FFT_LEN:
        raise InvalidParamError(
            f"n_fft {n_fft} exceeds MAX_FFT_LEN {MAX_FFT_LEN}"
        )
    if n < n_fft:
        raise SignalTooShortError(
            f"signal length {n} < n_fft {n_fft} (strict-mode framing)"
        )

    num_frames = 1 + (n - n_fft) // hop_length
    if num_frames > MAX_FRAMES:
        raise InvalidParamError(
            f"num_frames {num_frames} exceeds MAX_FRAMES {MAX_FRAMES}"
        )

    bins = n_fft // 2 + 1
    out_len = num_frames * bins * 2
    if out_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"STFT output length {out_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    validate_floats(signal, "signal")

    win = build_window(window, n_fft)

    out = [0.0] * out_len
    windowed = [0.0] * n_fft
    frame_floats = bins * 2

    # Process each frame iteratively (CWE-674 zero recursion)
    for m in range(num_frames):
        frame_start = m * hop_length
        for k in range(n_fft):
            windowed[k] = signal[frame_start + k] * win[k]

        try:
            spectrum = rfft_scalar(windowed)
        except Exception as exc:
            raise FftError(f"FFT computation failed on frame {m}: {exc}") from exc

        out_offset = m * frame_floats
        for idx in range(frame_floats):
            out[out_offset + idx] = spectrum[idx]

    return out
