"""Onset detection via spectral flux and wavelet detail energy (MUS08, VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_stft.stft import stft
from dsp_stft.windows import WindowType
from dsp_wavelets.dwt import dwt

from music_analysis.features import rms_energy
from music_analysis.types import (
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_ONSETS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    BudgetExceededError,
    validate_positive_float,
    validate_samples,
)


def detect_onsets(
    samples: Sequence[float],
    sample_rate_hz: float,
    *,
    hop_length: int = 512,
    n_fft: int = 1024,
    threshold_factor: float = 1.5,
    min_distance_seconds: float = 0.08,
    return_samples: bool = False,
) -> list[float] | list[int]:
    """Detect musical note onset timestamps combining STFT flux and DWT energy.

    Args:
        samples: Time-domain audio sample buffer.
        sample_rate_hz: Audio sampling frequency in Hertz.
        hop_length: Step size between analysis windows.
        n_fft: Window size for Short-Time Fourier Transform.
        threshold_factor: Multiplier for adaptive standard-deviation threshold.
        min_distance_seconds: Refractory lockout duration between consecutive onsets.
        return_samples: If True, return sample indices instead of seconds.

    Returns:
        List of onset positions in seconds (or sample indices if return_samples=True).

    Raises:
        InvalidInputError: If inputs are invalid or non-finite.
        BudgetExceededError: If resource limits are exceeded.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    sig = validate_samples(samples, "samples")
    n = len(sig)

    hop = int(
        validate_positive_float(
            hop_length, "hop_length", max_val=float(MAX_SAMPLES)
        )
    )
    fft_len = int(
        validate_positive_float(
            n_fft, "n_fft", max_val=float(MAX_FFT_LEN)
        )
    )
    t_factor = validate_positive_float(
        threshold_factor, "threshold_factor", max_val=10.0
    )
    min_dist_sec = validate_positive_float(
        min_distance_seconds, "min_distance_seconds", max_val=10.0
    )

    if float(rms_energy(sig)) < 1e-4:
        return []

    # If clip is shorter than one FFT window, return single onset at 0 if active
    if n < fft_len:
        return [0] if return_samples else [0.0]

    # Pre-flight frame budget enforcement BEFORE calling stft/allocating
    expected_frames = 1 + (n - fft_len) // hop
    if expected_frames > MAX_FRAMES:
        raise BudgetExceededError(
            f"frame count {expected_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )

    # --- 1. Spectral Flux Novelty Curve ---
    stft_matrix = stft(sig, fft_len, hop, WindowType.HANN)
    bins = fft_len // 2 + 1
    frame_floats = bins * 2
    num_frames = len(stft_matrix) // frame_floats

    if num_frames <= 1:
        return [0] if return_samples else [0.0]

    mags: list[list[float]] = []
    for m in range(num_frames):
        offset = m * frame_floats
        frame_mag = [0.0] * bins
        for k in range(bins):
            re = stft_matrix[offset + 2 * k]
            im = stft_matrix[offset + 2 * k + 1]
            frame_mag[k] = math.sqrt(re * re + im * im)
        mags.append(frame_mag)

    sf = [0.0] * num_frames
    for m in range(1, num_frames):
        flux = 0.0
        prev_m = mags[m - 1]
        curr_m = mags[m]
        for k in range(bins):
            diff = curr_m[k] - prev_m[k]
            if diff > 0.0:
                flux += diff
        sf[m] = flux

    max_sf = max(sf) if sf else 0.0
    if max_sf > 0.0:
        sf = [x / max_sf for x in sf]

    # --- 2. Wavelet Detail Energy Difference Curve ---
    try:
        _, cd = dwt(sig, "haar")
    except Exception:
        cd = []

    we_diff = [0.0] * num_frames
    if cd:
        d_hop = max(1, hop // 2)
        n_cd = len(cd)
        raw_we = [0.0] * num_frames
        for m in range(num_frames):
            c_start = m * d_hop
            c_end = min(n_cd, c_start + d_hop)
            if c_start < n_cd:
                e_sum = 0.0
                for idx in range(c_start, c_end):
                    val = cd[idx]
                    e_sum += val * val
                raw_we[m] = e_sum

        for m in range(1, num_frames):
            diff = raw_we[m] - raw_we[m - 1]
            if diff > 0.0:
                we_diff[m] = diff

        max_we = max(we_diff) if we_diff else 0.0
        if max_we > 0.0:
            we_diff = [x / max_we for x in we_diff]

    # --- 3. Combined Novelty Curve ---
    novelty = [0.0] * num_frames
    for m in range(num_frames):
        novelty[m] = 0.5 * sf[m] + 0.5 * we_diff[m]

    # --- 4. Adaptive Peak Picking ---
    window_half = 3  # 7-frame moving window
    detected_frames: list[int] = []

    # Check if frame 0 starts with immediate active energy
    initial_check_len = min(n, fft_len)
    if float(rms_energy(sig[:initial_check_len])) > 0.02:
        detected_frames.append(0)

    min_dist_frames = max(1, int(round(min_dist_sec * valid_sr / float(hop))))

    for m in range(1, num_frames - 1):
        val = novelty[m]
        # Must be local peak
        if val <= novelty[m - 1] or val < novelty[m + 1]:
            continue

        # Local window statistics
        w_start = max(0, m - window_half)
        w_end = min(num_frames, m + window_half + 1)
        w_len = w_end - w_start
        mean_val = sum(novelty[w_start:w_end]) / float(w_len)
        var_sum = sum((novelty[idx] - mean_val) ** 2 for idx in range(w_start, w_end))
        std_val = math.sqrt(var_sum / float(w_len))

        threshold = mean_val + t_factor * std_val
        is_far_enough = (
            not detected_frames or (m - detected_frames[-1]) >= min_dist_frames
        )
        if val >= threshold and val > 0.15 and is_far_enough:
            detected_frames.append(m)

    if len(detected_frames) > MAX_ONSETS:
        raise BudgetExceededError(
            f"detected onsets {len(detected_frames)} exceeds MAX_ONSETS={MAX_ONSETS}"
        )

    if not detected_frames:
        detected_frames.append(0)

    if return_samples:
        return [f * hop for f in detected_frames]

    return [float(f * hop) / valid_sr for f in detected_frames]
