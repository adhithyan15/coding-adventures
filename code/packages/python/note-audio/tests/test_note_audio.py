"""Tests for the visible note-to-sound signal chain."""

from __future__ import annotations

import math
import wave
from io import BytesIO
from pathlib import Path

import pytest
from note_frequency import parse_note
from oscillator import SampleBuffer

from note_audio import (
    DEFAULT_SAMPLE_RATE_HZ,
    LinearSpeakerSignal,
    NoteEvent,
    PCMBuffer,
    PCMFormat,
    ZeroOrderHoldDACSignal,
    __version__,
    encode_sample_buffer,
    float_to_pcm16,
    pcm16_to_voltage,
    render_note_to_sound_chain,
    samples_to_pcm_buffer,
    to_wav_bytes,
    write_wav,
)

ABS_TOLERANCE = 1e-9


def assert_close(got: float, want: float, tolerance: float = ABS_TOLERANCE) -> None:
    """Keep float assertions readable in the tutorial-style tests."""

    assert abs(got - want) <= tolerance


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


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


def test_pcm_parity_vector_for_one_hz_sampled_at_four_hz() -> None:
    sample_buffer = SampleBuffer(
        samples=(0.0, 1.0, 0.0, -1.0),
        sample_rate_hz=4.0,
    )
    pcm_buffer = encode_sample_buffer(sample_buffer, PCMFormat(sample_rate_hz=4.0))
    dac = ZeroOrderHoldDACSignal(pcm_buffer)
    speaker = LinearSpeakerSignal(dac)

    assert pcm_buffer.samples == (0, 32767, 0, -32768)
    assert dac.value_at(0.00) == 0.0
    assert dac.value_at(0.24) == 0.0
    assert dac.value_at(0.25) == 1.0
    assert dac.value_at(0.49) == 1.0
    assert dac.value_at(0.50) == 0.0
    assert dac.value_at(0.74) == 0.0
    assert dac.value_at(0.75) == -1.0
    assert dac.value_at(0.99) == -1.0
    assert dac.value_at(1.00) == 0.0
    assert speaker.value_at(0.75) == -1.0


def test_float_to_pcm16_clips_and_reports_clipping() -> None:
    assert float_to_pcm16(0.0) == (0, False)
    assert float_to_pcm16(1.0) == (32767, False)
    assert float_to_pcm16(-1.0) == (-32768, False)
    assert float_to_pcm16(2.0) == (32767, True)
    assert float_to_pcm16(-2.0) == (-32768, True)


def test_encode_sample_buffer_tracks_clipped_samples() -> None:
    buffer = SampleBuffer(samples=(2.0, -2.0, 0.5, -0.5), sample_rate_hz=8.0)
    pcm_buffer = encode_sample_buffer(buffer, PCMFormat(sample_rate_hz=8.0))

    assert pcm_buffer.samples == (32767, -32768, 16384, -16384)
    assert pcm_buffer.clipped_sample_count == 2


def test_samples_to_pcm_buffer_validates_and_encodes_raw_samples() -> None:
    pcm_buffer = samples_to_pcm_buffer((0.0, 1.0, -1.0), sample_rate_hz=3.0)

    assert pcm_buffer.samples == (0, 32767, -32768)
    assert pcm_buffer.sample_count() == 3
    assert_close(pcm_buffer.duration_seconds(), 1.0)


def test_pcm_buffer_metadata_and_bytes() -> None:
    pcm_buffer = PCMBuffer(
        samples=(0, 32767, -32768),
        pcm_format=PCMFormat(sample_rate_hz=3.0),
        start_time_seconds=5.0,
    )

    assert pcm_buffer.sample_count() == 3
    assert_close(pcm_buffer.sample_period_seconds(), 1.0 / 3.0)
    assert_close(pcm_buffer.duration_seconds(), 1.0)
    assert_close(pcm_buffer.time_at(2), 5.0 + 2.0 / 3.0)
    assert pcm_buffer.to_little_endian_bytes() == b"\x00\x00\xff\x7f\x00\x80"


def test_pcm16_to_voltage_uses_asymmetric_signed_range() -> None:
    pcm_format = PCMFormat(sample_rate_hz=4.0, full_scale_voltage=2.0)

    assert pcm16_to_voltage(0, pcm_format) == 0.0
    assert pcm16_to_voltage(32767, pcm_format) == 2.0
    assert pcm16_to_voltage(-32768, pcm_format) == -2.0


def test_linear_speaker_gain_scales_the_dac_signal() -> None:
    pcm_buffer = PCMBuffer((0, 32767), PCMFormat(sample_rate_hz=2.0))
    speaker = LinearSpeakerSignal(ZeroOrderHoldDACSignal(pcm_buffer), speaker_gain=0.5)

    assert speaker.value_at(0.0) == 0.0
    assert speaker.value_at(0.5) == 0.5


def test_wav_bytes_are_parseable_mono_pcm() -> None:
    pcm_buffer = PCMBuffer(
        samples=(0, 32767, -32768),
        pcm_format=PCMFormat(sample_rate_hz=8.0),
    )
    wav_bytes = to_wav_bytes(pcm_buffer)

    assert wav_bytes.startswith(b"RIFF")
    assert wav_bytes[8:12] == b"WAVE"
    with wave.open(BytesIO(wav_bytes), "rb") as wav_file:
        assert wav_file.getnchannels() == 1
        assert wav_file.getsampwidth() == 2
        assert wav_file.getframerate() == 8
        assert wav_file.getnframes() == 3
        assert wav_file.readframes(3) == b"\x00\x00\xff\x7f\x00\x80"


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
        ({"sample_rate_hz": 0.0}, "sample_rate_hz"),
        ({"amplitude": -0.1}, "amplitude"),
        ({"phase_cycles": math.inf}, "phase_cycles"),
        ({"full_scale_voltage": 0.0}, "full_scale_voltage"),
        ({"speaker_gain": math.nan}, "speaker_gain"),
        ({"max_sample_count": -1}, "max_sample_count"),
    ],
)
def test_render_rejects_invalid_numeric_inputs(
    kwargs: dict[str, object],
    message: str,
) -> None:
    render_kwargs = {"duration_seconds": 1.0}
    render_kwargs.update(kwargs)
    with pytest.raises(ValueError, match=message):
        render_note_to_sound_chain("A4", **render_kwargs)


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
