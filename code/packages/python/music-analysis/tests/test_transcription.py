"""Unit tests for sample-to-score transcription round-trip (MUS08, MUS04)."""

from __future__ import annotations

import math

import pytest
from music_machine import (
    PORTABLE_FORMAT_VERSION,
    PortableScore,
    PortableScoreBuilder,
    render_portable_score_to_pcm,
)
from pcm_audio import PCMBuffer, PCMFormat

from music_analysis import transcribe_pcm_to_portable_score
from music_analysis.types import InvalidInputError


def test_transcribe_portable_score_round_trip() -> None:
    sr = 44100
    bpm = 120.0
    ppq = 480

    builder = PortableScoreBuilder(title="Round Trip Triad", ppq=ppq, sample_rate_hz=sr)
    builder.add_tempo(0, bpm)
    builder.add_meter(0, "4/4")
    builder.add_instrument("lead", profile="sine", gain=0.8)
    builder.add_track("melody", instrument_id="lead")
    builder.add_note("melody", 0, 480, "C4")
    builder.add_note("melody", 480, 480, "E4")
    builder.add_note("melody", 960, 480, "G4")
    original_score = builder.build()

    rendered = render_portable_score_to_pcm(original_score)
    assert isinstance(rendered.pcm_buffer, PCMBuffer)

    transcribed = transcribe_pcm_to_portable_score(
        rendered.pcm_buffer,
        bpm=bpm,
        ppq=ppq,
        title="Transcribed Triad",
    )

    assert isinstance(transcribed, PortableScore)
    assert transcribed.format_version == PORTABLE_FORMAT_VERSION
    assert transcribed.title == "Transcribed Triad"
    assert transcribed.ppq == ppq
    assert transcribed.sample_rate_hz == sr

    # Verify notes and ticks match
    assert len(transcribed.events) == 3

    e0 = transcribed.events[0]
    assert e0.notes == ("C4",)
    assert e0.start_tick == 0
    assert e0.duration_tick == 480

    e1 = transcribed.events[1]
    assert e1.notes == ("E4",)
    assert e1.start_tick == 480
    assert e1.duration_tick == 480

    e2 = transcribed.events[2]
    assert e2.notes == ("G4",)
    assert e2.start_tick == 960
    assert e2.duration_tick == 480


def test_transcribe_raw_float_sequence() -> None:
    sr = 44100.0
    bpm = 120.0
    # Create 0.5s of A4 (440 Hz)
    n = int(0.5 * sr)
    phase_inc = 2.0 * math.pi * 440.0 / sr
    sig = [0.8 * math.sin(i * phase_inc) for i in range(n)]

    score = transcribe_pcm_to_portable_score(sig, sample_rate_hz=sr, bpm=bpm)
    assert isinstance(score, PortableScore)
    assert len(score.events) == 1
    event = score.events[0]
    assert event.notes == ("A4",)
    assert event.start_tick == 0


def test_transcribe_pcm16_integer_sequence() -> None:
    sr = 44100.0
    bpm = 120.0
    # Create 0.5s of C4 (261.63 Hz) in 16-bit PCM integer range
    n = int(0.5 * sr)
    phase_inc = 2.0 * math.pi * 261.63 / sr
    sig = [int(24000.0 * math.sin(i * phase_inc)) for i in range(n)]

    score = transcribe_pcm_to_portable_score(sig, sample_rate_hz=sr, bpm=bpm)
    assert len(score.events) == 1
    assert score.events[0].notes == ("C4",)


def test_transcribe_silence() -> None:
    sr = 44100.0
    sig = [0.0] * 8000
    score = transcribe_pcm_to_portable_score(sig, sample_rate_hz=sr, bpm=120.0)
    assert isinstance(score, PortableScore)
    assert len(score.events) == 0


def test_transcribe_pcm_buffer_metadata() -> None:
    sr = 22050
    pcm_format = PCMFormat(sample_rate_hz=sr, channel_count=1, bit_depth=16)
    # 0.5s tone
    n = int(0.5 * sr)
    phase_inc = 2.0 * math.pi * 440.0 / sr
    int_samples = tuple(int(20000.0 * math.sin(i * phase_inc)) for i in range(n))
    buf = PCMBuffer(samples=int_samples, pcm_format=pcm_format)

    # When passing PCMBuffer, sample_rate_hz is inferred automatically
    score = transcribe_pcm_to_portable_score(buf, bpm=120.0)
    assert score.sample_rate_hz == sr
    assert len(score.events) == 1
    assert score.events[0].notes == ("A4",)


def test_transcribe_invalid_inputs() -> None:
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([1.0], sample_rate_hz=None)
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([1.0], sample_rate_hz=100.0)  # below 1000 Hz
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([1.0], sample_rate_hz=44100.0, bpm=0.0)
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score(["invalid"], sample_rate_hz=44100.0)
