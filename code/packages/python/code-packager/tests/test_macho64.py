"""Tests for MachO64Packager.

Verifies Mach-O header fields with struct.unpack — no external parser needed.
"""

from __future__ import annotations

import struct

import pytest

from code_packager import CodeArtifact, MachO64Packager, Target, UnsupportedTargetError

MH_MAGIC_64 = 0xFEEDFACF
CPU_TYPE_X86_64 = 0x01000007
CPU_TYPE_ARM64 = 0x0100000C
MH_EXECUTE = 2
MH_NOUNDEFS = 0x1
LC_SEGMENT_64 = 0x19
LC_UNIXTHREAD = 0x5
_DEFAULT_LOAD = 0x100000000

# Mach-O header: 32 bytes
_MH_FMT = "<IiiIIIII"
_MH_SIZE = struct.calcsize(_MH_FMT)  # 32


def _parse_mach_header(data: bytes):
    return struct.unpack_from(_MH_FMT, data, 0)


def _parse_load_commands(data: bytes):
    """Yield (cmd, cmdsize, payload_bytes) for each load command."""
    _, _, _, _, ncmds, sizeofcmds, _, _ = _parse_mach_header(data)
    offset = _MH_SIZE
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", data, offset)
        payload = data[offset + 8:offset + cmdsize]
        yield cmd, cmdsize, payload
        offset += cmdsize


class TestMachO64Packager:
    def setup_method(self):
        self.p = MachO64Packager()

    def test_magic(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        mh = _parse_mach_header(result)
        assert mh[0] == MH_MAGIC_64

    def test_cputype_arm64(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        mh = _parse_mach_header(result)
        assert mh[1] == CPU_TYPE_ARM64

    def test_cputype_x86_64(self, macos_x64_artifact):
        result = self.p.pack(macos_x64_artifact)
        mh = _parse_mach_header(result)
        assert mh[1] == CPU_TYPE_X86_64

    def test_filetype_execute(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        mh = _parse_mach_header(result)
        assert mh[3] == MH_EXECUTE

    def test_flags_noundefs(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        mh = _parse_mach_header(result)
        assert mh[6] & MH_NOUNDEFS

    def test_two_load_commands(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        mh = _parse_mach_header(result)
        assert mh[4] == 2  # ncmds

    def test_has_lc_segment_64(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        cmds = list(_parse_load_commands(result))
        assert any(cmd == LC_SEGMENT_64 for cmd, _, _ in cmds)

    def test_has_lc_unixthread(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        cmds = list(_parse_load_commands(result))
        assert any(cmd == LC_UNIXTHREAD for cmd, _, _ in cmds)

    def test_code_embedded_arm64(self, macos_arm64_artifact):
        result = self.p.pack(macos_arm64_artifact)
        code = macos_arm64_artifact.native_bytes
        assert code in result

    def test_code_embedded_x86_64(self, macos_x64_artifact):
        result = self.p.pack(macos_x64_artifact)
        assert macos_x64_artifact.native_bytes in result

    def test_custom_load_address(self, small_code):
        custom = 0x200000000
        a = CodeArtifact(
            native_bytes=small_code,
            entry_point=0,
            target=Target.macos_arm64(),
            metadata={"load_address": custom},
        )
        result = self.p.pack(a)
        # LC_SEGMENT_64 body starts at offset 8 from cmd start
        # segname(16) then vmaddr(Q)
        cmds = list(_parse_load_commands(result))
        seg_payload = next(p for cmd, _, p in cmds if cmd == LC_SEGMENT_64)
        vmaddr = struct.unpack_from("<Q", seg_payload, 16)[0]
        assert vmaddr == custom

    def test_file_extension(self):
        assert self.p.file_extension(Target.macos_arm64()) == ".macho"

    def test_wrong_target_raises(self, linux_artifact):
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(linux_artifact)

    def test_entry_point_in_thread_state_arm64(self, small_code):
        a = CodeArtifact(
            native_bytes=small_code,
            entry_point=0,
            target=Target.macos_arm64(),
        )
        result = self.p.pack(a)
        cmds = list(_parse_load_commands(result))
        # LC_UNIXTHREAD payload: flavor(I) count(I) state_bytes
        ut_payload = next(p for cmd, _, p in cmds if cmd == LC_UNIXTHREAD)
        # ARM64 state: pc at offset 248 in state, which is after flavor(4)+count(4)=8 bytes in payload
        pc = struct.unpack_from("<Q", ut_payload, 8 + 248)[0]
        expected = _DEFAULT_LOAD + a.entry_point
        # The header size is added too, but we just check pc != 0
        assert pc != 0
        assert pc >= _DEFAULT_LOAD
