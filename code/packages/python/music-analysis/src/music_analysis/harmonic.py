"""Harmonic profile and ADSR timbre estimation (MUS08, MUS05, VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_fft.rfft import rfft_scalar
from dsp_stft.windows import WindowType, build_window
from musical_instruments import (
    MAX_PARTIALS,
    ADSREnvelope,
    HarmonicPartial,
    InstrumentProfile,
    VariationProfile,
)

from music_analysis.features import rms_energy
from music_analysis.pitch import hps_pitch, yin_pitch
from music_analysis.types import (
    MAX_FFT_LEN,
    MAX_SAMPLE_RATE_HZ,
    InvalidInputError,
    validate_positive_float,
    validate_samples,
)


def extract_harmonic_profile(
    samples: Sequence[float],
    sample_rate_hz: float,
    fundamental_hz: float | None = None,
    max_partials: int = 16,
    *,
    instrument_id: str = "extracted_instrument",
    display_name: str = "Extracted Instrument",
) -> InstrumentProfile:
    """Extract fundamental, harmonic partial amplitudes, and ADSR envelope from audio.

    Returns a MUS05 InstrumentProfile suitable for re-synthesis in musical-instruments
    and portable score playback in music-machine.

    Args:
        samples: Time-domain audio sample buffer of a single note.
        sample_rate_hz: Sampling frequency in Hertz.
        fundamental_hz: Optional known fundamental frequency (Hz). If None, estimated.
        max_partials: Maximum number of harmonic partials to extract (up to 64).
        instrument_id: Identifier string for the resulting instrument profile.
        display_name: Human-readable display label.

    Returns:
        InstrumentProfile with additive synthesis kind, harmonics, and ADSR envelope.

    Raises:
        InvalidInputError: If parameters are invalid or signal is silent.
        BudgetExceededError: If resource bounds are exceeded.
    """
    valid_sr = validate_positive_float(
        sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
    )
    n_partials = int(
        validate_positive_float(
            max_partials, "max_partials", max_val=float(MAX_PARTIALS)
        )
    )
    raw_samples = (
        samples.samples if hasattr(samples, "samples") else samples  # type: ignore[union-attr]
    )
    sig = validate_samples(raw_samples, "samples")
    n = len(sig)

    overall_rms = float(rms_energy(sig))
    if overall_rms < 1e-4:
        raise InvalidInputError("signal is silent, cannot extract harmonic profile")

    # 1. Determine fundamental frequency
    if fundamental_hz is None:
        f0 = yin_pitch(sig, valid_sr)
        if f0 <= 0.0:
            f0 = hps_pitch(sig, valid_sr)
        if f0 <= 0.0:
            raise InvalidInputError("failed to detect fundamental pitch in signal")
    else:
        f0 = validate_positive_float(
            fundamental_hz, "fundamental_hz", max_val=valid_sr / 2.0
        )

    # 2. Extract harmonic partials from sustained portion
    target_fft = 4096
    if n > target_fft:
        # Take center portion
        start = (n - target_fft) // 2
        analysis_slice = sig[start : start + target_fft]
        fft_len = target_fft
    else:
        fft_len = max(512, 1 << (n - 1).bit_length())
        fft_len = min(MAX_FFT_LEN, fft_len)
        analysis_slice = sig + [0.0] * (fft_len - n)

    win = build_window(WindowType.HANN, fft_len)
    windowed = [analysis_slice[i] * win[i] for i in range(fft_len)]

    spec = rfft_scalar(windowed)
    k_bins = fft_len // 2 + 1
    mags = [0.0] * k_bins
    for k in range(k_bins):
        re = spec[2 * k]
        im = spec[2 * k + 1]
        mags[k] = math.sqrt(re * re + im * im)

    freq_step = valid_sr / float(fft_len)

    # Find fundamental amplitude
    k1 = int(round(f0 / freq_step))
    search_w = 4
    k1_low = max(0, k1 - search_w)
    k1_high = min(k_bins, k1 + search_w + 1)
    a1 = max(mags[k1_low:k1_high]) if k1_low < k1_high else 1.0
    if a1 <= 0.0:
        a1 = 1.0

    partials: list[HarmonicPartial] = []
    for h in range(1, n_partials + 1):
        fh = float(h) * f0
        if fh >= valid_sr / 2.0:
            break
        kh = int(round(fh / freq_step))
        kh_low = max(0, kh - search_w)
        kh_high = min(k_bins, kh + search_w + 1)
        if kh_low < kh_high:
            ah = max(mags[kh_low:kh_high])
            rel_amp = min(1.0, max(0.0, ah / a1))
        else:
            rel_amp = 0.0

        partials.append(
            HarmonicPartial(
                frequency_multiplier=float(h),
                amplitude=round(rel_amp, 4),
                phase_cycles=0.0,
            )
        )

    if not partials:
        partials = [
            HarmonicPartial(
                frequency_multiplier=1.0,
                amplitude=1.0,
                phase_cycles=0.0,
            )
        ]

    # 3. Estimate ADSR Envelope
    frame_len = 256
    hop_len = 128
    rms_track = rms_energy(sig, frame_length=frame_len, hop_length=hop_len)
    if not isinstance(rms_track, list) or len(rms_track) == 0:
        rms_track = [overall_rms]

    max_rms = max(rms_track)
    if max_rms <= 0.0:
        max_rms = 1.0

    peak_idx = 0
    for idx, r_val in enumerate(rms_track):
        if r_val == max_rms:
            peak_idx = idx
            break

    total_dur_sec = float(n) / valid_sr
    attack_sec = max(0.005, min(total_dur_sec * 0.4, (peak_idx * hop_len) / valid_sr))

    # Release detection: when energy drops below 15% of max near end
    rel_frame = len(rms_track) - 1
    while rel_frame > peak_idx and rms_track[rel_frame] < 0.15 * max_rms:
        rel_frame -= 1

    release_sec = max(
        0.005,
        min(
            total_dur_sec * 0.4,
            ((len(rms_track) - 1 - rel_frame) * hop_len) / valid_sr,
        ),
    )

    # Sustain level: average between peak and release
    sustain_start = min(peak_idx + 1, len(rms_track) - 1)
    sustain_end = max(sustain_start, rel_frame)
    if sustain_start < sustain_end:
        sustain_frames = rms_track[sustain_start:sustain_end]
        sustain_avg = sum(sustain_frames) / float(len(sustain_frames))
        sustain_level = min(1.0, max(0.05, sustain_avg / max_rms))
    else:
        sustain_level = 0.7

    decay_sec = max(0.005, min(total_dur_sec * 0.3, attack_sec * 0.5))

    envelope = ADSREnvelope(
        attack_seconds=round(min(60.0, attack_sec), 4),
        decay_seconds=round(min(60.0, decay_sec), 4),
        sustain_level=round(sustain_level, 3),
        release_seconds=round(min(60.0, release_sec), 4),
    )

    peak_amp = max(abs(x) for x in sig)
    gain = min(1.0, max(0.05, float(peak_amp)))

    return InstrumentProfile(
        id=instrument_id,
        display_name=display_name,
        synthesis_kind="additive",
        gain=round(gain, 3),
        harmonic_profile=tuple(partials),
        envelope_profile=envelope,
        variation_profile=VariationProfile(),
        family="custom",
    )
