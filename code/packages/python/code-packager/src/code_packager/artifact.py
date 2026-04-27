"""CodeArtifact: the handoff object between a compilation backend and the packager.

A ``CodeArtifact`` carries everything the packager needs to produce a
platform-specific binary:

- The raw machine code bytes, ready for the target ISA
- The byte offset where the OS should transfer control (entry point)
- Which platform the bytes are compiled for (the ``Target``)
- An optional symbol table mapping function names to byte offsets
- An optional metadata dict for packager-specific hints

The ``metadata`` field is intentionally untyped — each packager documents the
keys it consumes and silently ignores unknown keys.  This keeps the artifact
API stable as new packagers add new options.

Example
-------
::

    from code_packager import CodeArtifact, Target

    code = b"\\x48\\x31\\xc0\\xc3"  # xor rax, rax; ret  (x86-64)
    artifact = CodeArtifact(
        native_bytes=code,
        entry_point=0,
        target=Target.linux_x64(),
        symbol_table={"main": 0},
    )
"""

from __future__ import annotations

from dataclasses import dataclass, field

from code_packager.target import Target


@dataclass
class CodeArtifact:
    """Native-code handoff between a backend and a packager.

    Attributes
    ----------
    native_bytes:
        Raw machine code for the target ISA.  All compiled functions are
        concatenated in link order (produced by ``aot_core.link.link()``).
    entry_point:
        Byte offset within ``native_bytes`` where execution begins.
        Produced by ``aot_core.link.entry_point_offset()``.
    target:
        The ``Target`` these bytes were compiled for.
    symbol_table:
        Maps function name → byte offset within ``native_bytes``.
        The packager uses this to populate the binary's export section.
    metadata:
        Packager-specific key/value hints.  Each packager documents which
        keys it reads; unknown keys are silently ignored.

        Common keys:

        - ``"load_address"`` (int) — override virtual load address
          (ELF/Mach-O/PE)
        - ``"subsystem"`` (int) — PE subsystem: 2 = GUI, 3 = console (PE only)
        - ``"origin"`` (int) — ROM origin address (Intel HEX only)
        - ``"stack_size"`` (int) — hint for the OS (Mach-O/PE)
        - ``"exports"`` (list[str]) — WASM function export names
        - ``"imports"`` (list[dict]) — WASM import declarations
    """

    native_bytes: bytes
    entry_point: int
    target: Target
    symbol_table: dict[str, int] = field(default_factory=dict)
    metadata: dict[str, object] = field(default_factory=dict)
