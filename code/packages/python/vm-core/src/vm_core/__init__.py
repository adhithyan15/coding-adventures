"""vm-core: generic register VM interpreter for the LANG pipeline.

Public surface
--------------
``VMCore``              — the main interpreter (register VM).
``VMFrame``             — per-call-frame state (frame stack entry).
``VMMetrics``           — execution statistics snapshot.
``BranchStats``         — taken/not-taken counters for one conditional branch.
``VMProfiler``          — inline type profiler.
``VMTrace`` / ``VMTracer`` — opt-in per-instruction trace records.
``TypeMapper``          — type alias for a runtime-value → type-string callable.
``default_type_mapper`` — the Python-primitive default type mapper.
``BuiltinRegistry``     — maps builtin names to host callables.
``DebugHooks``          — callback interface for debug adapters (LANG06).
``StepMode``            — step granularity enum (IN, OVER, OUT).
``VMError``             — base exception.
``UnknownOpcodeError``, ``FrameOverflowError``, ``UndefinedVariableError``,
``VMInterrupt``         — specific error types.
``UncaughtConditionError`` — raised when THROW has no matching handler (VMCOND00).
"""

from vm_core.builtins import BuiltinRegistry
from vm_core.core import VMCore
from vm_core.debug import DebugHooks, StepMode
from vm_core.errors import (
    FrameOverflowError,
    UncaughtConditionError,
    UndefinedVariableError,
    UnknownOpcodeError,
    VMError,
    VMInterrupt,
)
from vm_core.frame import RegisterFile, VMFrame
from vm_core.metrics import BranchStats, VMMetrics
from vm_core.profiler import TypeMapper, VMProfiler, default_type_mapper
from vm_core.tracer import VMTrace, VMTracer

__all__ = [
    "VMCore",
    "VMFrame",
    "RegisterFile",
    "VMMetrics",
    "BranchStats",
    "VMProfiler",
    "VMTrace",
    "VMTracer",
    "TypeMapper",
    "default_type_mapper",
    "BuiltinRegistry",
    "DebugHooks",
    "StepMode",
    "VMError",
    "UnknownOpcodeError",
    "FrameOverflowError",
    "UndefinedVariableError",
    "VMInterrupt",
    "UncaughtConditionError",
]
