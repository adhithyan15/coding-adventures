"""Spectral and time-domain audio feature extraction (MUS08, VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

from music_analysis.types import (
    MAX_FRAMES,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    InvalidInputError,
    validate_positive_float,
    validate_samples,
    validate_spectrum,
)


def spectral_centroid(
    spectrum: Sequence[float],
    sample_rate_hz: float = 44100.0,
) -> float:
    """Compute the spectral centroid (center of mass / brightness) of a spectrum.

    Args:
        spectrum: Magnitude or power spectrum bins from 0 Hz to Nyquist.
        sample_rate_hz: Sampling frequency in Hertz.

    Returns:
        Spectral centroid frequency in Hertz.

    Raises:
        InvalidInputError: If parameters are non-finite or empty.
        BudgetExceededError: If spectrum length exceeds MAX_FFT_LEN.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    mags = validate_spectrum(spectrum, "spectrum")
    k_bins = len(mags)
    if k_bins <= 1:
        return 0.0

    nyquist = valid_sr / 2.0
    freq_step = nyquist / float(k_bins - 1)

    weighted_sum = 0.0
    total_mag = 0.0

    # Iterative center of mass computation (CWE-674 zero recursion)
    for k in range(k_bins):
        mag = abs(mags[k])
        freq = float(k) * freq_step
        weighted_sum += freq * mag
        total_mag += mag

    if total_mag <= 0.0:
        return 0.0
    return weighted_sum / total_mag


