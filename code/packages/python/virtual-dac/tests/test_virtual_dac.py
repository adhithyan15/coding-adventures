"""Tests for the reusable virtual DAC stage."""

from __future__ import annotations

import math

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from virtual_dac import ZeroOrderHoldDACSignal, __version__, pcm16_to_voltage


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_pcm16_to_voltage_uses_asymmetric_signed_range() -> None:
    pcm_format = PCMFormat(sample_rate_hz=4.0, full_scale_voltage=2.0)

    assert pcm16_to_voltage(0, pcm_format) == 0.0
    assert pcm16_to_voltage(32767, pcm_format) == 2.0
    assert pcm16_to_voltage(-32768, pcm_format) == -2.0


def test_pcm16_to_voltage_uses_default_format() -> None:
    assert pcm16_to_voltage(32767) == 1.0


def test_zero_order_hold_parity_vector() -> None:
    pcm = PCMBuffer((0, 32767, 0, -32768), PCMFormat(sample_rate_hz=4.0))
    dac = ZeroOrderHoldDACSignal(pcm)

    assert dac.value_at(0.00) == 0.0
    assert dac.value_at(0.24) == 0.0
    assert dac.value_at(0.25) == 1.0
    assert dac.value_at(0.49) == 1.0
    assert dac.value_at(0.50) == 0.0
    assert dac.value_at(0.74) == 0.0
    assert dac.value_at(0.75) == -1.0
    assert dac.value_at(0.99) == -1.0
    assert dac.value_at(1.00) == 0.0


def test_zero_order_hold_respects_start_time() -> None:
    pcm = PCMBuffer(
        (32767,),
        PCMFormat(sample_rate_hz=2.0),
        start_time_seconds=10.0,
    )
    dac = ZeroOrderHoldDACSignal(pcm)

    assert dac.value_at(9.999) == 0.0
    assert dac.value_at(10.0) == 1.0
    assert dac.value_at(10.49) == 1.0
    assert dac.value_at(10.5) == 0.0


def test_zero_order_hold_returns_silence_for_empty_buffers() -> None:
    dac = ZeroOrderHoldDACSignal(PCMBuffer((), PCMFormat(sample_rate_hz=4.0)))

    assert dac.value_at(0.0) == 0.0


def test_rejects_invalid_inputs() -> None:
    with pytest.raises(ValueError, match="pcm_buffer"):
        ZeroOrderHoldDACSignal(object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="signed 16-bit"):
        pcm16_to_voltage(32768)

    dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat()))
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(math.inf)
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(True)  # type: ignore[arg-type]
