"""Sample-to-Score transcription producing MUS04 PortableScore (MUS08, VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

from music_machine import (
    PORTABLE_FORMAT_VERSION,
    InstrumentDeclaration,
    MeterEvent,
    PortableScore,
    PortableScoreEvent,
    TempoEvent,
    TrackDeclaration,
)
from pcm_audio import PCMBuffer

from music_analysis.features import rms_energy
from music_analysis.frequency import frequency_to_note
from music_analysis.onset import detect_onsets
from music_analysis.pitch import hps_pitch, yin_pitch
from music_analysis.types import (
    MAX_BPM,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MIN_BPM,
    MIN_SAMPLE_RATE_HZ,
    BudgetExceededError,
    InvalidInputError,
    validate_positive_float,
    validate_samples,
)


def transcribe_pcm_to_portable_score(
    samples: Sequence[float | int] | PCMBuffer,
    sample_rate_hz: float | None = None,
    *,
    bpm: float = 120.0,
    ppq: int = 480,
    quantize_grid_beats: float = 0.25,
    title: str = "Transcribed Score",
    instrument_id: str = "sine",
    track_id: str = "melody",
) -> PortableScore:
    """Transcribe raw audio samples or PCMBuffer into a MUS04 PortableScore.

    Segments audio into discrete notes via multi-resolution onset detection,
    estimates pitch using YIN and HPS, quantizes note timings to a musical
    beat grid at the given tempo, and returns a verified PortableScore.

    Args:
        samples: Sequence of float/int samples or a pcm_audio.PCMBuffer.
        sample_rate_hz: Sampling rate in Hertz (required if samples is not PCMBuffer).
        bpm: Beats per minute tempo for metric grid quantization (default 120.0).
        ppq: Pulses (ticks) per quarter note (default 480).
        quantize_grid_beats: Rhythmic quantization step in beats (0.25 = 16th note).
        title: Score title string.
        instrument_id: Instrument preset profile identifier for the track.
        track_id: Track identifier string.

    Returns:
        PortableScore conforming to music-machine-score/v2.

    Raises:
        InvalidInputError: If sample rate, tempo, or sample values are invalid.
        BudgetExceededError: If inputs exceed pre-flight bounds.
    """
    if isinstance(ppq, bool) or not isinstance(ppq, int):
        raise InvalidInputError("ppq must be an integer")
    if not (1 <= ppq <= 9600):
        raise InvalidInputError(f"ppq {ppq} out of valid range [1, 9600]")

    valid_quantize = validate_positive_float(
        quantize_grid_beats, "quantize_grid_beats", max_val=16.0
    )

    valid_bpm = validate_positive_float(bpm, "bpm", max_val=MAX_BPM)
    if valid_bpm < MIN_BPM:
        raise InvalidInputError(f"bpm {valid_bpm} below minimum {MIN_BPM}")

    if isinstance(samples, PCMBuffer):
        valid_sr = validate_positive_float(
            samples.pcm_format.sample_rate_hz,
            "sample_rate_hz",
            max_val=MAX_SAMPLE_RATE_HZ,
        )
        if valid_sr < MIN_SAMPLE_RATE_HZ:
            raise InvalidInputError(
                f"sample_rate_hz {valid_sr} below minimum {MIN_SAMPLE_RATE_HZ}"
            )
        raw = samples.samples
        if len(raw) == 0:
            raise InvalidInputError("samples must be non-empty")
        if len(raw) > MAX_SAMPLES:
            raise BudgetExceededError(
                f"sample count {len(raw)} exceeds MAX_SAMPLES={MAX_SAMPLES}"
            )
        float_samples = [float(s) / 32767.0 for s in raw]
    else:
        if sample_rate_hz is None:
            raise InvalidInputError(
                "sample_rate_hz must be specified when samples is not a PCMBuffer"
            )
        valid_sr = validate_positive_float(
            sample_rate_hz, "sample_rate_hz", max_val=MAX_SAMPLE_RATE_HZ
        )
        if valid_sr < MIN_SAMPLE_RATE_HZ:
            raise InvalidInputError(
                f"sample_rate_hz {valid_sr} below minimum {MIN_SAMPLE_RATE_HZ}"
            )
        if len(samples) == 0:
            raise InvalidInputError("samples must be non-empty")
        if len(samples) > MAX_SAMPLES:
            raise BudgetExceededError(
                f"sample count {len(samples)} exceeds MAX_SAMPLES={MAX_SAMPLES}"
            )

        float_samples = []
        for idx, s in enumerate(samples):
            if isinstance(s, bool) or not isinstance(s, (int, float)):
                raise InvalidInputError(f"sample at {idx} must be numeric")
            fs = float(s)
            if not math.isfinite(fs):
                raise InvalidInputError(f"sample at {idx} must be finite")
            float_samples.append(fs)

        is_int_pcm = any(abs(s) > 1.5 for s in float_samples[:100])
        if is_int_pcm:
            float_samples = [s / 32767.0 for s in float_samples]

    sig = validate_samples(float_samples, "samples")
    n = len(sig)

    overall_rms = float(rms_energy(sig))

    events: list[PortableScoreEvent] = []

    if overall_rms >= 1e-4:
        onsets = detect_onsets(sig, valid_sr, min_distance_seconds=0.08)
        if not onsets:
            onsets = [0.0]

        total_dur_sec = float(n) / valid_sr
        tps = (valid_bpm * float(ppq)) / 60.0
        grid_step_ticks = max(1, int(round(valid_quantize * float(ppq))))

        for idx, t_start in enumerate(onsets):
            t_next = onsets[idx + 1] if idx + 1 < len(onsets) else total_dur_sec
            s_start = int(t_start * valid_sr)
            s_end = min(n, int(t_next * valid_sr))

            if s_end - s_start < 128:
                continue

            slice_samples = sig[s_start:s_end]
            slice_dur = float(len(slice_samples)) / valid_sr

            # Estimate active duration: find where energy stays significant
            hop_len = max(64, len(slice_samples) // 16)
            rms_frames = rms_energy(
                slice_samples,
                frame_length=min(256, len(slice_samples)),
                hop_length=hop_len,
            )
            if isinstance(rms_frames, list) and rms_frames:
                max_frame_rms = max(rms_frames)
                active_count = sum(
                    1 for r in rms_frames if r >= 0.2 * max_frame_rms
                )
                dur_sec = max(
                    0.05, min(slice_dur, float(active_count * hop_len) / valid_sr)
                )
            else:
                dur_sec = slice_dur

            # Extract pitch from stable portion
            skip_transient = min(len(slice_samples) // 4, int(0.015 * valid_sr))
            stable = slice_samples[skip_transient:]
            pitch = yin_pitch(stable, valid_sr)
            if pitch <= 0.0:
                pitch = hps_pitch(stable, valid_sr)

            if pitch > 0.0:
                note_match = frequency_to_note(pitch)
                raw_start_tick = t_start * tps
                start_tick = (
                    int(round(raw_start_tick / grid_step_ticks)) * grid_step_ticks
                )

                raw_dur_ticks = dur_sec * tps
                dur_ticks = max(
                    grid_step_ticks,
                    int(round(raw_dur_ticks / grid_step_ticks)) * grid_step_ticks,
                )

                vel = min(1.0, max(0.1, float(rms_energy(stable)) * 2.5))

                events.append(
                    PortableScoreEvent(
                        track_id=track_id,
                        start_tick=start_tick,
                        duration_tick=dur_ticks,
                        kind="note",
                        notes=(note_match.note,),
                        velocity=round(vel, 2),
                        source_order=len(events),
                    )
                )

    inst_decl = InstrumentDeclaration(
        id=instrument_id,
        profile_id=instrument_id,
        kind="preset",
        gain=1.0,
    )
    track_decl = TrackDeclaration(id=track_id, instrument_id=instrument_id)

    return PortableScore(
        format_version=PORTABLE_FORMAT_VERSION,
        title=title,
        ppq=ppq,
        sample_rate_hz=int(valid_sr),
        tempo_events=(TempoEvent(start_tick=0, bpm=valid_bpm),),
        meter_events=(MeterEvent(start_tick=0, meter="4/4"),),
        instruments=(inst_decl,),
        tracks=(track_decl,),
        events=tuple(events),
    )
