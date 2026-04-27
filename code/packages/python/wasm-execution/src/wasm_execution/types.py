"""types.py --- Shared type definitions for the WASM execution engine.

These data structures are shared across the execution engine, instruction
handlers, and control flow logic. Defining them here avoids circular imports.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from wasm_types import FuncType, FunctionBody, GlobalType

from wasm_execution.values import WasmValue


# ===========================================================================
# Control Flow Structures
# ===========================================================================


@dataclass
class Label:
    """A label on the label stack, tracking one level of structured control flow.

    When ``br N`` executes, it unwinds to the Nth label from the top. The
    behaviour depends on whether the label is from a block or a loop:

    - block/if labels: branch jumps to END (forward).
    - loop labels: branch jumps to LOOP START (backward).
    """

    arity: int
    """How many result values this block/loop/if produces (0 or 1 in WASM 1.0)."""

    target_pc: int
    """Where to jump when branching to this label."""

    stack_height: int
    """The typed stack height when this block started."""

    is_loop: bool
    """Whether this label is from a ``loop`` instruction."""


@dataclass
class ControlTarget:
    """A control flow map entry: records where a block/loop/if ends."""

    end_pc: int
    """Instruction index of the matching ``end``."""

    else_pc: int | None
    """Instruction index of ``else``, or None if no else branch."""


# ===========================================================================
# Execution Context
# ===========================================================================


@dataclass
class WasmExecutionContext:
    """Per-execution context passed to all WASM instruction handlers.

    Carries all runtime state: memory, tables, globals, locals, labels, etc.
    """

    memory: Any | None
    """Linear memory (None if the module has no memory section)."""

    tables: list[Any]
    """Function reference tables for indirect calls."""

    globals: list[WasmValue]
    """Global variable values."""

    global_types: list[GlobalType]
    """Global variable type descriptors."""

    func_types: list[FuncType]
    """All function type signatures (imports + module functions)."""

    func_bodies: list[FunctionBody | None]
    """Function bodies (None for imported functions)."""

    host_functions: list[Any | None]
    """Host function implementations (None for module-defined functions)."""

    typed_locals: list[WasmValue]
    """The current frame's local variables (params + declared locals)."""

    label_stack: list[Label]
    """Control flow label stack for the current frame."""

    control_flow_map: dict[int, ControlTarget]
    """Pre-computed control flow map: block/loop/if start -> end/else."""

    saved_frames: list[SavedFrame]
    """Saved frames for function calls."""

    returned: bool = False
    """Whether the current function has returned."""

    return_values: list[WasmValue] = field(default_factory=list)
    """Return values from the current function."""


@dataclass
class SavedFrame:
    """A saved call frame -- the caller's state before a function call."""

    locals: list[WasmValue]
    """The caller's local variables."""

    label_stack: list[Label]
    """The caller's label stack."""

    stack_height: int
    """The typed stack height when the call was made."""

    control_flow_map: dict[int, ControlTarget]
    """The caller's control flow map."""

    code: Any
    """The caller's current CodeObject."""

    return_pc: int
    """The caller's return PC (instruction after the call)."""

    return_arity: int
    """The caller's function return arity."""
