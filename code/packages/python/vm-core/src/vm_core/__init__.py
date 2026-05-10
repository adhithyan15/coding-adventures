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
``UncaughtConditionError`` — raised when THROW/ERROR has no matching handler (VMCOND00).
``HandlerChainError``   — raised on handler-chain underflow (VMCOND00 Phase 3).
``HandlerNode``         — one node in the VMCOND00 Layer 3 handler chain.
``RestartChainError``   — raised on restart-chain error (VMCOND00 Phase 4).
``UnboundExitTagError`` — raised by exit_to with no matching exit point (VMCOND00
Phase 4).
``RestartNode``         — one node in the VMCOND00 Layer 4 restart chain.
``ExitPointNode``       — one node in the VMCOND00 Layer 5 exit-point chain.
"""

from vm_core.builtins import BuiltinRegistry
from vm_core.core import VMCore
from vm_core.debug import DebugHooks, StepMode
from vm_core.errors import (
    FrameOverflowError,
    HandlerChainError,
    RestartChainError,
    UnboundExitTagError,
    UncaughtConditionError,
    UndefinedVariableError,
    UnknownOpcodeError,
    VMError,
    VMInterrupt,
)
from vm_core.exit_chain import ExitPointNode
from vm_core.frame import RegisterFile, VMFrame
from vm_core.handler_chain import HandlerNode
from vm_core.metrics import BranchStats, VMMetrics
from vm_core.profiler import TypeMapper, VMProfiler, default_type_mapper
from vm_core.restart_chain import RestartNode
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
    "HandlerChainError",
    "HandlerNode",
    "RestartChainError",
    "UnboundExitTagError",
    "RestartNode",
    "ExitPointNode",
]
