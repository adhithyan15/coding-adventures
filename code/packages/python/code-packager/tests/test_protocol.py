"""Tests for PackagerProtocol and PackagerRegistry."""

from __future__ import annotations

import pytest

from code_packager import (
    CodeArtifact,
    Elf64Packager,
    PackagerRegistry,
    Target,
    UnsupportedTargetError,
)
from code_packager.protocol import PackagerProtocol


class TestPackagerProtocol:
    def test_elf_is_protocol(self):
        assert isinstance(Elf64Packager(), PackagerProtocol)

    def test_custom_packager_satisfies_protocol(self):
        class MyPackager:
            supported_targets = frozenset({Target.raw()})

            def pack(self, artifact: CodeArtifact) -> bytes:
                return artifact.native_bytes

            def file_extension(self, target: Target) -> str:
                return ".bin"

        assert isinstance(MyPackager(), PackagerProtocol)


class TestPackagerRegistry:
    def test_register_and_get(self):
        reg = PackagerRegistry()
        p = Elf64Packager()
        reg.register(p)
        assert reg.get(Target.linux_x64()) is p
        assert reg.get(Target.linux_arm64()) is p

    def test_get_missing_raises(self):
        reg = PackagerRegistry()
        with pytest.raises(UnsupportedTargetError) as exc_info:
            reg.get(Target.linux_x64())
        assert exc_info.value.target == Target.linux_x64()

    def test_pack_routes_correctly(self):
        reg = PackagerRegistry.default()
        artifact = CodeArtifact(
            native_bytes=b"\x48\x31\xc0\xc3",
            entry_point=0,
            target=Target.linux_x64(),
        )
        result = reg.pack(artifact)
        # Should be a valid ELF (magic check)
        assert result[:4] == b"\x7fELF"

    def test_pack_unsupported_raises(self):
        reg = PackagerRegistry()
        artifact = CodeArtifact(
            native_bytes=b"\x90",
            entry_point=0,
            target=Target.linux_x64(),
        )
        with pytest.raises(UnsupportedTargetError):
            reg.pack(artifact)

    def test_default_has_all_packagers(self):
        reg = PackagerRegistry.default()
        for target in [
            Target.linux_x64(),
            Target.linux_arm64(),
            Target.macos_x64(),
            Target.macos_arm64(),
            Target.windows_x64(),
            Target.wasm(),
            Target.raw(),
            Target.raw(arch="i4004"),
            Target.intel_4004(),
            Target.intel_8008(),
        ]:
            assert reg.get(target) is not None

    def test_register_overwrites(self):
        reg = PackagerRegistry()
        p1 = Elf64Packager()
        p2 = Elf64Packager()
        reg.register(p1)
        reg.register(p2)
        assert reg.get(Target.linux_x64()) is p2
