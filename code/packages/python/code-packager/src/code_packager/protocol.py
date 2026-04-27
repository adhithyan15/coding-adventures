"""PackagerProtocol and PackagerRegistry.

The ``PackagerProtocol`` is a structural protocol — any object with the right
methods qualifies, no inheritance required.  This makes it easy to add a new
packager without changing the registry or the protocol definition.

``PackagerRegistry.default()`` returns a registry pre-populated with all the
built-in packagers so callers don't have to wire them up manually.

Example
-------
::

    from code_packager import PackagerRegistry, CodeArtifact, Target

    registry = PackagerRegistry.default()
    artifact = CodeArtifact(
        native_bytes=b"\\x90",      # NOP
        entry_point=0,
        target=Target.linux_x64(),
    )
    binary = registry.pack(artifact)  # Returns ELF64 bytes
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol, runtime_checkable

from code_packager.errors import UnsupportedTargetError

if TYPE_CHECKING:
    from code_packager.artifact import CodeArtifact
    from code_packager.target import Target


@runtime_checkable
class PackagerProtocol(Protocol):
    """Structural protocol every packager must satisfy.

    A packager wraps a :class:`~code_packager.artifact.CodeArtifact`'s
    native bytes in a platform-specific binary container and returns the
    resulting bytes.

    Implementations must be **pure Python** — no OS-specific system calls —
    so they can run on any host and produce output for any target.
    """

    supported_targets: frozenset["Target"]

    def pack(self, artifact: "CodeArtifact") -> bytes:
        """Wrap ``artifact.native_bytes`` in the platform binary format.

        Parameters
        ----------
        artifact:
            The compiled artifact to package.

        Returns
        -------
        bytes
            Complete binary ready for writing to disk.

        Raises
        ------
        UnsupportedTargetError
            If ``artifact.target`` is not in ``supported_targets``.
        ArtifactTooLargeError
            If the binary format's size limit would be exceeded.
        """
        ...

    def file_extension(self, target: "Target") -> str:
        """Return the conventional file extension for ``target``.

        For example, ``".exe"`` for PE targets, ``".elf"`` for ELF64.
        Includes the leading dot.
        """
        ...


class PackagerRegistry:
    """Maps :class:`~code_packager.target.Target` to the packager that handles it.

    Packagers are looked up by exact ``Target`` equality.  A single packager
    may handle multiple targets (registered once per target).

    Use :meth:`default` to obtain a registry pre-populated with all built-in
    packagers.
    """

    def __init__(self) -> None:
        self._registry: dict["Target", PackagerProtocol] = {}

    def register(self, packager: PackagerProtocol) -> None:
        """Register ``packager`` for every target it declares support for."""
        for target in packager.supported_targets:
            self._registry[target] = packager

    def get(self, target: "Target") -> PackagerProtocol:
        """Return the packager for ``target``.

        Raises
        ------
        UnsupportedTargetError
            If no packager handles ``target``.
        """
        if target not in self._registry:
            raise UnsupportedTargetError(target)
        return self._registry[target]

    def pack(self, artifact: "CodeArtifact") -> bytes:
        """Convenience: look up the right packager and call ``pack``."""
        return self.get(artifact.target).pack(artifact)

    @classmethod
    def default(cls) -> "PackagerRegistry":
        """Return a registry populated with all built-in packagers."""
        from code_packager.elf64 import Elf64Packager
        from code_packager.intel_hex import IntelHexPackager
        from code_packager.macho64 import MachO64Packager
        from code_packager.pe import PePackager
        from code_packager.raw import RawPackager
        from code_packager.wasm import WasmPackager

        registry = cls()
        registry.register(RawPackager())
        registry.register(IntelHexPackager())
        registry.register(Elf64Packager())
        registry.register(MachO64Packager())
        registry.register(PePackager())
        registry.register(WasmPackager())
        return registry
