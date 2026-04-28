"""RawPackager: writes native_bytes verbatim, with no container.

The simplest possible packager.  Useful for embedded targets where the
firmware loader or flash programmer handles placement and execution directly —
no ELF, Mach-O, or PE header required.

Any ``Target`` with ``binary_format="raw"`` is accepted.

Example
-------
::

    code = bytes([0xD7, 0x01, 0xC0])  # Some ISA's opcodes
    artifact = CodeArtifact(native_bytes=code, entry_point=0,
                            target=Target.raw(arch="i4004"))
    raw = RawPackager()
    result = raw.pack(artifact)
    assert result == code
"""

from __future__ import annotations

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target


class RawPackager:
    """Pass-through packager that returns ``native_bytes`` unchanged.

    Accepted targets: any ``Target`` with ``binary_format="raw"``.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.raw(),
        Target.raw(arch="i4004"),
        Target.raw(arch="i8008"),
        Target.raw(arch="x86_64"),
        Target.raw(arch="arm64"),
        Target.raw(arch="wasm32"),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target not in self.supported_targets:
            if artifact.target.binary_format != "raw":
                raise UnsupportedTargetError(artifact.target)
        return artifact.native_bytes

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".bin"
