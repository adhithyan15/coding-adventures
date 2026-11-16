"""Tests for the visible note-to-sound signal chain."""

from __future__ import annotations

import math
import wave
from io import BytesIO
from pathlib import Path

import pytest
from note_frequency import parse_note

from note_audio import (
    DEFAULT_AMPLITUDE,
    DEFAULT_MAX_SAMPLE_COUNT,
    DEFAULT_SAMPLE_RATE_HZ,
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_COUNT,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    NoteEvent,
    PCMBuffer,
    PCMFormat,
    RenderedNote,
    ZeroOrderHoldDACSignal,
    __version__,
    encode_sample_buffer,
    float_to_pcm16,
    render_note_to_sound_chain,
    to_wav_bytes,
    write_wav,
)

ABS_TOLERANCE = 1e-9


# =============================================================================
# § 1. Pure Python SplitMix64 PRNG for PBT / Fuzzing
# =============================================================================


class SplitMix64:
    """Deterministic 64-bit PRNG (Steele et al., 2014) for reproducible PBT."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
        return z ^ (z >> 31)

    def next_float(self) -> float:
        """Return uniform float in [0.0, 1.0)."""
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, low: float, high: float) -> float:
        """Return uniform float in [low, high)."""
        return low + (high - low) * self.next_float()


def assert_close(got: float, want: float, tolerance: float = ABS_TOLERANCE) -> None:
    """Keep float assertions readable in the tutorial-style tests."""
    assert abs(got - want) <= tolerance


def test_version_and_constants_exist() -> None:
    assert __version__ == "0.1.0"
    assert MAX_SAMPLE_RATE_HZ == 192_000.0
    assert MAX_DURATION_SECONDS == 60.0
    assert MAX_SAMPLES == 16_777_216
    assert MAX_SAMPLE_COUNT == 16_777_216
    assert DEFAULT_MAX_SAMPLE_COUNT == 16_777_216
    assert DEFAULT_AMPLITUDE == 0.8


def test_note_event_parses_strings_and_preserves_timing() -> None:
    event = NoteEvent(
        "a4",
        duration_seconds=0.5,
        amplitude=0.25,
        start_time_seconds=2.0,
    )

    assert str(event.note) == "A4"
    assert event.duration_seconds == 0.5
    assert event.amplitude == 0.25
    assert event.start_time_seconds == 2.0


def test_note_event_accepts_parsed_notes() -> None:
    note = parse_note("C4")
    event = NoteEvent(note, duration_seconds=1.0)

    assert event.note is note
    assert_close(event.note.frequency(), 261.6255653005986)


def test_render_a4_keeps_every_layer_visible() -> None:
    chain = render_note_to_sound_chain("A4", duration_seconds=0.01)

    assert str(chain.note_event.note) == "A4"
    assert chain.frequency_hz == 440.0
    assert chain.oscillator.frequency_hz == 440.0
    assert chain.oscillator.amplitude == 0.8
    assert chain.floating_samples.sample_rate_hz == DEFAULT_SAMPLE_RATE_HZ
    assert chain.floating_samples.sample_count() == 441
    assert chain.pcm_buffer.sample_count() == 441
    assert chain.pcm_buffer.clipped_sample_count == 0
    assert chain.pcm_buffer.samples[0] == 0
    assert chain.dac_signal.value_at(0.0) == 0.0
    assert chain.speaker_signal.value_at(0.0) == 0.0

    first_nonzero_time = chain.pcm_buffer.time_at(1)
    assert chain.dac_signal.value_at(first_nonzero_time) > 0.0
    assert chain.speaker_signal.value_at(first_nonzero_time) > 0.0


def test_render_uses_start_time_for_samples_and_dac_hold() -> None:
    chain = render_note_to_sound_chain(
        "A4",
        duration_seconds=0.001,
        start_time_seconds=10.0,
    )

    assert_close(chain.floating_samples.time_at(0), 10.0)
    assert chain.dac_signal.value_at(9.999) == 0.0
    assert chain.dac_signal.value_at(10.0) == 0.0
    assert chain.dac_signal.value_at(10.0 + chain.pcm_buffer.duration_seconds()) == 0.0


def test_zero_duration_render_is_an_empty_but_valid_chain() -> None:
    chain = render_note_to_sound_chain("A4", duration_seconds=0.0)

    assert chain.floating_samples.sample_count() == 0
    assert chain.pcm_buffer.sample_count() == 0
    assert chain.dac_signal.value_at(0.0) == 0.0
    assert chain.speaker_signal.value_at(0.0) == 0.0


def test_speaker_gain_scales_pressure_linearly() -> None:
    chain = render_note_to_sound_chain(
        "A4",
        duration_seconds=0.001,
        speaker_gain=3.0,
    )

    sample_time = chain.pcm_buffer.time_at(1)
    voltage = chain.dac_signal.value_at(sample_time)
    pressure = chain.speaker_signal.value_at(sample_time)

    assert_close(pressure, voltage * 3.0)


def test_rendered_note_frequency_validates() -> None:
    chain = render_note_to_sound_chain("A4", duration_seconds=0.001)
    with pytest.raises(ValueError, match="frequency_hz"):
        RenderedNote(
            note_event=chain.note_event,
            frequency_hz=0.0,
            oscillator=chain.oscillator,
            floating_samples=chain.floating_samples,
            pcm_buffer=chain.pcm_buffer,
            dac_signal=chain.dac_signal,
            speaker_signal=chain.speaker_signal,
        )


def test_to_wav_bytes_round_trip() -> None:
    chain = render_note_to_sound_chain("A4", duration_seconds=0.01)
    wav_bytes = to_wav_bytes(chain.pcm_buffer)

    assert wav_bytes.startswith(b"RIFF")
    with wave.open(BytesIO(wav_bytes), "rb") as wav_file:
        assert wav_file.getnchannels() == 1
        assert wav_file.getsampwidth() == 2
        assert wav_file.getframerate() == int(DEFAULT_SAMPLE_RATE_HZ)
        assert wav_file.getnframes() == chain.pcm_buffer.sample_count()


def test_write_wav_writes_the_same_bytes(tmp_path: Path) -> None:
    pcm_buffer = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
    output_path = tmp_path / "tone.wav"

    returned_path = write_wav(output_path, pcm_buffer)

    assert returned_path == output_path
    assert output_path.read_bytes() == to_wav_bytes(pcm_buffer)


@pytest.mark.parametrize(
    ("note", "message"),
    [
        ("H4", "Invalid note"),
        (object(), "note must be"),
    ],
)
def test_render_rejects_invalid_notes(note: object, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        render_note_to_sound_chain(note, duration_seconds=1.0)


def test_render_rejects_nyquist_violation() -> None:
    with pytest.raises(ValueError, match="below Nyquist"):
        render_note_to_sound_chain("A4", duration_seconds=1.0, sample_rate_hz=800.0)


def test_render_rejects_oversized_sample_budget() -> None:
    with pytest.raises(ValueError, match="above max_sample_count"):
        render_note_to_sound_chain("A4", duration_seconds=1.0, max_sample_count=10)


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"duration_seconds": -1.0}, "duration_seconds"),
        ({"duration_seconds": math.nan}, "duration_seconds"),
        ({"duration_seconds": MAX_DURATION_SECONDS + 1.0}, "MAX_DURATION_SECONDS"),
        ({"sample_rate_hz": 0.0}, "sample_rate_hz"),
        ({"sample_rate_hz": MAX_SAMPLE_RATE_HZ + 1.0}, "MAX_SAMPLE_RATE_HZ"),
        ({"amplitude": -0.1}, "amplitude"),
        ({"phase_cycles": math.inf}, "phase_cycles"),
        ({"full_scale_voltage": 0.0}, "full_scale_voltage"),
        ({"speaker_gain": math.nan}, "speaker_gain"),
        ({"max_sample_count": -1}, "max_sample_count"),
        ({"max_sample_count": MAX_SAMPLES + 1}, "MAX_SAMPLES"),
        ({"duration_seconds": True}, "duration_seconds"),
        ({"sample_rate_hz": True}, "sample_rate_hz"),
        ({"max_sample_count": True}, "max_sample_count"),
    ],
)
def test_render_rejects_invalid_numeric_inputs(
    kwargs: dict[str, object],
    message: str,
) -> None:
    render_kwargs: dict[str, object] = {"duration_seconds": 1.0}
    render_kwargs.update(kwargs)
    with pytest.raises(ValueError, match=message):
        render_note_to_sound_chain("A4", **render_kwargs)


def test_helper_validation_branches() -> None:
    from note_audio.note_audio import _finite_float, _non_negative_int

    with pytest.raises(ValueError, match="test must be finite"):
        _finite_float("test", float("nan"))
    with pytest.raises(ValueError, match="test must be >= 0"):
        _non_negative_int("test", -5)


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"sample_rate_hz": 0.0}, "sample_rate_hz"),
        ({"channel_count": 2}, "mono"),
        ({"bit_depth": 24}, "16-bit"),
        ({"full_scale_voltage": 0.0}, "full_scale_voltage"),
    ],
)
def test_pcm_format_rejects_unsupported_v1_shapes(
    kwargs: dict[str, object],
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        PCMFormat(**kwargs)


@pytest.mark.parametrize(
    ("samples", "message"),
    [
        ((True,), "PCM integer"),
        ((32768,), "signed 16-bit"),
        ((-32769,), "signed 16-bit"),
    ],
)
def test_pcm_buffer_rejects_invalid_integer_samples(
    samples: tuple[object, ...],
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        PCMBuffer(samples, PCMFormat())


def test_pcm_buffer_rejects_invalid_metadata_and_indexes() -> None:
    with pytest.raises(ValueError, match="pcm_format"):
        PCMBuffer((0,), object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="clipped_sample_count"):
        PCMBuffer((0,), PCMFormat(), clipped_sample_count=-1)

    buffer = PCMBuffer((0,), PCMFormat())
    with pytest.raises(ValueError, match="index"):
        buffer.time_at(1)
    with pytest.raises(ValueError, match="index"):
        buffer.time_at(True)  # type: ignore[arg-type]


def test_dac_and_encoder_reject_invalid_inputs() -> None:
    with pytest.raises(ValueError, match="sample"):
        float_to_pcm16(math.nan)
    with pytest.raises(ValueError, match="sample_buffer"):
        encode_sample_buffer(object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="pcm_buffer"):
        ZeroOrderHoldDACSignal(object())  # type: ignore[arg-type]

    dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat()))
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(math.inf)


def test_wav_output_rejects_non_integer_sample_rates_and_bad_buffers() -> None:
    pcm_buffer = PCMBuffer((0,), PCMFormat(sample_rate_hz=44_100.5))

    with pytest.raises(ValueError, match="integer-valued"):
        to_wav_bytes(pcm_buffer)
    with pytest.raises(ValueError, match="pcm_buffer"):
        to_wav_bytes(object())  # type: ignore[arg-type]


# =============================================================================
# § 2. Property-Based Testing (PBT) using SplitMix64 PRNG
# =============================================================================


def test_splitmix64_pbt_rendered_note_invariants() -> None:
    """PBT: Render notes across octaves and verify chain property invariants."""
    prng = SplitMix64(seed=0xBEEF_CAFE_0001)
    notes = ["C3", "D3", "E3", "F3", "G3", "A3", "B3", "C4", "E4", "G4", "A4", "C5"]

    for _ in range(20):
        note_name = notes[prng.next_u64() % len(notes)]
        dur = prng.uniform(0.005, 0.05)
        amp = prng.uniform(0.1, 0.9)
        gain = prng.uniform(0.5, 2.0)

        chain = render_note_to_sound_chain(
            note_name,
            duration_seconds=dur,
            amplitude=amp,
            speaker_gain=gain,
            sample_rate_hz=44100.0,
        )

        expected_count = int(dur * 44100.0)
        assert abs(chain.pcm_buffer.sample_count() - expected_count) <= 1
        assert chain.frequency_hz == parse_note(note_name).frequency()

        # Check all PCM samples are in valid range
        for sample in chain.pcm_buffer.samples:
            assert -32768 <= sample <= 32767
