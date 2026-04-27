"""Tests for aot_core.snapshot — .aot binary writer and reader."""

from __future__ import annotations

import pytest

from aot_core.snapshot import (
    FLAG_VM_RUNTIME,
    HEADER_SIZE,
    MAGIC,
    VERSION,
    AOTSnapshot,
    read,
    write,
)

# ---------------------------------------------------------------------------
# Basic write / read roundtrip
# ---------------------------------------------------------------------------

class TestWriteReadRoundtrip:
    def test_empty_code(self):
        raw = write(b"")
        snap = read(raw)
        assert snap.native_code == b""
        assert snap.iir_table is None

    def test_code_roundtrip(self):
        code = b"\xde\xad\xbe\xef"
        raw = write(code)
        snap = read(raw)
        assert snap.native_code == code

    def test_code_with_iir_table(self):
        code = b"\x01\x02\x03"
        iir = b"[{\"name\":\"f\"}]"
        raw = write(code, iir_table=iir)
        snap = read(raw)
        assert snap.native_code == code
        assert snap.iir_table == iir

    def test_entry_point_offset_preserved(self):
        code = b"\x00" * 16
        raw = write(code, entry_point_offset=8)
        snap = read(raw)
        assert snap.entry_point_offset == 8

    def test_version_correct(self):
        raw = write(b"")
        snap = read(raw)
        assert snap.version == VERSION

    def test_flags_no_iir(self):
        raw = write(b"")
        snap = read(raw)
        assert not snap.has_vm_runtime

    def test_flags_with_iir(self):
        raw = write(b"", iir_table=b"[]")
        snap = read(raw)
        assert snap.has_vm_runtime

    def test_flags_vm_runtime_bit_set(self):
        raw = write(b"", iir_table=b"[]")
        snap = read(raw)
        assert snap.flags & FLAG_VM_RUNTIME


# ---------------------------------------------------------------------------
# Header format details
# ---------------------------------------------------------------------------

class TestHeaderFormat:
    def test_magic_bytes(self):
        raw = write(b"")
        assert raw[:4] == MAGIC

    def test_total_length_no_iir(self):
        code = b"\xAA\xBB"
        raw = write(code)
        assert len(raw) == HEADER_SIZE + len(code)

    def test_total_length_with_iir(self):
        code = b"\x01\x02"
        iir = b"\x03\x04\x05"
        raw = write(code, iir_table=iir)
        assert len(raw) == HEADER_SIZE + len(code) + len(iir)

    def test_header_size_is_26(self):
        assert HEADER_SIZE == 26

    def test_iir_table_offset_zero_when_absent(self):
        raw = write(b"\x01\x02\x03")
        snap = read(raw)
        assert not snap.has_vm_runtime


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------

class TestReadErrors:
    def test_too_short_raises(self):
        with pytest.raises(ValueError, match="too short"):
            read(b"\x00" * 10)

    def test_bad_magic_raises(self):
        raw = write(b"")
        bad = b"BAD\x00" + raw[4:]
        with pytest.raises(ValueError, match="bad magic"):
            read(bad)

    def test_truncated_code_section_raises(self):
        code = b"\x01\x02\x03\x04"
        raw = write(code)
        # Chop 2 bytes from the code section.
        with pytest.raises(ValueError, match="truncated"):
            read(raw[:-2])

    def test_truncated_iir_table_raises(self):
        code = b"\xAA"
        iir = b"\xBB\xCC\xDD"
        raw = write(code, iir_table=iir)
        # Keep header + code but chop the IIR table.
        with pytest.raises(ValueError, match="truncated"):
            read(raw[:HEADER_SIZE + len(code) + 1])


# ---------------------------------------------------------------------------
# AOTSnapshot properties
# ---------------------------------------------------------------------------

class TestAOTSnapshotProperties:
    def test_has_vm_runtime_false(self):
        snap = AOTSnapshot(version=VERSION, flags=0, entry_point_offset=0,
                           native_code=b"", iir_table=None)
        assert not snap.has_vm_runtime

    def test_has_vm_runtime_true(self):
        snap = AOTSnapshot(version=VERSION, flags=FLAG_VM_RUNTIME, entry_point_offset=0,
                           native_code=b"", iir_table=b"[]")
        assert snap.has_vm_runtime

    def test_empty_iir_table_with_flag(self):
        raw = write(b"", iir_table=b"")
        snap = read(raw)
        # FLAG_VM_RUNTIME is set but iir_table has 0 bytes.
        assert snap.has_vm_runtime
        assert snap.iir_table == b""
