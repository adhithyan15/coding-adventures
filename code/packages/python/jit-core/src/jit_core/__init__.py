"""jit-core: generic JIT specialization engine for the LANG pipeline (LANG03).

Public surface
--------------
``JITCore``         — the top-level JIT compilation and dispatch controller.
``CIRInstr``        — typed CompilerIR instruction emitted by the specializer.
``JITCache``        — compiled-function cache.
``JITCacheEntry``   — per-function cache entry with runtime statistics.
``BackendProtocol`` — structural protocol for JIT backends.
``specialise``      — IIRFunction → list[CIRInstr] specialization pass.
``optimizer``       — CIR constant-folding and DCE optimizer module.
``JITError``        — base exception.
``DeoptimizerError``, ``UnspecializableError`` — specific error types.
"""

from jit_core.backend import BackendProtocol
from jit_core.cache import JITCache, JITCacheEntry
from jit_core.cir import CIRInstr
from jit_core.core import JITCore
from jit_core.errors import DeoptimizerError, JITError, UnspecializableError
from jit_core.specialise import specialise

__all__ = [
    "JITCore",
    "CIRInstr",
    "JITCache",
    "JITCacheEntry",
    "BackendProtocol",
    "specialise",
    "JITError",
    "DeoptimizerError",
    "UnspecializableError",
]
