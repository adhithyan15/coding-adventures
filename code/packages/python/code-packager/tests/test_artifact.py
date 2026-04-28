"""Tests for code_packager.artifact.CodeArtifact."""

from __future__ import annotations

from code_packager import CodeArtifact, Target


class TestCodeArtifact:
    def test_defaults(self):
        a = CodeArtifact(native_bytes=b"\x90", entry_point=0, target=Target.linux_x64())
        assert a.symbol_table == {}
        assert a.metadata == {}

    def test_with_symbol_table(self):
        st = {"main": 0, "add": 4}
        a = CodeArtifact(
            native_bytes=b"\x90" * 8,
            entry_point=0,
            target=Target.linux_x64(),
            symbol_table=st,
        )
        assert a.symbol_table["add"] == 4

    def test_with_metadata(self):
        a = CodeArtifact(
            native_bytes=b"\x90",
            entry_point=0,
            target=Target.windows_x64(),
            metadata={"subsystem": 2},
        )
        assert a.metadata["subsystem"] == 2

    def test_entry_point_offset(self):
        code = b"\x00" * 16 + b"\x90"
        a = CodeArtifact(native_bytes=code, entry_point=16, target=Target.linux_x64())
        assert a.native_bytes[a.entry_point] == 0x90

    def test_fields_preserved(self):
        code = b"\x48\x31\xc0\xc3"
        t = Target.linux_arm64()
        a = CodeArtifact(native_bytes=code, entry_point=0, target=t)
        assert a.native_bytes == code
        assert a.target is t
