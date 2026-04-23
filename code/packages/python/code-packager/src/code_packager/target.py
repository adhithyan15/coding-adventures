"""Target triple: describes the machine a binary will run on.

A target is independent of the host machine running the compiler —
cross-compilation is first-class.  ``arch`` names the instruction set,
``os`` names the operating system ABI, and ``binary_format`` names the
container format the OS expects.

The three-field split (rather than an autoconf-style string like
``x86_64-unknown-linux-gnu``) lets callers branch on individual components
without string parsing and keeps the API extensible for future fields like
``abi`` or ``float_abi``.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Target:
    """Immutable description of a compilation target.

    Instances are hashable and equality-comparable so they can be used as
    dict keys in a :class:`~code_packager.protocol.PackagerRegistry`.

    Attributes
    ----------
    arch:
        Instruction-set name.  Conventional values:
        ``"x86_64"``, ``"arm64"``, ``"wasm32"``, ``"i4004"``, ``"i8008"``.
    os:
        Operating-system / runtime environment.  Conventional values:
        ``"linux"``, ``"macos"``, ``"windows"``, ``"none"`` (bare metal).
    binary_format:
        Container format the OS loader expects.  Conventional values:
        ``"elf64"``, ``"macho64"``, ``"pe"``, ``"wasm"``, ``"raw"``,
        ``"intel_hex"``.
    """

    arch: str
    os: str
    binary_format: str

    # ------------------------------------------------------------------
    # Factory methods for the common triples
    # ------------------------------------------------------------------

    @classmethod
    def linux_x64(cls) -> "Target":
        """Linux x86-64 ELF64 executable."""
        return cls(arch="x86_64", os="linux", binary_format="elf64")

    @classmethod
    def linux_arm64(cls) -> "Target":
        """Linux AArch64 ELF64 executable."""
        return cls(arch="arm64", os="linux", binary_format="elf64")

    @classmethod
    def macos_x64(cls) -> "Target":
        """macOS x86-64 Mach-O64 executable."""
        return cls(arch="x86_64", os="macos", binary_format="macho64")

    @classmethod
    def macos_arm64(cls) -> "Target":
        """macOS Apple Silicon Mach-O64 executable."""
        return cls(arch="arm64", os="macos", binary_format="macho64")

    @classmethod
    def windows_x64(cls) -> "Target":
        """Windows x86-64 PE32+ executable (.exe)."""
        return cls(arch="x86_64", os="windows", binary_format="pe")

    @classmethod
    def wasm(cls) -> "Target":
        """WebAssembly module."""
        return cls(arch="wasm32", os="none", binary_format="wasm")

    @classmethod
    def raw(cls, arch: str = "unknown") -> "Target":
        """Bare binary with no container (embedded / ROM targets)."""
        return cls(arch=arch, os="none", binary_format="raw")

    @classmethod
    def intel_4004(cls) -> "Target":
        """Intel 4004 Intel HEX ROM image."""
        return cls(arch="i4004", os="none", binary_format="intel_hex")

    @classmethod
    def intel_8008(cls) -> "Target":
        """Intel 8008 Intel HEX ROM image."""
        return cls(arch="i8008", os="none", binary_format="intel_hex")

    def __str__(self) -> str:
        return f"{self.arch}-{self.os}-{self.binary_format}"
