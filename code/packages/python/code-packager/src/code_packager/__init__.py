"""code-packager: cross-platform binary packaging for compiled code.

This package is the final stage of the ahead-of-time compilation pipeline.
It takes a :class:`CodeArtifact` — a blob of native machine code produced by
any backend — and wraps it in the appropriate OS-specific binary format.

Because binary formats (ELF, Mach-O, PE, WASM) are just data layouts, a Mac
can produce a Windows ``.exe``, a Linux CI machine can produce a macOS Mach-O
binary, and a Pi can produce a WASM module.  Cross-compilation is first-class.

Quick start
-----------
::

    from code_packager import CodeArtifact, PackagerRegistry, Target

    # Code compiled by a backend for Linux x86-64
    code = b"\\x48\\x31\\xc0\\xc3"          # xor rax, rax; ret
    artifact = CodeArtifact(
        native_bytes=code,
        entry_point=0,
        target=Target.linux_x64(),
    )

    registry = PackagerRegistry.default()
    elf_bytes = registry.pack(artifact)     # → valid ELF64 binary

    # Same code, Windows target (cross-compilation)
    win_artifact = CodeArtifact(
        native_bytes=code,
        entry_point=0,
        target=Target.windows_x64(),
    )
    exe_bytes = registry.pack(win_artifact) # → valid PE32+ (.exe)

Public API
----------
- :class:`Target` — immutable target triple with factory methods
- :class:`CodeArtifact` — handoff object between backend and packager
- :class:`PackagerRegistry` — looks up the right packager for a target
- :class:`PackagerProtocol` — structural protocol every packager satisfies
- Packagers: :class:`RawPackager`, :class:`IntelHexPackager`,
  :class:`Elf64Packager`, :class:`MachO64Packager`,
  :class:`PePackager`, :class:`WasmPackager`
- Exceptions: :class:`PackagerError`, :class:`UnsupportedTargetError`,
  :class:`ArtifactTooLargeError`, :class:`MissingMetadataError`
"""

from __future__ import annotations

from code_packager.artifact import CodeArtifact
from code_packager.elf64 import Elf64Packager
from code_packager.errors import (
    ArtifactTooLargeError,
    MissingMetadataError,
    PackagerError,
    UnsupportedTargetError,
)
from code_packager.intel_hex import IntelHexPackager
from code_packager.macho64 import MachO64Packager
from code_packager.pe import PePackager
from code_packager.protocol import PackagerProtocol, PackagerRegistry
from code_packager.raw import RawPackager
from code_packager.target import Target
from code_packager.wasm import WasmPackager

__all__ = [
    "ArtifactTooLargeError",
    "CodeArtifact",
    "Elf64Packager",
    "IntelHexPackager",
    "MachO64Packager",
    "MissingMetadataError",
    "PackagerError",
    "PackagerProtocol",
    "PackagerRegistry",
    "PePackager",
    "RawPackager",
    "Target",
    "UnsupportedTargetError",
    "WasmPackager",
]
