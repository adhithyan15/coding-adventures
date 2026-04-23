"""Tests for Elf64Packager.

All verification is done with struct.unpack — no external ELF parser needed.
"""

from __future__ import annotations

import struct

import pytest

from code_packager import CodeArtifact, Elf64Packager, Target, UnsupportedTargetError

_ELF_MAGIC = b"\x7fELF"
_ELF_HEADER_SIZE = 64
_PROG_HEADER_SIZE = 56
_DEFAULT_LOAD = 0x400000

# ELF header format (little-endian)
_EH_FMT = "<16sHHIQQQIHHHHHH"
# Program header format
_PH_FMT = "<IIQQQQQQ"


def _parse_elf(data: bytes):
    eh = struct.unpack_from(_EH_FMT, data, 0)
    ph = struct.unpack_from(_PH_FMT, data, _ELF_HEADER_SIZE)
    return eh, ph


class TestElf64Packager:
    def setup_method(self):
        self.p = Elf64Packager()

    def test_magic(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        assert result[:4] == _ELF_MAGIC

    def test_class_64bit(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        assert result[4] == 2  # ELFCLASS64

    def test_little_endian(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        assert result[5] == 1  # ELFDATA2LSB

    def test_type_exec(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        eh, _ = _parse_elf(result)
        assert eh[1] == 2  # ET_EXEC

    def test_machine_x86_64(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        eh, _ = _parse_elf(result)
        assert eh[2] == 62  # EM_X86_64

    def test_machine_arm64(self, small_code):
        a = CodeArtifact(native_bytes=small_code, entry_point=0, target=Target.linux_arm64())
        result = self.p.pack(a)
        eh, _ = _parse_elf(result)
        assert eh[2] == 183  # EM_AARCH64

    def test_entry_point_address(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        eh, _ = _parse_elf(result)
        expected = _DEFAULT_LOAD + _ELF_HEADER_SIZE + _PROG_HEADER_SIZE + linux_artifact.entry_point
        assert eh[4] == expected  # e_entry

    def test_entry_point_offset(self, small_code):
        a = CodeArtifact(
            native_bytes=b"\x00" * 8 + small_code,
            entry_point=8,
            target=Target.linux_x64(),
        )
        result = self.p.pack(a)
        eh, _ = _parse_elf(result)
        expected = _DEFAULT_LOAD + _ELF_HEADER_SIZE + _PROG_HEADER_SIZE + 8
        assert eh[4] == expected

    def test_phoff(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        eh, _ = _parse_elf(result)
        assert eh[5] == _ELF_HEADER_SIZE  # e_phoff

    def test_phnum(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        eh, _ = _parse_elf(result)
        assert eh[10] == 1  # e_phnum (index 10 in parsed tuple)

    def test_pt_load_type(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        _, ph = _parse_elf(result)
        assert ph[0] == 1  # PT_LOAD

    def test_pt_load_flags_rx(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        _, ph = _parse_elf(result)
        assert ph[1] == (4 | 1)  # PF_R | PF_X

    def test_code_embedded(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        code_offset = _ELF_HEADER_SIZE + _PROG_HEADER_SIZE
        assert result[code_offset:code_offset + len(linux_artifact.native_bytes)] == linux_artifact.native_bytes

    def test_custom_load_address(self, small_code):
        custom_addr = 0x800000
        a = CodeArtifact(
            native_bytes=small_code,
            entry_point=0,
            target=Target.linux_x64(),
            metadata={"load_address": custom_addr},
        )
        result = self.p.pack(a)
        _, ph = _parse_elf(result)
        assert ph[3] == custom_addr  # p_vaddr

    def test_file_extension(self):
        assert self.p.file_extension(Target.linux_x64()) == ".elf"

    def test_wrong_target_raises(self, windows_artifact):
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(windows_artifact)

    def test_total_size(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        expected = _ELF_HEADER_SIZE + _PROG_HEADER_SIZE + len(linux_artifact.native_bytes)
        assert len(result) == expected

    def test_filesz_matches(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        _, ph = _parse_elf(result)
        expected = _ELF_HEADER_SIZE + _PROG_HEADER_SIZE + len(linux_artifact.native_bytes)
        assert ph[5] == expected  # p_filesz

    def test_align(self, linux_artifact):
        result = self.p.pack(linux_artifact)
        _, ph = _parse_elf(result)
        assert ph[7] == 0x200000  # p_align
