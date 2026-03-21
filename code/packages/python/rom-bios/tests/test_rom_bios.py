"""Tests for ROM & BIOS firmware package."""

from __future__ import annotations

import struct

import pytest

from rom_bios.bios import AnnotatedInstruction, BIOSConfig, BIOSFirmware, DefaultBIOSConfig
from rom_bios.hardware_info import HARDWARE_INFO_SIZE, HardwareInfo
from rom_bios.rom import ROM, DefaultROMConfig, ROMConfig, DEFAULT_ROM_BASE, DEFAULT_ROM_SIZE


# ═══════════════════════════════════════════════════════════════
# ROM Tests
# ═══════════════════════════════════════════════════════════════


class TestROM:
    """Tests for the ROM (Read-Only Memory) class."""

    def test_new_rom_loads_firmware(self: TestROM) -> None:
        firmware = bytes([0xAA, 0xBB, 0xCC, 0xDD])
        rom = ROM(DefaultROMConfig(), firmware)
        assert rom.size() == DEFAULT_ROM_SIZE

    def test_new_rom_raises_on_oversized_firmware(self: TestROM) -> None:
        oversized = bytes(DEFAULT_ROM_SIZE + 1)
        with pytest.raises(ValueError, match="firmware larger than ROM size"):
            ROM(DefaultROMConfig(), oversized)

    def test_read_byte(self: TestROM) -> None:
        firmware = bytes([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0])
        rom = ROM(DefaultROMConfig(), firmware)
        base = DEFAULT_ROM_BASE

        assert rom.read(base) == 0x12
        assert rom.read(base + 1) == 0x34
        assert rom.read(base + 3) == 0x78
        assert rom.read(base + 7) == 0xF0

    def test_read_word(self: TestROM) -> None:
        firmware = bytes([0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A])
        rom = ROM(DefaultROMConfig(), firmware)
        base = DEFAULT_ROM_BASE

        assert rom.read_word(base) == 0x12345678
        assert rom.read_word(base + 4) == 0x9ABCDEF0

    def test_write_is_ignored(self: TestROM) -> None:
        firmware = bytes([0xAA, 0xBB, 0xCC, 0xDD])
        rom = ROM(DefaultROMConfig(), firmware)
        base = DEFAULT_ROM_BASE

        rom.write(base, 0xFF)
        assert rom.read(base) == 0xAA

    def test_out_of_range_returns_zero(self: TestROM) -> None:
        rom = ROM(DefaultROMConfig(), bytes([0xAA]))
        assert rom.read(0x00000000) == 0
        assert rom.read_word(0x00000000) == 0

    def test_firmware_smaller_than_rom(self: TestROM) -> None:
        rom = ROM(DefaultROMConfig(), bytes([0xAA, 0xBB]))
        base = DEFAULT_ROM_BASE
        assert rom.read(base + 2) == 0
        assert rom.read(base + 100) == 0

    def test_custom_config(self: TestROM) -> None:
        config = ROMConfig(base_address=0x10000000, size=256)
        rom = ROM(config, bytes([0x11, 0x22, 0x33, 0x44]))
        assert rom.size() == 256
        assert rom.base_address() == 0x10000000
        assert rom.read(0x10000000) == 0x11
        assert rom.read(DEFAULT_ROM_BASE) == 0

    def test_contains(self: TestROM) -> None:
        rom = ROM(DefaultROMConfig(), bytes([0xAA]))
        assert rom.contains(DEFAULT_ROM_BASE)
        assert rom.contains(DEFAULT_ROM_BASE + DEFAULT_ROM_SIZE - 1)
        assert not rom.contains(DEFAULT_ROM_BASE - 1)
        assert not rom.contains(0x00000000)

    def test_boundary_reads(self: TestROM) -> None:
        firmware = bytearray(DEFAULT_ROM_SIZE)
        firmware[0:4] = bytes([0x01, 0x02, 0x03, 0x04])
        firmware[-4:] = bytes([0xA1, 0xA2, 0xA3, 0xA4])
        rom = ROM(DefaultROMConfig(), bytes(firmware))
        base = DEFAULT_ROM_BASE

        assert rom.read_word(base) == 0x04030201
        last_word_addr = base + DEFAULT_ROM_SIZE - 4
        assert rom.read_word(last_word_addr) == 0xA4A3A2A1

    def test_empty_firmware(self: TestROM) -> None:
        rom = ROM(DefaultROMConfig())
        assert rom.read_word(DEFAULT_ROM_BASE) == 0


# ═══════════════════════════════════════════════════════════════
# HardwareInfo Tests
# ═══════════════════════════════════════════════════════════════


class TestHardwareInfo:
    """Tests for the HardwareInfo struct."""

    def test_defaults(self: TestHardwareInfo) -> None:
        info = HardwareInfo()
        assert info.memory_size == 0
        assert info.display_columns == 80
        assert info.display_rows == 25
        assert info.framebuffer_base == 0xFFFB0000
        assert info.idt_base == 0x00000000
        assert info.idt_entries == 256
        assert info.bootloader_entry == 0x00010000

    def test_to_bytes_roundtrip(self: TestHardwareInfo) -> None:
        info = HardwareInfo(
            memory_size=64 * 1024 * 1024,
            display_columns=80,
            display_rows=25,
            framebuffer_base=0xFFFB0000,
            idt_base=0x00000000,
            idt_entries=256,
            bootloader_entry=0x00010000,
        )
        data = info.to_bytes()
        assert len(data) == HARDWARE_INFO_SIZE
        restored = HardwareInfo.from_bytes(data)
        assert restored == info

    def test_to_bytes_layout(self: TestHardwareInfo) -> None:
        info = HardwareInfo(memory_size=0x04000000)
        data = info.to_bytes()
        assert data[0] == 0x00
        assert data[3] == 0x04
        assert data[4] == 80  # display_columns

    def test_from_bytes_raises_on_short_data(self: TestHardwareInfo) -> None:
        with pytest.raises(ValueError, match="data too short"):
            HardwareInfo.from_bytes(bytes([0x01, 0x02]))


