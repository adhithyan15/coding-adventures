"""Tetrad-on-LANG runtime — package entry point.

Public API
----------
``compile_to_iir(source) -> IIRModule``
    Front-end pipeline: source → CodeObject → IIRModule (standard opcodes).

``code_object_to_iir(code, *, module_name=...) -> IIRModule``
    Translation only: pre-built CodeObject → IIRModule.

``code_object_to_iir_with_sidecar(main, source_path, ...) -> (IIRModule, bytes)``
    Translation + source-map composition: produces an ``IIRModule`` *and* a
    ``DebugSidecar`` blob that maps IIR instruction indices back to the
    original Tetrad source file, line, and column.  Pass the returned bytes
    to ``debug_sidecar.DebugSidecarReader`` for breakpoint resolution.

``TetradRuntime``
    End-to-end façade.  ``runtime.run(source)`` for the interpreter path;
    ``runtime.run_with_jit(source)`` for the JIT path;
    ``runtime.compile_with_debug(source, source_path)`` for debug compilation;
    ``runtime.run_with_debug(source, source_path, hooks=..., breakpoints=...)``
    for execution with attached debug hooks.

``Intel4004Backend``
    Re-export from the ``intel4004-backend`` package — kept here so
    callers that reach ``tetrad_runtime.Intel4004Backend`` keep
    working.  New code should import from ``intel4004_backend``
    directly.

``TETRAD_OPCODE_EXTENSIONS``
    Dict of Tetrad-specific opcode handlers (``tetrad.move`` and the
    u8-wrapping ``shl`` override) that vm-core needs to execute Tetrad IIR.
    Exported for advanced callers who want to construct their own
    ``VMCore`` rather than using ``TetradRuntime``.
"""

from __future__ import annotations

from tetrad_runtime.iir_translator import (
    TETRAD_OPCODE_EXTENSIONS,
    code_object_to_iir,
)
from tetrad_runtime.intel4004_backend import Intel4004Backend
from tetrad_runtime.runtime import TetradRuntime, compile_to_iir
from tetrad_runtime.sidecar_builder import code_object_to_iir_with_sidecar

__all__ = [
    "TETRAD_OPCODE_EXTENSIONS",
    "Intel4004Backend",
    "TetradRuntime",
    "code_object_to_iir",
    "code_object_to_iir_with_sidecar",
    "compile_to_iir",
]
