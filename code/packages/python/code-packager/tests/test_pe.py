"""Tests for PePackager.

Verifies PE32+ header fields with struct.unpack — no external parser needed.
"""

from __future__ import annotations

import struct

import pytest

from code_packager import CodeArtifact, PePackager, Target, UnsupportedTargetError

_DOS_MAGIC = b"MZ"
_PE_SIG = b"PE\x00\x00"
_PE32PLUS_MAGIC = 0x020B
_MACHINE_AMD64 = 0x8664
_SUBSYSTEM_CUI = 3
_SUBSYSTEM_GUI = 2
_FILE_ALIGN = 0x200
_SECT_ALIGN = 0x1000
_E_LFANEW_OFFSET = 60

# COFF header format (after PE signature, 20 bytes)
_COFF_FMT = "<HHIIIHH"
_COFF_SIZE = struct.calcsize(_COFF_FMT)  # 20

# Optional header (partial — first fields up through ImageBase)
# Magic(H) MajLinker(B) MinLinker(B) SizeOfCode(I) SizeOfInitData(I)
# SizeOfUninitData(I) AddressOfEntryPoint(I) BaseOfCode(I) ImageBase(Q)
_OPT_PARTIAL_FMT = "<HBBIIIIIQ"


def _parse_pe(data: bytes):
    assert data[:2] == _DOS_MAGIC
    e_lfanew = struct.unpack_from("<I", data, _E_LFANEW_OFFSET)[0]
    assert data[e_lfanew:e_lfanew + 4] == _PE_SIG
    pe_hdr_offset = e_lfanew + 4
    coff = struct.unpack_from(_COFF_FMT, data, pe_hdr_offset)
    opt_offset = pe_hdr_offset + _COFF_SIZE
    opt_partial = struct.unpack_from(_OPT_PARTIAL_FMT, data, opt_offset)
    return coff, opt_partial, opt_offset


class TestPePackager:
    def setup_method(self):
        self.p = PePackager()

    def test_mz_magic(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        assert result[:2] == _DOS_MAGIC

    def test_pe_signature(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        e_lfanew = struct.unpack_from("<I", result, _E_LFANEW_OFFSET)[0]
        assert result[e_lfanew:e_lfanew + 4] == _PE_SIG

    def test_machine_amd64(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        coff, _, _ = _parse_pe(result)
        assert coff[0] == _MACHINE_AMD64

    def test_one_section(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        coff, _, _ = _parse_pe(result)
        assert coff[1] == 1  # NumberOfSections

    def test_timestamp_zero(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        coff, _, _ = _parse_pe(result)
        assert coff[2] == 0  # reproducible build

    def test_pe32plus_magic(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        _, opt, _ = _parse_pe(result)
        assert opt[0] == _PE32PLUS_MAGIC

    def test_default_subsystem_cui(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        _, opt, opt_off = _parse_pe(result)
        # Subsystem is at offset 68 in the optional header
        subsystem = struct.unpack_from("<H", result, opt_off + 68)[0]
        assert subsystem == _SUBSYSTEM_CUI

    def test_gui_subsystem_metadata(self, small_code):
        a = CodeArtifact(
            native_bytes=small_code,
            entry_point=0,
            target=Target.windows_x64(),
            metadata={"subsystem": _SUBSYSTEM_GUI},
        )
        result = self.p.pack(a)
        _, _, opt_off = _parse_pe(result)
        subsystem = struct.unpack_from("<H", result, opt_off + 68)[0]
        assert subsystem == _SUBSYSTEM_GUI

    def test_entry_point_rva(self, small_code):
        a = CodeArtifact(native_bytes=small_code, entry_point=0, target=Target.windows_x64())
        result = self.p.pack(a)
        _, opt, _ = _parse_pe(result)
        # AddressOfEntryPoint (RVA) must be non-zero and section-aligned start
        aep = opt[6]
        assert aep >= _SECT_ALIGN

    def test_entry_point_offset_applied(self, small_code):
        padded = b"\x00" * 16 + small_code
        a = CodeArtifact(native_bytes=padded, entry_point=16, target=Target.windows_x64())
        no_offset = CodeArtifact(native_bytes=padded, entry_point=0, target=Target.windows_x64())
        r1 = self.p.pack(a)
        r2 = self.p.pack(no_offset)
        _, opt1, _ = _parse_pe(r1)
        _, opt2, _ = _parse_pe(r2)
        assert opt1[6] == opt2[6] + 16  # AEP differs by the entry_point offset

    def test_code_embedded(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        assert windows_artifact.native_bytes in result

    def test_custom_image_base(self, small_code):
        custom_base = 0x180000000
        a = CodeArtifact(
            native_bytes=small_code,
            entry_point=0,
            target=Target.windows_x64(),
            metadata={"image_base": custom_base},
        )
        result = self.p.pack(a)
        _, opt, _ = _parse_pe(result)
        assert opt[8] == custom_base  # ImageBase

    def test_headers_file_aligned(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        _, opt, opt_off = _parse_pe(result)
        size_of_headers = struct.unpack_from("<I", result, opt_off + 60)[0]
        assert size_of_headers % _FILE_ALIGN == 0

    def test_size_of_image_section_aligned(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        _, _, opt_off = _parse_pe(result)
        size_of_image = struct.unpack_from("<I", result, opt_off + 56)[0]
        assert size_of_image % _SECT_ALIGN == 0

    def test_file_extension(self):
        assert self.p.file_extension(Target.windows_x64()) == ".exe"

    def test_wrong_target_raises(self, linux_artifact):
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(linux_artifact)

    def test_section_name_text(self, windows_artifact):
        result = self.p.pack(windows_artifact)
        # Section table follows COFF + opt header
        e_lfanew = struct.unpack_from("<I", result, _E_LFANEW_OFFSET)[0]
        coff, _, _ = _parse_pe(result)
        section_offset = e_lfanew + 4 + _COFF_SIZE + coff[5]  # SizeOfOptionalHeader
        sect_name = result[section_offset:section_offset + 8].rstrip(b"\x00")
        assert sect_name == b".text"