# ═══════════════════════════════════════════════════════════════
# BIOS Firmware Generation Tests
# ═══════════════════════════════════════════════════════════════


class TestBIOSFirmware:
    """Tests for the BIOS firmware generator."""

    def test_generate_non_empty(self: TestBIOSFirmware) -> None:
        bios = BIOSFirmware(DefaultBIOSConfig())
        code = bios.generate()
        assert len(code) > 0

    def test_generate_word_aligned(self: TestBIOSFirmware) -> None:
        bios = BIOSFirmware(DefaultBIOSConfig())
        code = bios.generate()
        assert len(code) % 4 == 0

    def test_generate_deterministic(self: TestBIOSFirmware) -> None:
        config = DefaultBIOSConfig()
        code1 = BIOSFirmware(config).generate()
        code2 = BIOSFirmware(config).generate()
        assert code1 == code2

    def test_configurable_produces_different_output(self: TestBIOSFirmware) -> None:
        config1 = DefaultBIOSConfig()
        config2 = BIOSConfig(memory_size=128 * 1024 * 1024)
        code1 = BIOSFirmware(config1).generate()
        code2 = BIOSFirmware(config2).generate()
        assert code1 != code2

    def test_configured_memory_size_shorter(self: TestBIOSFirmware) -> None:
        probe_code = BIOSFirmware(DefaultBIOSConfig()).generate()
        fixed_code = BIOSFirmware(BIOSConfig(memory_size=64 * 1024 * 1024)).generate()
        assert len(fixed_code) < len(probe_code)

    def test_fits_in_rom(self: TestBIOSFirmware) -> None:
        code = BIOSFirmware(DefaultBIOSConfig()).generate()
        assert len(code) <= DEFAULT_ROM_SIZE

    def test_load_into_rom(self: TestBIOSFirmware) -> None:
        bios = BIOSFirmware(DefaultBIOSConfig())
        code = bios.generate()
        rom = ROM(DefaultROMConfig(), code)
        first_word = rom.read_word(DEFAULT_ROM_BASE)
        expected = struct.unpack_from("<I", code, 0)[0]
        assert first_word == expected


# ═══════════════════════════════════════════════════════════════
# Annotated Output Tests
# ═══════════════════════════════════════════════════════════════


class TestAnnotatedOutput:
    """Tests for GenerateWithComments output."""

    def test_matches_generate(self: TestAnnotatedOutput) -> None:
        bios = BIOSFirmware(DefaultBIOSConfig())
        code = bios.generate()
        annotated = bios.generate_with_comments()
        assert len(annotated) * 4 == len(code)
        for i, inst in enumerate(annotated):
            expected = struct.unpack_from("<I", code, i * 4)[0]
            assert inst.machine_code == expected, (
                f"instruction {i}: annotated 0x{inst.machine_code:08X} != "
                f"generate 0x{expected:08X}"
            )

    def test_address_continuity(self: TestAnnotatedOutput) -> None:
        annotated = BIOSFirmware(DefaultBIOSConfig()).generate_with_comments()
        assert len(annotated) > 0
        assert annotated[0].address == DEFAULT_ROM_BASE
        for i in range(1, len(annotated)):
            assert annotated[i].address == annotated[i - 1].address + 4

    def test_non_empty_strings(self: TestAnnotatedOutput) -> None:
        annotated = BIOSFirmware(DefaultBIOSConfig()).generate_with_comments()
        for i, inst in enumerate(annotated):
            assert inst.assembly, f"instruction {i} has empty assembly"
            assert inst.comment, f"instruction {i} has empty comment"

    def test_contains_riscv_mnemonics(self: TestAnnotatedOutput) -> None:
        annotated = BIOSFirmware(DefaultBIOSConfig()).generate_with_comments()
        mnemonics = {"lui", "addi", "sw", "jalr"}
        found = set()
        for inst in annotated:
            for m in mnemonics:
                if inst.assembly.startswith(m + " ") or inst.assembly == m:
                    found.add(m)
        assert found == mnemonics, f"missing mnemonics: {mnemonics - found}"

    def test_last_instruction_is_jump(self: TestAnnotatedOutput) -> None:
        annotated = BIOSFirmware(DefaultBIOSConfig()).generate_with_comments()
        assert annotated[-1].assembly.startswith("jalr")


# ═══════════════════════════════════════════════════════════════
# Default Config Tests
# ═══════════════════════════════════════════════════════════════


class TestDefaults:
    """Tests for default configurations."""

    def test_default_rom_config(self: TestDefaults) -> None:
        config = DefaultROMConfig()
        assert config.base_address == 0xFFFF0000
        assert config.size == 65536

    def test_default_bios_config(self: TestDefaults) -> None:
        config = DefaultBIOSConfig()
        assert config.memory_size == 0
        assert config.display_columns == 80
        assert config.display_rows == 25
        assert config.framebuffer_base == 0xFFFB0000
        assert config.bootloader_entry == 0x00010000
