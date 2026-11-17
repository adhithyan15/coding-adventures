"""Tests for the reusable virtual DAC stage."""

from __future__ import annotations

import math

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from virtual_dac import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    ZeroOrderHoldDACSignal,
    __version__,
    pcm16_to_voltage,
)

# =============================================================================
# § 1. Pure Python SplitMix64 Deterministic PRNG for PBT / Fuzzing
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


def test_version_and_constants_exist() -> None:
    assert __version__ == "0.1.0"
    assert MAX_SAMPLE_RATE_HZ == 192_000.0
    assert MAX_DURATION_SECONDS == 60.0
    assert MAX_SAMPLES == 16_777_216


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


def test_zero_order_hold_float_rounding_at_boundary() -> None:
    pcm = PCMBuffer((100, 200), PCMFormat(sample_rate_hz=10.0))
    dac = ZeroOrderHoldDACSignal(pcm)
    near_end = math.nextafter(0.2, 0.0)
    val = dac.value_at(near_end)
    assert isinstance(val, float)
    assert dac.value_at(0.2) == 0.0


def test_rejects_invalid_inputs() -> None:
    with pytest.raises(ValueError, match="pcm_buffer"):
        ZeroOrderHoldDACSignal(object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="must fit signed 16-bit"):
        pcm16_to_voltage(32768)
    with pytest.raises(ValueError, match="must fit signed 16-bit"):
        pcm16_to_voltage(-32769)
    with pytest.raises(ValueError, match="sample must be a PCM integer"):
        pcm16_to_voltage(True)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="sample must be a PCM integer"):
        pcm16_to_voltage(1.5)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="pcm_format"):
        pcm16_to_voltage(0, object())  # type: ignore[arg-type]

    dac = ZeroOrderHoldDACSignal(PCMBuffer((0,), PCMFormat()))
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(math.inf)
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(math.nan)
    with pytest.raises(ValueError, match="time_seconds"):
        dac.value_at(True)  # type: ignore[arg-type]


def test_zero_order_hold_rejects_oversized_sample_count(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import virtual_dac.virtual_dac as dac_mod

    monkeypatch.setattr(dac_mod, "MAX_SAMPLES", 2)
    pcm = PCMBuffer((0, 10, 20), PCMFormat())
    with pytest.raises(ValueError, match="MAX_SAMPLES"):
        ZeroOrderHoldDACSignal(pcm)


def test_zero_order_hold_rejects_oversized_sample_rate(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import virtual_dac.virtual_dac as dac_mod

    monkeypatch.setattr(dac_mod, "MAX_SAMPLE_RATE_HZ", 100.0)
    pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=200.0))
    with pytest.raises(ValueError, match="MAX_SAMPLE_RATE_HZ"):
        ZeroOrderHoldDACSignal(pcm)


# =============================================================================
# § 2. Property-Based Testing (PBT) using SplitMix64 PRNG
# =============================================================================


def test_splitmix64_pbt_pcm16_to_voltage_bounds() -> None:
    """PBT: Output voltage is always within [-v_ref, v_ref]."""
    prng = SplitMix64(seed=0x1111_2222_3333_4444)

    for _ in range(100):
        # Sample random 16-bit integer
        sample = (prng.next_u64() % 65536) - 32768
        fsv = prng.uniform(0.1, 10.0)
        fmt = PCMFormat(full_scale_voltage=fsv)

        voltage = pcm16_to_voltage(sample, fmt)
        assert -fsv - 1e-9 <= voltage <= fsv + 1e-9


def test_splitmix64_pbt_zero_order_hold_invariants() -> None:
    """PBT: DAC output holds constant within every sample period."""
    prng = SplitMix64(seed=0x5555_6666_7777_8888)

    for _ in range(25):
        sample = (prng.next_u64() % 65536) - 32768
        rate = 1000.0
        pcm = PCMBuffer(
            (sample,), PCMFormat(sample_rate_hz=rate), start_time_seconds=0.0
        )
        dac = ZeroOrderHoldDACSignal(pcm)
        expected_v = pcm16_to_voltage(sample)

        for _ in range(10):
            t = prng.uniform(0.0, 0.00099)
            assert dac.value_at(t) == expected_v

        assert dac.value_at(0.001) == 0.0
        assert dac.value_at(-0.001) == 0.0
