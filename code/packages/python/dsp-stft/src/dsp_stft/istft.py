"""Inverse Short-Time Fourier Transform (ISTFT) via overlap-add."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_fft.rfft import irfft_scalar

from dsp_stft.errors import (
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_SAMPLES,
    FftError,
    InvalidParamError,
    InvalidSpectrogramError,
    validate_floats,
)
from dsp_stft.windows import WindowType, build_window


def istft(
    spectrogram: Sequence[float],
    n_fft: int,
    hop_length: int,
    window: WindowType | str = WindowType.HANN,
    output_length: int | None = None,
) -> list[float]:
    """Reconstruct time-domain signal from STFT spectrogram via overlap-add (OLA).

    Under COLA-satisfying choices (e.g. Hann window with hop_length = n_fft // 2),
    istft(stft(x)) reconstructs x up to floating-point precision.
    """
    if len(spectrogram) == 0:
        raise InvalidSpectrogramError("spectrogram is empty")
    if n_fft <= 0:
        raise InvalidParamError(f"n_fft must be > 0; got {n_fft}")
    if hop_length <= 0:
        raise InvalidParamError(f"hop_length must be > 0; got {hop_length}")
    if n_fft > MAX_FFT_LEN:
        raise InvalidParamError(
            f"n_fft {n_fft} exceeds MAX_FFT_LEN {MAX_FFT_LEN}"
        )

    bins = n_fft // 2 + 1
    frame_floats = bins * 2
    if len(spectrogram) % frame_floats != 0:
        raise InvalidSpectrogramError(
            f"spectrogram length {len(spectrogram)} is not a multiple of "
            f"bins*2 = {frame_floats}"
        )

    num_frames = len(spectrogram) // frame_floats
    if num_frames > MAX_FRAMES:
        raise InvalidParamError(
            f"num_frames {num_frames} exceeds MAX_FRAMES {MAX_FRAMES}"
        )

    if output_length is None:
        out_len = (num_frames - 1) * hop_length + n_fft
    else:
        if output_length <= 0:
            raise InvalidParamError(
                f"output_length must be > 0; got {output_length}"
            )
        out_len = output_length

    if out_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"output_length {out_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    validate_floats(spectrogram, "spectrogram", exc_type=InvalidSpectrogramError)

    win = build_window(window, n_fft)

    out = [0.0] * out_len
    norm = [0.0] * out_len

    # Overlap-add synthesis loop (iterative, CWE-674)
    for m in range(num_frames):
        frame_start = m * hop_length
        frame_offset = m * frame_floats
        frame_slice = spectrogram[frame_offset : frame_offset + frame_floats]

        try:
            time_frame = irfft_scalar(list(frame_slice), n_fft)
        except Exception as exc:
            raise FftError(f"IFFT failure on frame {m}: {exc}") from exc

        for k in range(n_fft):
            n_idx = frame_start + k
            if n_idx >= out_len:
                break
            w = win[k]
            out[n_idx] += time_frame[k] * w
            norm[n_idx] += w * w

    # Normalisation with NOLA non-zero overlap-add guard
    for n_idx in range(out_len):
        if norm[n_idx] > 1e-10:
            out[n_idx] /= norm[n_idx]

    return out
