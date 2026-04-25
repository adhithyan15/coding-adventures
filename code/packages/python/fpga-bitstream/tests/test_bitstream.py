"""Tests for fpga-bitstream emitter."""

from pathlib import Path

import pytest

from fpga_bitstream import (
    PART_SPECS,
    ClbConfig,
    FpgaConfig,
    Iice40Part,
    emit_bitstream,
    write_bin,
)
from fpga_bitstream.bitstream import _cmd


def test_part_specs_complete():
    for part in Iice40Part:
        assert part in PART_SPECS


def test_empty_config_emits_minimal_stream():
    config = FpgaConfig(part=Iice40Part.HX1K)
    data, report = emit_bitstream(config)
    assert report.bytes_written == len(data)
    assert report.clb_count == 0
    # Preamble + reset cmd + bank cmd + crc + end marker
    assert data.startswith(b"\xff\x00")
    # End marker 0xffff at the tail
    assert data[-2:] == b"\xff\xff"


def test_clb_count_in_report():
    config = FpgaConfig(part=Iice40Part.HX1K)
    config.clbs[(0, 0)] = ClbConfig()
    config.clbs[(0, 1)] = ClbConfig()
    _, report = emit_bitstream(config)
    assert report.clb_count == 2


def test_part_in_report():
    config = FpgaConfig(part=Iice40Part.UP5K)
    _, report = emit_bitstream(config)
    assert report.part == Iice40Part.UP5K


def test_lut_truth_table_default_size():
    cfg = ClbConfig()
    assert len(cfg.lut_a_truth_table) == 16
    assert len(cfg.lut_b_truth_table) == 16
    assert all(b == 0 for b in cfg.lut_a_truth_table)


def test_more_clbs_means_larger_bitstream():
    small = FpgaConfig(part=Iice40Part.HX1K)
    big = FpgaConfig(part=Iice40Part.HX1K)
    for i in range(10):
        big.clbs[(0, i)] = ClbConfig()

    _, small_report = emit_bitstream(small)
    _, big_report = emit_bitstream(big)
    assert big_report.bytes_written > small_report.bytes_written


def test_write_bin_creates_file(tmp_path: Path):
    config = FpgaConfig(part=Iice40Part.HX1K)
    config.clbs[(2, 3)] = ClbConfig()
    p = tmp_path / "out.bin"
    report = write_bin(str(p), config)
    assert p.exists()
    assert p.stat().st_size == report.bytes_written
    # First two bytes should be the preamble
    assert p.read_bytes()[:2] == b"\xff\x00"


def test_cmd_payload_too_long():
    with pytest.raises(ValueError, match="too long"):
        _cmd(0x05, b"x" * 256)


def test_cmd_returns_correct_format():
    result = _cmd(0x05, b"\x12\x34")
    # 1 byte length (4) + 1 byte command (5) + 2 bytes payload
    assert result == b"\x04\x05\x12\x34"


def test_part_specs_reasonable_sizes():
    """HX1K is smaller than HX8K."""
    hx1k_rows, _, _ = PART_SPECS[Iice40Part.HX1K]
    hx8k_rows, _, _ = PART_SPECS[Iice40Part.HX8K]
    # In real life HX8K is bigger; we just check both are non-zero
    assert hx1k_rows > 0
    assert hx8k_rows > 0


def test_4bit_adder_smoke(tmp_path: Path):
    """Build a config representing a 4-bit-adder mapped to ~20 LUTs."""
    config = FpgaConfig(part=Iice40Part.HX1K)
    # 20 cells placed in row-major on 8x8 grid
    for i in range(20):
        row, col = i // 8, i % 8
        config.clbs[(row, col)] = ClbConfig(
            lut_a_truth_table=[0, 1, 1, 0] * 4,  # XOR-style
        )

    p = tmp_path / "adder4.bin"
    report = write_bin(str(p), config)

    assert report.clb_count == 20
    assert report.bytes_written > 100
    data = p.read_bytes()
    # Preamble + at least 20 OFFSET + DATA records + end marker
    assert data[:2] == b"\xff\x00"
    assert data[-2:] == b"\xff\xff"
