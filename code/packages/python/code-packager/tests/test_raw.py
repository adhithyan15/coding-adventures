"""Tests for RawPackager."""

from __future__ import annotations

import pytest

from code_packager import CodeArtifact, RawPackager, Target, UnsupportedTargetError


class TestRawPackager:
    def setup_method(self):
        self.p = RawPackager()

    def test_passes_bytes_through(self, raw_artifact):
        result = self.p.pack(raw_artifact)
        assert result == raw_artifact.native_bytes

    def test_empty_bytes(self):
        a = CodeArtifact(native_bytes=b"", entry_point=0, target=Target.raw())
        assert self.p.pack(a) == b""

    def test_large_blob(self):
        code = bytes(range(256)) * 100
        a = CodeArtifact(native_bytes=code, entry_point=0, target=Target.raw(arch="x86_64"))
        assert self.p.pack(a) == code

    def test_file_extension(self):
        assert self.p.file_extension(Target.raw()) == ".bin"

    def test_wrong_format_raises(self):
        a = CodeArtifact(
            native_bytes=b"\x90",
            entry_point=0,
            target=Target.linux_x64(),  # elf64, not raw
        )
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(a)

    def test_all_raw_arch_variants(self):
        for arch in ["unknown", "i4004", "i8008", "x86_64", "arm64", "wasm32"]:
            a = CodeArtifact(native_bytes=b"\xaa", entry_point=0, target=Target.raw(arch=arch))
            assert self.p.pack(a) == b"\xaa"
