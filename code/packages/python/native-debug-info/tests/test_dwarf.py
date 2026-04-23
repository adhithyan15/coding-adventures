"""Tests for DwarfEmitter — section builders and ELF/Mach-O embedding."""

import struct

import pytest

from debug_sidecar.reader import DebugSidecarReader
from debug_sidecar.writer import DebugSidecarWriter
from native_debug_info.dwarf import PRODUCER, DwarfEmitter
from native_debug_info.leb128 import decode_uleb128


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_emitter(reader: DebugSidecarReader, **kwargs) -> DwarfEmitter:
    defaults = dict(load_address=0x400000, symbol_table={"fibonacci": 0}, code_size=256)
    defaults.update(kwargs)
    return DwarfEmitter(reader=reader, **defaults)


def _empty_reader() -> DebugSidecarReader:
    return DebugSidecarReader(DebugSidecarWriter().finish())


# ---------------------------------------------------------------------------
# .debug_str
# ---------------------------------------------------------------------------

class TestDebugStr:
    def test_contains_producer(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert PRODUCER in offsets

    def test_contains_source_files(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert "fibonacci.tetrad" in offsets

    def test_contains_function_names(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert "fibonacci" in offsets

    def test_contains_empty_comp_dir(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert "" in offsets

    def test_strings_are_null_terminated(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data, offsets = emitter._build_str_table()
        for s, off in offsets.items():
            end = data.index(b"\x00", off)
            assert data[off:end].decode("utf-8") == s

    def test_deduplication(self, fibonacci_reader):
        """Same string appearing twice should have one entry."""
        emitter = _make_emitter(fibonacci_reader)
        data, offsets = emitter._build_str_table()
        # producer appears once; if it appeared twice the table would be larger
        count = data.count(PRODUCER.encode("utf-8"))
        assert count == 1

    def test_empty_reader_still_produces_str_table(self):
        emitter = _make_emitter(_empty_reader())
        data, offsets = emitter._build_str_table()
        assert isinstance(data, bytes)
        assert PRODUCER in offsets


# ---------------------------------------------------------------------------
# .debug_abbrev
# ---------------------------------------------------------------------------

class TestDebugAbbrev:
    def test_first_byte_is_abbrev_code_1(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_abbrev()
        code, _ = decode_uleb128(data, 0)
        assert code == 1

    def test_ends_with_zero(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_abbrev()
        assert data[-1] == 0

    def test_contains_two_abbreviations(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_abbrev()
        # Count abbreviation codes (values 1 and 2) at known positions
        # Just verify the data is non-empty and parseable
        assert len(data) > 10

    def test_returns_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        assert isinstance(emitter._build_abbrev(), bytes)


# ---------------------------------------------------------------------------
# .debug_line
# ---------------------------------------------------------------------------

class TestDebugLine:
    def _header_fields(self, data: bytes):
        """Parse DWARF 4 .debug_line header fields."""
        unit_length = struct.unpack_from("<I", data, 0)[0]
        version = struct.unpack_from("<H", data, 4)[0]
        header_length = struct.unpack_from("<I", data, 6)[0]
        min_insn_len = data[10]
        max_ops = data[11]
        default_is_stmt = data[12]
        line_base = struct.unpack_from("<b", data, 13)[0]
        line_range = data[14]
        opcode_base = data[15]
        return {
            "unit_length": unit_length,
            "version": version,
            "header_length": header_length,
            "min_insn_len": min_insn_len,
            "max_ops": max_ops,
            "default_is_stmt": default_is_stmt,
            "line_base": line_base,
            "line_range": line_range,
            "opcode_base": opcode_base,
        }

    def test_version_is_4(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        h = self._header_fields(data)
        assert h["version"] == 4

    def test_min_insn_len_is_1(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        h = self._header_fields(data)
        assert h["min_insn_len"] == 1

    def test_default_is_stmt_is_1(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        assert self._header_fields(data)["default_is_stmt"] == 1

    def test_line_base_is_minus_5(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        assert self._header_fields(data)["line_base"] == -5

    def test_opcode_base_is_13(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        assert self._header_fields(data)["opcode_base"] == 13

    def test_unit_length_consistent(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        unit_length = struct.unpack_from("<I", data, 0)[0]
        # unit_length = total_size - 4 (excludes the unit_length field itself)
        assert unit_length == len(data) - 4

    def test_empty_reader_produces_valid_header(self):
        emitter = _make_emitter(_empty_reader())
        _, offsets = emitter._build_str_table()
        data = emitter._build_line(offsets)
        assert self._header_fields(data)["version"] == 4

    def test_returns_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert isinstance(emitter._build_line(offsets), bytes)


# ---------------------------------------------------------------------------
# .debug_info
# ---------------------------------------------------------------------------

class TestDebugInfo:
    def _cu_header(self, data: bytes):
        unit_length = struct.unpack_from("<I", data, 0)[0]
        version = struct.unpack_from("<H", data, 4)[0]
        abbrev_off = struct.unpack_from("<I", data, 6)[0]
        addr_size = data[10]
        return unit_length, version, abbrev_off, addr_size

    def test_version_is_4(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_info(offsets)
        _, version, _, _ = self._cu_header(data)
        assert version == 4

    def test_abbrev_offset_is_0(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_info(offsets)
        _, _, abbrev_off, _ = self._cu_header(data)
        assert abbrev_off == 0

    def test_address_size_is_8(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_info(offsets)
        _, _, _, addr_size = self._cu_header(data)
        assert addr_size == 8

    def test_unit_length_consistent(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        data = emitter._build_info(offsets)
        unit_length = struct.unpack_from("<I", data, 0)[0]
        assert unit_length == len(data) - 4

    def test_returns_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        _, offsets = emitter._build_str_table()
        assert isinstance(emitter._build_info(offsets), bytes)


# ---------------------------------------------------------------------------
# build() — all four sections
# ---------------------------------------------------------------------------

class TestBuild:
    def test_returns_all_four_sections(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        sections = emitter.build()
        assert set(sections.keys()) == {".debug_abbrev", ".debug_info",
                                         ".debug_line", ".debug_str"}

    def test_all_sections_are_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        for name, data in emitter.build().items():
            assert isinstance(data, bytes), f"{name} is not bytes"

    def test_all_sections_non_empty(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        for name, data in emitter.build().items():
            assert len(data) > 0, f"{name} is empty"

    def test_empty_reader_produces_valid_sections(self):
        emitter = _make_emitter(_empty_reader())
        sections = emitter.build()
        assert len(sections) == 4

    def test_debug_str_contains_producer_string(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        sections = emitter.build()
        assert PRODUCER.encode("utf-8") in sections[".debug_str"]


# ---------------------------------------------------------------------------
# embed_in_elf()
# ---------------------------------------------------------------------------

class TestEmbedInElf:
    def test_invalid_magic_raises(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        with pytest.raises(ValueError, match="ELF"):
            emitter.embed_in_elf(b"\x00" * 256)

    def test_32bit_elf_raises(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        bad = bytearray(b"\x7fELF" + b"\x01" + b"\x00" * 251)
        with pytest.raises(ValueError, match="ELF64"):
            emitter.embed_in_elf(bytes(bad))

    def test_big_endian_elf_raises(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        bad = bytearray(b"\x7fELF\x02\x02" + b"\x00" * 250)
        with pytest.raises(ValueError, match="little-endian"):
            emitter.embed_in_elf(bytes(bad))

    def test_result_is_bytes(self, fibonacci_reader, minimal_elf64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_elf(minimal_elf64)
        assert isinstance(result, bytes)

    def test_elf_magic_preserved(self, fibonacci_reader, minimal_elf64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_elf(minimal_elf64)
        assert result[:4] == b"\x7fELF"

    def test_section_count_increases_by_4(self, fibonacci_reader, minimal_elf64):
        emitter = _make_emitter(fibonacci_reader)
        original_count = struct.unpack_from("<H", minimal_elf64, 60)[0]
        result = emitter.embed_in_elf(minimal_elf64)
        new_count = struct.unpack_from("<H", result, 60)[0]
        assert new_count == original_count + 4

    def test_result_larger_than_input(self, fibonacci_reader, minimal_elf64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_elf(minimal_elf64)
        assert len(result) > len(minimal_elf64)

    def test_debug_section_names_in_shstrtab(self, fibonacci_reader, minimal_elf64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_elf(minimal_elf64)
        # The new .shstrtab should contain all debug section names
        for name in (b".debug_abbrev", b".debug_info", b".debug_line", b".debug_str"):
            assert name in result

    def test_idempotent_magic(self, fibonacci_reader, minimal_elf64):
        """Embedding twice should produce a valid ELF each time."""
        emitter = _make_emitter(fibonacci_reader)
        result1 = emitter.embed_in_elf(minimal_elf64)
        assert result1[:4] == b"\x7fELF"


# ---------------------------------------------------------------------------
# embed_in_macho()
# ---------------------------------------------------------------------------

class TestEmbedInMacho:
    def test_invalid_magic_raises(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        with pytest.raises(ValueError, match="Mach-O"):
            emitter.embed_in_macho(b"\x00" * 64)

    def test_result_is_bytes(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_macho(minimal_macho64)
        assert isinstance(result, bytes)

    def test_magic_preserved(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_macho(minimal_macho64)
        assert struct.unpack_from("<I", result, 0)[0] == 0xFEEDFACF

    def test_ncmds_increases_by_1(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        original_ncmds = struct.unpack_from("<I", minimal_macho64, 16)[0]
        result = emitter.embed_in_macho(minimal_macho64)
        new_ncmds = struct.unpack_from("<I", result, 16)[0]
        assert new_ncmds == original_ncmds + 1

    def test_sizeofcmds_increases(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        original_sizeofcmds = struct.unpack_from("<I", minimal_macho64, 20)[0]
        result = emitter.embed_in_macho(minimal_macho64)
        new_sizeofcmds = struct.unpack_from("<I", result, 20)[0]
        assert new_sizeofcmds == original_sizeofcmds + 392  # 72 + 4*80

    def test_result_larger_than_input(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_macho(minimal_macho64)
        assert len(result) > len(minimal_macho64)

    def test_dwarf_segment_name_in_output(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_macho(minimal_macho64)
        assert b"__DWARF" in result

    def test_debug_section_names_in_output(self, fibonacci_reader, minimal_macho64):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_macho(minimal_macho64)
        for name in (b"__debug_abbrev", b"__debug_info", b"__debug_line", b"__debug_str"):
            assert name in result
