"""embed_debug_info — one-call convenience dispatcher.

Reads the target platform from a ``CodeArtifact``-like object and dispatches
to either ``DwarfEmitter`` (ELF/Mach-O) or ``CodeViewEmitter`` (PE).

The ``artifact`` parameter is duck-typed — any object with the following
attributes works:

    artifact.target          str  — "linux", "macos", "windows"
    artifact.load_address    int  — base address (DwarfEmitter)
    artifact.image_base      int  — PE image base (CodeViewEmitter)
    artifact.symbol_table    dict[str, int]  — fn_name → byte offset
    artifact.code_size       int  — total code size in bytes (DwarfEmitter)
    artifact.code_rva        int  — RVA of .text section (CodeViewEmitter)

Usage::

    from native_debug_info import embed_debug_info

    packed = packager.pack(artifact)
    if sidecar:
        packed = embed_debug_info(packed, artifact, sidecar)
"""

from __future__ import annotations

from debug_sidecar import DebugSidecarReader

from .codeview import CodeViewEmitter
from .dwarf import DwarfEmitter

_ELF_TARGETS = frozenset({"linux", "elf", "freebsd", "wasm"})
_MACHO_TARGETS = frozenset({"macos", "darwin", "macho"})
_PE_TARGETS = frozenset({"windows", "win32", "pe"})


def embed_debug_info(packed_bytes: bytes, artifact, sidecar_bytes: bytes) -> bytes:
    """Embed native debug info into a packed binary.

    Parameters
    ----------
    packed_bytes:
        Output of ``code_packager.pack()`` — a raw ELF, Mach-O, or PE binary.
    artifact:
        ``CodeArtifact``-like object that supplies target platform and
        symbol table.  See module docstring for required attributes.
    sidecar_bytes:
        Raw sidecar bytes from ``DebugSidecarWriter.finish()``.

    Returns
    -------
    bytes
        The same binary enriched with DWARF or CodeView debug sections.

    Raises
    ------
    ValueError
        If ``artifact.target`` is not a recognised platform.
    """
    reader = DebugSidecarReader(sidecar_bytes)
    target = getattr(artifact, "target", "").lower()

    if target in _ELF_TARGETS:
        emitter = DwarfEmitter(
            reader=reader,
            load_address=getattr(artifact, "load_address", 0x400000),
            symbol_table=getattr(artifact, "symbol_table", {}),
            code_size=getattr(artifact, "code_size", 0),
        )
        return emitter.embed_in_elf(packed_bytes)

    if target in _MACHO_TARGETS:
        emitter = DwarfEmitter(
            reader=reader,
            load_address=getattr(artifact, "load_address", 0),
            symbol_table=getattr(artifact, "symbol_table", {}),
            code_size=getattr(artifact, "code_size", 0),
        )
        return emitter.embed_in_macho(packed_bytes)

    if target in _PE_TARGETS:
        emitter = CodeViewEmitter(
            reader=reader,
            image_base=getattr(artifact, "image_base", 0x140000000),
            symbol_table=getattr(artifact, "symbol_table", {}),
            code_rva=getattr(artifact, "code_rva", 0x1000),
        )
        return emitter.embed_in_pe(packed_bytes)

    raise ValueError(f"unsupported target platform: {target!r}")
