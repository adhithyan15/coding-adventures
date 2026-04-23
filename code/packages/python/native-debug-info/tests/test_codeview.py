"""Tests for CodeViewEmitter — .debug$S and .debug$T section builders."""

import struct

import pytest

from debug_sidecar.reader import DebugSidecarReader
from debug_sidecar.writer import DebugSidecarWriter
from native_debug_info.codeview import (
    CV_SIGNATURE,
    DEBUG_S_FILECHKSMS,
    DEBUG_S_LINES,
    DEBUG_S_SYMBOLS,
    CodeViewEmitter,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_emitter(reader: DebugSidecarReader, **kwargs) -> CodeViewEmitter:
    defaults = dict(
        image_base=0x140000000,
        symbol_table={"fibonacci": 0},
        code_rva=0x1000,
    )
    defaults.update(kwargs)
    return CodeViewEmitter(reader=reader, **defaults)


def _empty_reader() -> DebugSidecarReader:
    return DebugSidecarReader(DebugSidecarWriter().finish())


# ---------------------------------------------------------------------------
# .debug$S
# ---------------------------------------------------------------------------

class TestDebugS:
    def test_starts_with_cv_signature(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        sig = struct.unpack_from("<I", data, 0)[0]
        assert sig == CV_SIGNATURE

    def test_contains_symbols_subsection(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert DEBUG_S_SYMBOLS.to_bytes(4, "little") in data

    def test_contains_lines_subsection(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert DEBUG_S_LINES.to_bytes(4, "little") in data

    def test_contains_filechksms_subsection(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert DEBUG_S_FILECHKSMS.to_bytes(4, "little") in data

    def test_function_name_in_output(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert b"fibonacci" in data

    def test_source_file_path_in_output(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert b"fibonacci.tetrad" in data

    def test_returns_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        assert isinstance(emitter._build_debug_s(), bytes)

    def test_empty_reader_produces_signature(self):
        emitter = _make_emitter(_empty_reader())
        data = emitter._build_debug_s()
        assert struct.unpack_from("<I", data, 0)[0] == CV_SIGNATURE

    def test_4byte_aligned_size(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_s()
        assert len(data) % 4 == 0


# ---------------------------------------------------------------------------
# .debug$T
# ---------------------------------------------------------------------------

class TestDebugT:
    def test_starts_with_cv_signature(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_t()
        sig = struct.unpack_from("<I", data, 0)[0]
        assert sig == CV_SIGNATURE

    def test_returns_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        assert isinstance(emitter._build_debug_t(), bytes)

    def test_non_empty(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        assert len(emitter._build_debug_t()) > 4

    def test_lf_procedure_present(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        data = emitter._build_debug_t()
        # LF_PROCEDURE = 0x1008 → bytes \x08\x10 in little-endian
        assert b"\x08\x10" in data


# ---------------------------------------------------------------------------
# build()
# ---------------------------------------------------------------------------

class TestBuild:
    def test_returns_both_sections(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        cv = emitter.build()
        assert set(cv.keys()) == {".debug$S", ".debug$T"}

    def test_all_sections_are_bytes(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        for name, data in emitter.build().items():
            assert isinstance(data, bytes), f"{name} is not bytes"

    def test_signatures_correct(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        cv = emitter.build()
        for key in (".debug$S", ".debug$T"):
            sig = struct.unpack_from("<I", cv[key], 0)[0]
            assert sig == CV_SIGNATURE, f"{key} has wrong signature"


# ---------------------------------------------------------------------------
# embed_in_pe()
# ---------------------------------------------------------------------------

class TestEmbedInPe:
    def test_invalid_magic_raises(self, fibonacci_reader):
        emitter = _make_emitter(fibonacci_reader)
        with pytest.raises(ValueError, match="PE"):
            emitter.embed_in_pe(b"\x00" * 512)

    def test_pe32_raises(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        # Corrupt the optional header magic to PE32 (0x010B)
        bad = bytearray(minimal_pe32plus)
        opt_off = 64 + 4 + 20  # dos(64) + pe_sig(4) + coff(20)
        struct.pack_into("<H", bad, opt_off, 0x010B)
        with pytest.raises(ValueError, match="PE32\\+"):
            emitter.embed_in_pe(bytes(bad))

    def test_result_is_bytes(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_pe(minimal_pe32plus)
        assert isinstance(result, bytes)

    def test_mz_magic_preserved(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_pe(minimal_pe32plus)
        assert result[:2] == b"MZ"

    def test_pe_signature_preserved(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_pe(minimal_pe32plus)
        assert result[64:68] == b"PE\x00\x00"

    def test_section_count_increases_by_2(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        coff_off = 64 + 4
        original_count = struct.unpack_from("<H", minimal_pe32plus, coff_off + 2)[0]
        result = emitter.embed_in_pe(minimal_pe32plus)
        new_count = struct.unpack_from("<H", result, coff_off + 2)[0]
        assert new_count == original_count + 2

    def test_debug_section_names_present(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_pe(minimal_pe32plus)
        assert b".debug$S" in result
        assert b".debug$T" in result

    def test_result_larger_than_input(self, fibonacci_reader, minimal_pe32plus):
        emitter = _make_emitter(fibonacci_reader)
        result = emitter.embed_in_pe(minimal_pe32plus)
        assert len(result) > len(minimal_pe32plus)

    def test_insufficient_header_space_raises(self, fibonacci_reader):
        """PE with no room for extra section headers raises ValueError."""
        emitter = _make_emitter(fibonacci_reader)
        # Build a PE where SizeOfHeaders leaves no room for new section headers
        pe = bytearray(minimal_pe_no_space())
        with pytest.raises(ValueError, match="insufficient"):
            emitter.embed_in_pe(bytes(pe))


def minimal_pe_no_space() -> bytes:
    """A PE32+ where the section table exactly fills the header area (no extra space)."""
    e_lfanew = 64
    dos = b"MZ" + b"\x00" * 58 + struct.pack("<I", e_lfanew)
    coff = struct.pack("<HHIIIHH", 0x8664, 0, 0, 0, 0, 240, 0x0022)
    opt = bytearray(240)
    struct.pack_into("<H", opt, 0, 0x020B)
    file_alignment = 512
    section_alignment = 4096
    # SizeOfHeaders = exactly the header area (no padding for extra sections)
    size_of_headers = len(dos) + 4 + len(coff) + 240  # = 64 + 4 + 20 + 240 = 328
    # Round up to file_alignment (512) — but only by the exact amount needed
    # to leave 0 bytes for extra section headers
    size_of_headers = 328  # deliberately NOT rounded up; no extra section space
    struct.pack_into("<I", opt, 32, section_alignment)
    struct.pack_into("<I", opt, 36, file_alignment)
    struct.pack_into("<I", opt, 56, 4096)
    struct.pack_into("<I", opt, 60, size_of_headers)
    raw = dos + b"PE\x00\x00" + coff + bytes(opt)
    return raw  # 328 bytes, leaving 0 bytes for section headers