def spectral_flux(
    spectrum_prev: Sequence[float] | Sequence[Sequence[float]],
    spectrum_curr: Sequence[float] | None = None,
    *,
    rectified: bool = True,
) -> float | list[float]:
    """Compute half-wave rectified or squared spectral flux between frames.

    Args:
        spectrum_prev: Previous frame spectrum OR a list of successive frames.
        spectrum_curr: Current frame spectrum (if computing between 2 frames).
        rectified: If True, only positive increases in spectral energy are summed.

    Returns:
        Scalar flux between two frames, or list of fluxes across successive frames.
    """
    if spectrum_curr is not None:
        prev = validate_spectrum(spectrum_prev, "spectrum_prev")  # type: ignore[arg-type]
        curr = validate_spectrum(spectrum_curr, "spectrum_curr")
        n_bins = min(len(prev), len(curr))
        flux = 0.0
        for k in range(n_bins):
            diff = curr[k] - prev[k]
            if rectified:
                if diff > 0.0:
                    flux += diff
            else:
                flux += diff * diff
        return flux

    # Sequence of frames mode
    frames = spectrum_prev
    n_frames = len(frames)
    if n_frames == 0:
        raise InvalidInputError("frame count must be >= 1")
    if n_frames > MAX_FRAMES:
        from music_analysis.types import BudgetExceededError

        raise BudgetExceededError(
            f"frame count {n_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )
    if n_frames == 1:
        validate_spectrum(frames[0], "frames[0]")  # type: ignore[index]
        return []

    fluxes: list[float] = [0.0] * (n_frames - 1)
    prev_frame = validate_spectrum(frames[0], "frames[0]")  # type: ignore[index]
    for m in range(1, n_frames):
        curr_frame = validate_spectrum(frames[m], f"frames[{m}]")  # type: ignore[index]
        n_bins = min(len(prev_frame), len(curr_frame))
        f_val = 0.0
        for k in range(n_bins):
            diff = curr_frame[k] - prev_frame[k]
            if rectified:
                if diff > 0.0:
                    f_val += diff
            else:
                f_val += diff * diff
        fluxes[m - 1] = f_val
        prev_frame = curr_frame
    return fluxes


def spectral_rolloff(
    spectrum: Sequence[float],
    sample_rate_hz: float = 44100.0,
    rolloff_percent: float = 0.85,
) -> float:
    """Compute spectral rolloff frequency below which rolloff_percent energy lies.

    Args:
        spectrum: Magnitude or power spectrum bins from 0 Hz to Nyquist.
        sample_rate_hz: Sampling frequency in Hertz.
        rolloff_percent: Fraction of total energy in (0.0, 1.0] (default 0.85).

    Returns:
        Rolloff frequency in Hertz.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    valid_pct = validate_positive_float(
        rolloff_percent, "rolloff_percent", max_val=1.0
    )
    mags = validate_spectrum(spectrum, "spectrum")
    k_bins = len(mags)
    if k_bins <= 1:
        return 0.0

    total_energy = 0.0
    for val in mags:
        total_energy += abs(val)

    if total_energy <= 0.0:
        return 0.0

    target = total_energy * valid_pct
    nyquist = valid_sr / 2.0
    freq_step = nyquist / float(k_bins - 1)

    accum = 0.0
    for k in range(k_bins):
        accum += abs(mags[k])
        if accum >= target:
            return float(k) * freq_step

    return nyquist


def zero_crossing_rate(
    samples: Sequence[float],
    frame_length: int | None = None,
    hop_length: int | None = None,
) -> float | list[float]:
    """Compute zero crossing rate of signal or across sliding frames.

    Args:
        samples: Time-domain audio sample sequence.
        frame_length: Optional frame length for sliding window analysis.
        hop_length: Optional hop size between sliding frames.

    Returns:
        Scalar ZCR in [0.0, 1.0] if unwindowed, or list of ZCR values per frame.
    """
    valid_samples = validate_samples(samples, "samples")
    n = len(valid_samples)

    if frame_length is None:
        if n <= 1:
            return 0.0
        crossings = 0
        for i in range(1, n):
            s_curr = valid_samples[i]
            s_prev = valid_samples[i - 1]
            if (s_curr >= 0.0 and s_prev < 0.0) or (s_curr < 0.0 and s_prev >= 0.0):
                crossings += 1
        return float(crossings) / float(n - 1)

    fl = int(
        validate_positive_float(
            frame_length, "frame_length", max_val=float(MAX_SAMPLES)
        )
    )
    hop = int(
        validate_positive_float(
            fl if hop_length is None else hop_length,
            "hop_length",
            max_val=float(MAX_SAMPLES),
        )
    )
    if fl <= 0 or hop <= 0:
        raise InvalidInputError("frame_length and hop_length must be positive integers")

    if n < fl:
        crossings = 0
        for i in range(1, n):
            s_curr = valid_samples[i]
            s_prev = valid_samples[i - 1]
            if (s_curr >= 0.0 and s_prev < 0.0) or (s_curr < 0.0 and s_prev >= 0.0):
                crossings += 1
        val = float(crossings) / float(n - 1) if n > 1 else 0.0
        return [val]

    num_frames = 1 + (n - fl) // hop
    if num_frames > MAX_FRAMES:
        from music_analysis.types import BudgetExceededError

        raise BudgetExceededError(
            f"num_frames {num_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )

    out: list[float] = [0.0] * num_frames
    for m in range(num_frames):
        start = m * hop
        crossings = 0
        for i in range(1, fl):
            s_curr = valid_samples[start + i]
            s_prev = valid_samples[start + i - 1]
            if (s_curr >= 0.0 and s_prev < 0.0) or (s_curr < 0.0 and s_prev >= 0.0):
                crossings += 1
        out[m] = float(crossings) / float(fl - 1)

    return out


def rms_energy(
    samples: Sequence[float],
    frame_length: int | None = None,
    hop_length: int | None = None,
) -> float | list[float]:
    """Compute root-mean-square (RMS) energy of signal or across sliding frames.

    Args:
        samples: Time-domain audio sample sequence.
        frame_length: Optional frame length for sliding window analysis.
        hop_length: Optional hop size between sliding frames.

    Returns:
        Scalar RMS energy if unwindowed, or list of RMS values per frame.
    """
    valid_samples = validate_samples(samples, "samples")
    n = len(valid_samples)

    if frame_length is None:
        if n == 0:
            return 0.0
        sum_sq = 0.0
        for s in valid_samples:
            sum_sq += s * s
        return math.sqrt(sum_sq / float(n))

    fl = int(
        validate_positive_float(
            frame_length, "frame_length", max_val=float(MAX_SAMPLES)
        )
    )
    hop = int(
        validate_positive_float(
            fl if hop_length is None else hop_length,
            "hop_length",
            max_val=float(MAX_SAMPLES),
        )
    )
    if fl <= 0 or hop <= 0:
        raise InvalidInputError("frame_length and hop_length must be positive integers")

    if n < fl:
        if n == 0:
            return [0.0]
        sum_sq = 0.0
        for s in valid_samples:
            sum_sq += s * s
        return [math.sqrt(sum_sq / float(n))]

    num_frames = 1 + (n - fl) // hop
    if num_frames > MAX_FRAMES:
        from music_analysis.types import BudgetExceededError

        raise BudgetExceededError(
            f"num_frames {num_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )

    out: list[float] = [0.0] * num_frames
    for m in range(num_frames):
        start = m * hop
        sum_sq = 0.0
        for i in range(fl):
            s = valid_samples[start + i]
            sum_sq += s * s
        out[m] = math.sqrt(sum_sq / float(fl))

    return out
