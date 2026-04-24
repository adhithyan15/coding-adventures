"""vm-core: generic register VM interpreter for the LANG pipeline.

Public surface
--------------
``VMCore``              — the main interpreter (register VM).
``VMFrame``             — per-call-frame state (frame stack entry).
``VMMetrics``           — execution statistics snapshot.
``BranchStats``         — taken/not-taken counters for one conditional branch.
``VMProfiler``          — inline type profiler.
``TypeMapper``          — type alias for a runtime-value → type-string callable.
``default_type_mapper`` — the Python-primitive default type mapper.
``BuiltinRegistry``     — maps builtin names to host callables.
``VMError``             — base exception.
``UnknownOpcodeError``, ``FrameOverflowError``, ``UndefinedVariableError``,
``VMInterrupt``         — specific error types.
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
from vm_core.metrics import BranchStats, VMMetrics
from vm_core.profiler import TypeMapper, VMProfiler, default_type_mapper

__all__ = [
    "VMCore",
    "VMFrame",
    "RegisterFile",
    "VMMetrics",
    "BranchStats",
    "VMProfiler",
    "TypeMapper",
    "default_type_mapper",
    "BuiltinRegistry",
    "VMError",
    "UnknownOpcodeError",
    "FrameOverflowError",
    "UndefinedVariableError",
    "VMInterrupt",
]
