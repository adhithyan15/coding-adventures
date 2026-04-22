"""vm-core: generic register VM interpreter for the LANG pipeline.

Public surface
--------------
``VMCore``          — the main interpreter (register VM).
``VMFrame``         — per-call-frame state (frame stack entry).
``VMMetrics``       — execution statistics snapshot.
``VMProfiler``      — inline type profiler.
``BuiltinRegistry`` — maps builtin names to host callables.
``VMError``         — base exception.
``UnknownOpcodeError``, ``FrameOverflowError``, ``UndefinedVariableError``,
``VMInterrupt``     — specific error types.
"""

from vm_core.builtins import BuiltinRegistry
from vm_core.core import VMCore
from vm_core.errors import (
    FrameOverflowError,
    UndefinedVariableError,
    UnknownOpcodeError,
    VMError,
    VMInterrupt,
)
from vm_core.frame import RegisterFile, VMFrame
from vm_core.metrics import VMMetrics
from vm_core.profiler import VMProfiler

__all__ = [
    "VMCore",
    "VMFrame",
    "RegisterFile",
    "VMMetrics",
    "VMProfiler",
    "BuiltinRegistry",
    "VMError",
    "UnknownOpcodeError",
    "FrameOverflowError",
    "UndefinedVariableError",
    "VMInterrupt",
]
