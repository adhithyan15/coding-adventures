"""Backwards-compatibility re-export of Backend / BackendProtocol from codegen-core.

``BackendProtocol`` was originally defined here in ``jit_core.backend``.
It has been moved to ``codegen_core.backend`` (LANG19) as a generic
``Backend[IR]`` protocol so it can serve both the JIT/AOT path
(``Backend[list[CIRInstr]]``) and the compiled-language path
(``Backend[IrProgram]``).

This module re-exports both ``BackendProtocol`` and ``Backend`` so that
existing callers of ``from jit_core.backend import BackendProtocol``
continue to work without modification.

New code should import from ``codegen_core`` directly:

    from codegen_core import Backend, BackendProtocol
"""

from codegen_core.backend import Backend, BackendProtocol, CIRBackend
from codegen_core.cir import CIRInstr  # re-export for convenience

__all__ = ["Backend", "BackendProtocol", "CIRBackend", "CIRInstr"]
