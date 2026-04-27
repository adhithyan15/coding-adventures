"""Tests for code_packager.target.Target."""

from __future__ import annotations

import pytest

from code_packager import Target


class TestTargetFactories:
    def test_linux_x64(self):
        t = Target.linux_x64()
        assert t.arch == "x86_64"
        assert t.os == "linux"
        assert t.binary_format == "elf64"

    def test_linux_arm64(self):
        t = Target.linux_arm64()
        assert t.arch == "arm64"
        assert t.os == "linux"
        assert t.binary_format == "elf64"

    def test_macos_x64(self):
        t = Target.macos_x64()
        assert t.arch == "x86_64"
        assert t.os == "macos"
        assert t.binary_format == "macho64"

    def test_macos_arm64(self):
        t = Target.macos_arm64()
        assert t.arch == "arm64"
        assert t.os == "macos"
        assert t.binary_format == "macho64"

    def test_windows_x64(self):
        t = Target.windows_x64()
        assert t.arch == "x86_64"
        assert t.os == "windows"
        assert t.binary_format == "pe"

    def test_wasm(self):
        t = Target.wasm()
        assert t.arch == "wasm32"
        assert t.os == "none"
        assert t.binary_format == "wasm"

    def test_raw_default(self):
        t = Target.raw()
        assert t.arch == "unknown"
        assert t.os == "none"
        assert t.binary_format == "raw"

    def test_raw_with_arch(self):
        t = Target.raw(arch="i4004")
        assert t.arch == "i4004"
        assert t.binary_format == "raw"

    def test_intel_4004(self):
        t = Target.intel_4004()
        assert t.arch == "i4004"
        assert t.binary_format == "intel_hex"

    def test_intel_8008(self):
        t = Target.intel_8008()
        assert t.arch == "i8008"
        assert t.binary_format == "intel_hex"


class TestTargetEquality:
    def test_equal(self):
        assert Target.linux_x64() == Target.linux_x64()

    def test_not_equal_arch(self):
        assert Target.linux_x64() != Target.linux_arm64()

    def test_not_equal_os(self):
        assert Target.linux_x64() != Target.macos_x64()

    def test_not_equal_format(self):
        a = Target(arch="x86_64", os="linux", binary_format="elf64")
        b = Target(arch="x86_64", os="linux", binary_format="raw")
        assert a != b

    def test_hashable(self):
        d: dict[Target, str] = {Target.linux_x64(): "linux", Target.windows_x64(): "win"}
        assert d[Target.linux_x64()] == "linux"

    def test_in_frozenset(self):
        s = frozenset({Target.linux_x64(), Target.windows_x64()})
        assert Target.linux_x64() in s
        assert Target.macos_arm64() not in s

    def test_frozen(self):
        t = Target.linux_x64()
        with pytest.raises((AttributeError, TypeError)):
            t.arch = "arm64"  # type: ignore[misc]


class TestTargetStr:
    def test_str_linux(self):
        assert str(Target.linux_x64()) == "x86_64-linux-elf64"

    def test_str_windows(self):
        assert str(Target.windows_x64()) == "x86_64-windows-pe"

    def test_str_wasm(self):
        assert str(Target.wasm()) == "wasm32-none-wasm"
