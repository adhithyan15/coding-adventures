"""Iterative pitch detection: YIN and Harmonic Product Spectrum (MUS08, VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_fft.rfft import rfft_scalar
from dsp_stft.windows import WindowType, build_window

from music_analysis.features import rms_energy
from music_analysis.types import (
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_PARTIALS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    BudgetExceededError,
    InvalidInputError,
    validate_positive_float,
    validate_samples,
)


def yin_pitch(
    samples: Sequence[float],
    sample_rate_hz: float,
    *,
    f_min: float = 40.0,
    f_max: float = 2000.0,
    threshold: float = 0.15,
) -> float:
    """Estimate fundamental pitch using strictly iterative YIN algorithm.

    Args:
        samples: Time-domain audio samples.
        sample_rate_hz: Sampling frequency in Hertz.
        f_min: Minimum expected fundamental frequency in Hertz.
        f_max: Maximum expected fundamental frequency in Hertz.
        threshold: YIN absolute threshold for cumulative normalized difference.

    Returns:
        Estimated fundamental frequency in Hertz, or 0.0 if unpitched/silent.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    valid_fmin = validate_positive_float(f_min, "f_min", max_val=valid_sr / 2.0)
    valid_fmax = validate_positive_float(f_max, "f_max", max_val=valid_sr / 2.0)
    if valid_fmin >= valid_fmax:
        raise InvalidInputError(
            f"f_min ({valid_fmin}) must be < f_max ({valid_fmax})"
        )
    valid_thresh = validate_positive_float(threshold, "threshold", max_val=1.0)
    sig = validate_samples(samples, "samples")
    n = len(sig)

    # Silence check
    if float(rms_energy(sig)) < 1e-4:
        return 0.0

    tau_min = max(2, int(math.floor(valid_sr / valid_fmax)))
    tau_max = min(n // 2, int(math.ceil(valid_sr / valid_fmin)))

    if tau_max <= tau_min:
        return 0.0

    w_size = n - tau_max

    # Step 1: Difference function d(tau)
    d = [0.0] * (tau_max + 1)
    for tau in range(1, tau_max + 1):
        diff_sum = 0.0
        for j in range(w_size):
            diff = sig[j] - sig[j + tau]
            diff_sum += diff * diff
        d[tau] = diff_sum

    # Step 2: Cumulative mean normalized difference d'(tau)
    d_prime = [1.0] * (tau_max + 1)
    running_sum = 0.0
    for tau in range(1, tau_max + 1):
        running_sum += d[tau]
        if running_sum > 0.0:
            d_prime[tau] = d[tau] / (running_sum / float(tau))
        else:
            d_prime[tau] = 1.0

    # Step 3: Absolute threshold search
    best_tau = -1
    for tau in range(tau_min, tau_max + 1):
        if d_prime[tau] < valid_thresh:
            while tau + 1 <= tau_max and d_prime[tau + 1] < d_prime[tau]:
                tau += 1
            best_tau = tau
            break

    # Fallback to global minimum in range
    if best_tau == -1:
        min_val = float("inf")
        for tau in range(tau_min, tau_max + 1):
            if d_prime[tau] < min_val:
                min_val = d_prime[tau]
                best_tau = tau
        if min_val > 0.85:
            return 0.0

    # Step 4: Parabolic interpolation
    if tau_min < best_tau < tau_max:
        y0 = d_prime[best_tau - 1]
        y1 = d_prime[best_tau]
        y2 = d_prime[best_tau + 1]
        denom = 2.0 * (y0 - 2.0 * y1 + y2)
        if abs(denom) > 1e-12:
            delta = (y0 - y2) / denom
            if abs(delta) < 1.0:
                refined_tau = float(best_tau) + delta
            else:
                refined_tau = float(best_tau)
        else:
            refined_tau = float(best_tau)
    else:
        refined_tau = float(best_tau)

    if refined_tau <= 0.0:
        return 0.0

    pitch = valid_sr / refined_tau
    if valid_fmin <= pitch <= valid_fmax:
        return pitch
    return 0.0


def yin_pitch_track(
    samples: Sequence[float],
    sample_rate_hz: float,
    *,
    frame_length: int = 2048,
    hop_length: int = 512,
    f_min: float = 40.0,
    f_max: float = 2000.0,
    threshold: float = 0.15,
) -> list[float]:
    """Track fundamental pitch over time using windowed YIN analysis.

    Args:
        samples: Time-domain audio sample sequence.
        sample_rate_hz: Sampling frequency in Hertz.
        frame_length: Length of each analysis frame in samples.
        hop_length: Step size between consecutive frame starts.
        f_min: Minimum fundamental frequency.
        f_max: Maximum fundamental frequency.
        threshold: YIN threshold.

    Returns:
        List of fundamental frequencies per frame (in Hertz, 0.0 for unpitched).
    """
    valid_samples = validate_samples(samples, "samples")
    n = len(valid_samples)
    fl = int(
        validate_positive_float(
            frame_length, "frame_length", max_val=float(MAX_SAMPLES)
        )
    )
    hop = int(
        validate_positive_float(
            hop_length, "hop_length", max_val=float(MAX_SAMPLES)
        )
    )

    if n < fl:
        pitch = yin_pitch(
            valid_samples,
            sample_rate_hz,
            f_min=f_min,
            f_max=f_max,
            threshold=threshold,
        )
        return [pitch]

    num_frames = 1 + (n - fl) // hop
    if num_frames > MAX_FRAMES:
        raise BudgetExceededError(
            f"num_frames {num_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )

    pitches: list[float] = [0.0] * num_frames
    for m in range(num_frames):
        start = m * hop
        frame = valid_samples[start : start + fl]
        pitches[m] = yin_pitch(
            frame,
            sample_rate_hz,
            f_min=f_min,
            f_max=f_max,
            threshold=threshold,
        )

    return pitches


def hps_pitch(
    samples: Sequence[float],
    sample_rate_hz: float,
    *,
    num_harmonics: int = 5,
    f_min: float = 40.0,
    f_max: float = 2000.0,
    n_fft: int | None = None,
) -> float:
    """Estimate fundamental pitch via Harmonic Product Spectrum (HPS).

    Args:
        samples: Time-domain audio samples.
        sample_rate_hz: Sampling frequency in Hertz.
        num_harmonics: Number of harmonic spectra to compress (default 5).
        f_min: Minimum frequency search bound.
        f_max: Maximum frequency search bound.
        n_fft: FFT length (defaults to power of two >= len(samples)).

    Returns:
        Estimated fundamental pitch in Hertz, or 0.0 if unpitched.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    if isinstance(num_harmonics, bool) or not isinstance(num_harmonics, int):
        raise InvalidInputError("num_harmonics must be an integer")
    if num_harmonics < 1:
        raise InvalidInputError("num_harmonics must be >= 1")
    if num_harmonics > MAX_PARTIALS:
        raise BudgetExceededError(
            f"num_harmonics {num_harmonics} exceeds MAX_PARTIALS={MAX_PARTIALS}"
        )
    valid_fmin = validate_positive_float(f_min, "f_min", max_val=valid_sr / 2.0)
    valid_fmax = validate_positive_float(f_max, "f_max", max_val=valid_sr / 2.0)
    if valid_fmin >= valid_fmax:
        raise InvalidInputError("f_min must be < f_max")
    sig = validate_samples(samples, "samples")
    n = len(sig)

    if float(rms_energy(sig)) < 1e-4:
        return 0.0

    # Determine power-of-two FFT length
    if n_fft is None:
        target = max(1024, 1 << (n - 1).bit_length())
        fft_len = min(8192, target)
    else:
        fft_len = int(
            validate_positive_float(n_fft, "n_fft", max_val=float(MAX_FFT_LEN))
        )

    if fft_len > MAX_FFT_LEN:
        raise BudgetExceededError(f"n_fft {fft_len} exceeds MAX_FFT_LEN")

    # Build window and apply
    win = build_window(WindowType.HANN, min(n, fft_len))
    windowed = [0.0] * fft_len
    take_len = min(n, fft_len)
    for i in range(take_len):
        windowed[i] = sig[i] * win[i]

    spec = rfft_scalar(windowed)
    k_bins = fft_len // 2 + 1
    mags = [0.0] * k_bins
    for k in range(k_bins):
        re = spec[2 * k]
        im = spec[2 * k + 1]
        mags[k] = math.sqrt(re * re + im * im)

    freq_step = valid_sr / float(fft_len)
    k_min = max(1, int(math.floor(valid_fmin / freq_step)))
    k_max = min(k_bins // num_harmonics, int(math.ceil(valid_fmax / freq_step)))

    if k_max <= k_min:
        return 0.0

    best_k = k_min
    max_hps = -float("inf")
    hps_scores: dict[int, float] = {}

    for k in range(k_min, k_max + 1):
        log_prod = 0.0
        for h in range(1, num_harmonics + 1):
            log_prod += math.log(mags[k * h] + 1e-9)
        hps_scores[k] = log_prod
        if log_prod > max_hps:
            max_hps = log_prod
            best_k = k

    # Parabolic interpolation
    if k_min < best_k < k_max:
        y0 = hps_scores.get(best_k - 1, max_hps)
        y1 = max_hps
        y2 = hps_scores.get(best_k + 1, max_hps)
        denom = 2.0 * (y0 - 2.0 * y1 + y2)
        if abs(denom) > 1e-12:
            delta = (y0 - y2) / denom
            refined_k = float(best_k) + delta if abs(delta) < 1.0 else float(best_k)
        else:
            refined_k = float(best_k)
    else:
        refined_k = float(best_k)

    pitch = refined_k * freq_step
    if valid_fmin <= pitch <= valid_fmax:
        return pitch
    return 0.0
