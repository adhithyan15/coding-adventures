"""Tests for embed_debug_info() — platform dispatch."""

import pytest

from debug_sidecar.writer import DebugSidecarWriter
from native_debug_info.embed import embed_debug_info


def _sidecar() -> bytes:
    w = DebugSidecarWriter()
    fid = w.add_source_file("prog.tetrad")
    w.begin_function("main", start_instr=0, param_count=0)
    w.record("main", 0, file_id=fid, line=1, col=1)
    w.end_function("main", end_instr=4)
    return w.finish()


class _Artifact:
    def __init__(self, target, **kwargs):
        self.target = target
        self.load_address = kwargs.get("load_address", 0x400000)
        self.image_base = kwargs.get("image_base", 0x140000000)
        self.symbol_table = kwargs.get("symbol_table", {"main": 0})
        self.code_size = kwargs.get("code_size", 64)
        self.code_rva = kwargs.get("code_rva", 0x1000)


class TestEmbedDebugInfo:
    def test_linux_calls_elf_emitter(self, minimal_elf64):
        artifact = _Artifact("linux")
        result = embed_debug_info(minimal_elf64, artifact, _sidecar())
        assert result[:4] == b"\x7fELF"
        assert len(result) > len(minimal_elf64)

    def test_elf_target_alias(self, minimal_elf64):
        artifact = _Artifact("elf")
        result = embed_debug_info(minimal_elf64, artifact, _sidecar())
        assert result[:4] == b"\x7fELF"

    def test_macos_calls_macho_emitter(self, minimal_macho64):
        import struct
        artifact = _Artifact("macos")
        result = embed_debug_info(minimal_macho64, artifact, _sidecar())
        assert struct.unpack_from("<I", result, 0)[0] == 0xFEEDFACF
        assert len(result) > len(minimal_macho64)

    def test_darwin_target_alias(self, minimal_macho64):
        import struct
        artifact = _Artifact("darwin")
        result = embed_debug_info(minimal_macho64, artifact, _sidecar())
        assert struct.unpack_from("<I", result, 0)[0] == 0xFEEDFACF

    def test_windows_calls_pe_emitter(self, minimal_pe32plus):
        artifact = _Artifact("windows")
        result = embed_debug_info(minimal_pe32plus, artifact, _sidecar())
        assert result[:2] == b"MZ"
        assert len(result) > len(minimal_pe32plus)

    def test_pe_target_alias(self, minimal_pe32plus):
        artifact = _Artifact("pe")
        result = embed_debug_info(minimal_pe32plus, artifact, _sidecar())
        assert result[:2] == b"MZ"

    def test_unknown_target_raises(self, minimal_elf64):
        artifact = _Artifact("amiga")
        with pytest.raises(ValueError, match="unsupported target"):
            embed_debug_info(minimal_elf64, artifact, _sidecar())

    def test_empty_target_raises(self, minimal_elf64):
        artifact = _Artifact("")
        with pytest.raises(ValueError, match="unsupported target"):
            embed_debug_info(minimal_elf64, artifact, _sidecar())

    def test_case_insensitive_target(self, minimal_elf64):
        artifact = _Artifact("Linux")
        result = embed_debug_info(minimal_elf64, artifact, _sidecar())
        assert result[:4] == b"\x7fELF"
