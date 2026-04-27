"""Tests for the reusable PCM encoding stage."""

from __future__ import annotations

import math

import pytest
from oscillator import SampleBuffer

from pcm_audio import (
    DEFAULT_BIT_DEPTH,
    DEFAULT_CHANNEL_COUNT,
    DEFAULT_FULL_SCALE_VOLTAGE,
    DEFAULT_SAMPLE_RATE_HZ,
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
    __version__,
    encode_sample_buffer,
    float_to_pcm16,
    samples_to_pcm_buffer,
)


def assert_close(got: float, want: float, tolerance: float = 1e-9) -> None:
    """Keep timing assertions readable."""

    assert abs(got - want) <= tolerance


def test_version_and_defaults_are_visible() -> None:
    assert __version__ == "0.1.0"
    assert DEFAULT_SAMPLE_RATE_HZ == 44_100.0
    assert DEFAULT_CHANNEL_COUNT == 1
    assert DEFAULT_BIT_DEPTH == 16
    assert DEFAULT_FULL_SCALE_VOLTAGE == 1.0
    assert PCM16_MIN == -32_768
    assert PCM16_MAX == 32_767


def test_pcm_format_validates_v1_shape() -> None:
    pcm_format = PCMFormat(sample_rate_hz=8.0, full_scale_voltage=2.0)

    assert pcm_format.minimum_integer == -32_768
    assert pcm_format.maximum_integer == 32_767
    assert pcm_format.sample_width_bytes == 2
    assert pcm_format.integer_sample_rate() == 8


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"sample_rate_hz": 0.0}, "sample_rate_hz"),
        ({"sample_rate_hz": math.inf}, "sample_rate_hz"),
        ({"channel_count": 2}, "mono"),
        ({"channel_count": True}, "channel_count"),
        ({"bit_depth": 24}, "16-bit"),
        ({"bit_depth": 0}, "bit_depth"),
        ({"full_scale_voltage": 0.0}, "full_scale_voltage"),
    ],
)
def test_pcm_format_rejects_invalid_shapes(
    kwargs: dict[str, object],
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        PCMFormat(**kwargs)


def test_integer_sample_rate_rejects_fractional_rates() -> None:
    with pytest.raises(ValueError, match="integer-valued"):
        PCMFormat(sample_rate_hz=44_100.5).integer_sample_rate()


def test_float_to_pcm16_clips_and_reports_clipping() -> None:
    assert float_to_pcm16(0.0) == (0, False)
    assert float_to_pcm16(1.0) == (32767, False)
    assert float_to_pcm16(-1.0) == (-32768, False)
    assert float_to_pcm16(2.0) == (32767, True)
    assert float_to_pcm16(-2.0) == (-32768, True)


@pytest.mark.parametrize("sample", [math.nan, math.inf, True, object()])
def test_float_to_pcm16_rejects_non_finite_or_non_real_samples(
    sample: object,
) -> None:
    with pytest.raises(ValueError, match="sample"):
        float_to_pcm16(sample)  # type: ignore[arg-type]


def test_encode_sample_buffer_tracks_clipping_and_timing() -> None:
    floating = SampleBuffer(
        samples=(0.0, 1.0, -1.0, 2.0),
        sample_rate_hz=4.0,
        start_time_seconds=10.0,
    )
    pcm = encode_sample_buffer(floating)

    assert pcm.samples == (0, 32767, -32768, 32767)
    assert pcm.clipped_sample_count == 1
    assert pcm.sample_count() == 4
    assert_close(pcm.sample_period_seconds(), 0.25)
    assert_close(pcm.duration_seconds(), 1.0)
    assert_close(pcm.time_at(2), 10.5)


def test_samples_to_pcm_buffer_encodes_raw_float_sequences() -> None:
    pcm = samples_to_pcm_buffer((0.0, 0.5, -0.5), sample_rate_hz=3.0)

    assert pcm.samples == (0, 16384, -16384)
    assert_close(pcm.duration_seconds(), 1.0)


def test_pcm_buffer_packs_little_endian_bytes() -> None:
    pcm = PCMBuffer((0, 32767, -32768), PCMFormat(sample_rate_hz=3.0))

    assert pcm.to_little_endian_bytes() == b"\x00\x00\xff\x7f\x00\x80"


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


def test_pcm_buffer_rejects_invalid_metadata() -> None:
    with pytest.raises(ValueError, match="pcm_format"):
        PCMBuffer((0,), object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="start_time_seconds"):
        PCMBuffer((0,), PCMFormat(), start_time_seconds=math.nan)
    with pytest.raises(ValueError, match="clipped_sample_count"):
        PCMBuffer((0,), PCMFormat(), clipped_sample_count=-1)
    with pytest.raises(ValueError, match="clipped_sample_count"):
        PCMBuffer((0,), PCMFormat(), clipped_sample_count=True)  # type: ignore[arg-type]


def test_pcm_buffer_rejects_invalid_indexes() -> None:
    pcm = PCMBuffer((0,), PCMFormat())

    with pytest.raises(ValueError, match="index"):
        pcm.time_at(1)
    with pytest.raises(ValueError, match="index"):
        pcm.time_at(True)  # type: ignore[arg-type]


def test_encode_sample_buffer_rejects_non_sample_buffer() -> None:
    with pytest.raises(ValueError, match="sample_buffer"):
        encode_sample_buffer(object())  # type: ignore[arg-type]
