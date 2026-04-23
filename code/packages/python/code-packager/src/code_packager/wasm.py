"""WasmPackager: wraps native bytes in a WebAssembly module.

For WASM targets, ``native_bytes`` is expected to be a raw WASM *function
body* (the ``expr`` part of a Code section entry) — not a complete .wasm file.
The packager wraps it in a minimal module with:

- A type section declaring one function type: ``() → i32``
- A function section referencing that type
- An export section exporting the function as ``"main"`` (or the first name
  from ``metadata["exports"]``)
- A code section containing ``native_bytes`` as the function body

This is the minimal module the WASM runtime needs to execute the code.

Metadata keys
-------------
``exports`` (list[str])
    Names to export the entry function as.  Default: ``["main"]``.
    Only the first element is used (single-function modules).

Why not use ``native_bytes`` directly as a .wasm file?
-------------------------------------------------------
Our backends emit *instruction bytes* for a function body, not a complete
module.  The module envelope (type declarations, function index, export table)
must be constructed by the packager.  If a backend ever produces a complete
WASM module, ``RawPackager`` can be used directly instead.

Supported targets: :func:`Target.wasm`.
"""

from __future__ import annotations

from wasm_module_encoder import encode_module
from wasm_types import (
    Export,
    ExternalKind,
    FuncType,
    FunctionBody,
    ValueType,
    WasmModule,
)

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target


class WasmPackager:
    """Package native bytes as a minimal WASM module.

    Accepted targets: :func:`Target.wasm`.

    Metadata keys
    -------------
    ``exports`` (list[str])
        Function export names.  Default: ``["main"]``.  Only the first is used.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.wasm(),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target not in self.supported_targets:
            raise UnsupportedTargetError(artifact.target)

        exports_meta = artifact.metadata.get("exports", ["main"])
        export_name: str = exports_meta[0] if exports_meta else "main"

        # Type: () → i32
        func_type = FuncType(params=[], results=[ValueType.I32])

        # Function body from native_bytes (no local declarations)
        body = FunctionBody(locals=[], code=artifact.native_bytes)

        module = WasmModule(
            types=[func_type],
            functions=[0],      # function 0 has type 0
            exports=[Export(name=export_name, kind=ExternalKind.FUNCTION, index=0)],
            code=[body],
        )

        return encode_module(module)

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".wasm"
