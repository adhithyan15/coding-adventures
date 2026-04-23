"""Tests for WasmPackager."""

from __future__ import annotations

import pytest

from code_packager import CodeArtifact, Target, UnsupportedTargetError, WasmPackager

_WASM_MAGIC = b"\x00asm"
_WASM_VERSION = b"\x01\x00\x00\x00"


class TestWasmPackager:
    def setup_method(self):
        self.p = WasmPackager()

    def test_magic(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        assert result[:4] == _WASM_MAGIC

    def test_version(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        assert result[4:8] == _WASM_VERSION

    def test_returns_bytes(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        assert isinstance(result, bytes)

    def test_non_empty(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        assert len(result) > 8

    def test_code_present(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        # native_bytes should appear somewhere in the module (in the code section)
        assert wasm_artifact.native_bytes in result

    def test_custom_export_name(self):
        a = CodeArtifact(
            native_bytes=b"\x41\x00\x0b",  # i32.const 0; end
            entry_point=0,
            target=Target.wasm(),
            metadata={"exports": ["run"]},
        )
        result = self.p.pack(a)
        assert b"run" in result

    def test_default_export_main(self, wasm_artifact):
        result = self.p.pack(wasm_artifact)
        assert b"main" in result

    def test_file_extension(self):
        assert self.p.file_extension(Target.wasm()) == ".wasm"

    def test_wrong_target_raises(self, linux_artifact):
        with pytest.raises(UnsupportedTargetError):
            self.p.pack(linux_artifact)

    def test_empty_exports_uses_main(self):
        a = CodeArtifact(
            native_bytes=b"\x41\x00\x0b",
            entry_point=0,
            target=Target.wasm(),
            metadata={"exports": []},
        )
        result = self.p.pack(a)
        # With empty exports list, packager falls back to "main"
        assert b"main" in result
