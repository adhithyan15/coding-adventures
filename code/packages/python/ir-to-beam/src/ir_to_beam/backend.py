"""Lower a ``compiler-ir`` ``IrProgram`` into a ``BEAMModule``.

This is BEAM01 Phase 3 + TW03 Phase 1 (BEAM extension).  See
``code/specs/BEAM01-twig-on-real-erl.md`` and
``code/specs/TW03-lisp-primitives-and-gc.md`` for the broader
plan.

Pipeline
========

The package consumes already-lowered ``IrProgram`` objects (the
output of ``ir-optimizer`` or, for tests, of a hand-built
program).  It produces a ``BEAMModule`` ready for
``beam_bytecode_encoder.encode_beam`` to serialize.

Calling convention (TW03 Phase 1)
=================================

BEAM has two register files:

* ``x`` — caller-saves scratch.  Args to a call go in ``x0..xN-1``;
  return value comes back in ``x0``.
* ``y`` — stack-allocated locals (callee-saves).  Survive across
  calls.  Require an ``allocate``/``deallocate`` framing pair.

Recursive functions need to preserve state across calls, so we
**use y-registers for everything in the function body** and only
touch x-registers at call boundaries.  This sidesteps the
"caller-saves" footgun that bit JVM01 — the BEAM stack frame
naturally provides per-invocation isolation.

Concretely, **IR register ``rN`` lowers to BEAM ``yN``** in every
body opcode (LOAD_IMM, ADD, BRANCH_Z, etc.).  At function entry
we ``allocate K, 0`` enough y-registers for the program's max
register index, then copy the args from ``x0..x{arity-1}`` into
``y2..y{arity+1}`` to match the Twig calling convention (param
``i`` at register ``_REG_PARAM_BASE + i = 2 + i``).

At a CALL site:

1. Move each arg from its IR param slot ``y{2+i}`` into the BEAM
   ``x{i}`` slot the callee expects.
2. ``call N, label`` (where N = arity).
3. Move the result back from ``x0`` into ``y1`` (the IR's
   ``_REG_HALT_RESULT`` convention).

At RET:

1. Move ``y1`` (the IR return-value register) into ``x0``.
2. ``deallocate K``.
3. ``return``.

Why this matters: the JVM and CLR Twig frontends both put params
in ``r2..rN+1`` and the return value in ``r1`` — same convention,
shared IR.  ir-to-beam absorbs the BEAM-specific x↔y dance so
the upstream compilers don't need a backend-aware register layout.

Branch lowering (TW03 Phase 1 additions)
========================================

BEAM has no "set boolean register from comparison" opcode pattern;
its comparison opcodes (is_lt, is_eq_exact, …) are conditional
*jumps*: each takes a fail-label and falls through if the
condition holds, jumping otherwise.  ``CMP_EQ dst, a, b`` lowers
as a 5-instruction if-then-else::

    is_eq_exact F, a, b
    move {integer, 1}, dst
    jump END
  F:
    move {integer, 0}, dst
  END:

with synthetic labels ``F`` and ``END`` allocated from the
backend's ``fresh_label`` pool.

``BRANCH_Z reg, target`` lowers to ``is_ne_exact target, reg, {integer,0}``
— is_ne_exact's "fall through if NOT equal, jump if equal"
semantics directly match BRANCH_Z's "jump when zero" condition.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Final

from beam_bytecode_encoder import (
    BEAMExport,
    BEAMImport,
    BEAMInstruction,
    BEAMModule,
    BEAMOperand,
    BEAMTag,
)
from compiler_ir import (
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)


# ---------------------------------------------------------------------------
# BEAM opcodes we emit (subset of the full table)
# ---------------------------------------------------------------------------

# Names line up with ``beam_opcode_metadata.catalog`` for sanity.
_OP_LABEL: Final[int] = 1
_OP_FUNC_INFO: Final[int] = 2
_OP_INT_CODE_END: Final[int] = 3
_OP_CALL: Final[int] = 4
_OP_ALLOCATE: Final[int] = 12
_OP_DEALLOCATE: Final[int] = 18
_OP_RETURN: Final[int] = 19
_OP_IS_LT: Final[int] = 39           # is_lt fail src1 src2
_OP_IS_EQ_EXACT: Final[int] = 43     # is_eq_exact fail src1 src2
_OP_IS_NE_EXACT: Final[int] = 44     # is_ne_exact fail src1 src2
_OP_JUMP: Final[int] = 61
_OP_MOVE: Final[int] = 64
_OP_CALL_EXT_ONLY: Final[int] = 78
_OP_GC_BIF2: Final[int] = 125

# Minimum ``max_opcode`` value the modern Erlang loader accepts.
# Loader treats this as a "what BEAM dialect was this compiled
# against" declaration, not the actual highest opcode used.
_MIN_RUNTIME_MAX_OPCODE: Final[int] = 178

# Erlang BIFs we may need.
_BIF_PLUS: Final[tuple[str, str, int]] = ("erlang", "+", 2)
_BIF_MINUS: Final[tuple[str, str, int]] = ("erlang", "-", 2)
_BIF_MUL: Final[tuple[str, str, int]] = ("erlang", "*", 2)
_BIF_DIV: Final[tuple[str, str, int]] = ("erlang", "div", 2)

_ARITHMETIC_BIF: dict[IrOp, tuple[str, str, int]] = {
    IrOp.ADD: _BIF_PLUS,
    IrOp.SUB: _BIF_MINUS,
    IrOp.MUL: _BIF_MUL,
    IrOp.DIV: _BIF_DIV,
}

# Twig calling-convention constants (shared with twig-jvm-compiler
# and twig-clr-compiler — see those files for the rationale).
# IR register N maps to BEAM y-register N in body code.
_REG_HALT_RESULT: Final = 1
_REG_PARAM_BASE: Final = 2


class BEAMBackendError(ValueError):
    """Raised when an IR program cannot be lowered to a BEAM module."""


@dataclass(frozen=True)
class BEAMBackendConfig:
    """Knobs for the lowering.

    ``module_name`` is the Erlang module name (must be a valid
    atom).  ``arity_overrides`` declares the arity of specific
    callable regions (function names) — required when the
    function takes any arguments, since the IR itself doesn't
    encode arity per region.  Defaults to 0 for any region not
    listed.

    ``y_register_count`` declares how many y-registers each
    function body needs.  Auto-derived from the program's max IR
    register index when ``None`` (the typical case).
    """

    module_name: str
    arity_overrides: dict[str, int] = field(default_factory=dict)
    y_register_count: int | None = None


# ---------------------------------------------------------------------------
# Atom + import tables
# ---------------------------------------------------------------------------


@dataclass
class _AtomTable:
    """Insertion-ordered atom table that maps to 1-based BEAM indices."""

    atoms: list[str]
    index_of: dict[str, int]

    @classmethod
    def starting_with_module(cls, module_name: str) -> _AtomTable:
        return cls(atoms=[module_name], index_of={module_name: 1})

    def add(self, atom: str) -> int:
        existing = self.index_of.get(atom)
        if existing is not None:
            return existing
        self.atoms.append(atom)
        idx = len(self.atoms)
        self.index_of[atom] = idx
        return idx

    def as_tuple(self) -> tuple[str, ...]:
        return tuple(self.atoms)


@dataclass
class _ImportTable:
    """Insertion-ordered ``ImpT`` row builder."""

    rows: list[BEAMImport]
    index_of: dict[tuple[int, int, int], int]

    @classmethod
    def empty(cls) -> _ImportTable:
        return cls(rows=[], index_of={})

    def add(
        self,
        module_atom_idx: int,
        function_atom_idx: int,
        arity: int,
    ) -> int:
        key = (module_atom_idx, function_atom_idx, arity)
        existing = self.index_of.get(key)
        if existing is not None:
            return existing
        self.rows.append(
            BEAMImport(
                module_atom_index=module_atom_idx,
                function_atom_index=function_atom_idx,
                arity=arity,
            )
        )
        idx = len(self.rows)
        self.index_of[key] = idx
        return idx

    def as_tuple(self) -> tuple[BEAMImport, ...]:
        return tuple(self.rows)


@dataclass
class _Builder:
    """Mutable state during a single lower_ir_to_beam call."""

    atoms: _AtomTable
    imports: _ImportTable
    instructions: list[BEAMInstruction] = field(default_factory=list)
    exports: list[BEAMExport] = field(default_factory=list)
    next_label: int = 1

    def fresh_label(self) -> int:
        n = self.next_label
        self.next_label += 1
        return n

    def emit(self, opcode: int, *operands: BEAMOperand) -> None:
        self.instructions.append(BEAMInstruction(opcode=opcode, operands=operands))


# ---------------------------------------------------------------------------
# Region splitting and label discovery
# ---------------------------------------------------------------------------


def _discover_callable_names(program: IrProgram) -> set[str]:
    """All region names that are CALL targets or the entry label.

    Internal labels (``_else_0``, ``_endif_0``, etc.) are NOT
    callable — they're targets of branches/jumps within a single
    function, and they stay as ``label N`` opcodes inside that
    function's body.
    """
    callable_names: set[str] = {program.entry_label}
    for instr in program.instructions:
        if instr.opcode is IrOp.CALL:
            target = _operand_label(instr.operands[0], role="CALL target")
            callable_names.add(target)
    return callable_names


def _split_callable_regions(
    program: IrProgram,
    callable_names: set[str],
) -> list[tuple[str, list[IrInstruction]]]:
    """Group an IR instruction stream into ``(label_name, body)`` regions.

    A region starts at any LABEL whose name is in ``callable_names``.
    Other LABELs (synthetic ``_else_*`` / ``_endif_*`` markers)
    stay in the body for emission as ``label N`` opcodes.
    """
    regions: list[tuple[str, list[IrInstruction]]] = []
    current_name: str | None = None
    current_body: list[IrInstruction] = []
    for instr in program.instructions:
        if instr.opcode is IrOp.LABEL:
            label_arg = instr.operands[0]
            if not isinstance(label_arg, IrLabel):
                msg = (
                    "LABEL instruction must carry an IrLabel operand; "
                    f"got {label_arg!r}"
                )
                raise BEAMBackendError(msg)
            if label_arg.name in callable_names:
                if current_name is not None:
                    regions.append((current_name, current_body))
                current_name = label_arg.name
                current_body = []
                continue
            # Internal label — keep it in the current body.
            if current_name is None:
                msg = (
                    f"internal LABEL {label_arg.name!r} appears before any "
                    "callable region — this is structurally invalid IR"
                )
                raise BEAMBackendError(msg)
            current_body.append(instr)
        else:
            if current_name is None:
                msg = (
                    "instruction encountered before any LABEL — every IR "
                    "instruction stream must begin with at least one LABEL"
                )
                raise BEAMBackendError(msg)
            current_body.append(instr)
    if current_name is not None:
        regions.append((current_name, current_body))
    return regions


def _max_register_index(program: IrProgram) -> int:
    highest = 1  # enough for at least ``_REG_HALT_RESULT``
    for instr in program.instructions:
        for op in instr.operands:
            if isinstance(op, IrRegister) and op.index > highest:
                highest = op.index
    return highest


def _operand_register(value: object, *, role: str) -> int:
    if not isinstance(value, IrRegister):
        msg = f"expected IR register for {role}, got {value!r}"
        raise BEAMBackendError(msg)
    if value.index < 0:
        msg = f"register index must be non-negative, got {value.index}"
        raise BEAMBackendError(msg)
    return value.index


def _operand_immediate(value: object, *, role: str) -> int:
    if isinstance(value, int):
        return value
    if hasattr(value, "value") and isinstance(value.value, int):
        return int(value.value)
    msg = f"expected integer immediate for {role}, got {value!r}"
    raise BEAMBackendError(msg)


def _operand_label(value: object, *, role: str) -> str:
    if not isinstance(value, IrLabel):
        msg = f"expected IR label for {role}, got {value!r}"
        raise BEAMBackendError(msg)
    return value.name


# ---------------------------------------------------------------------------
# Per-instruction lowering — body code uses y-registers
# ---------------------------------------------------------------------------


def _y(reg_index: int) -> BEAMOperand:
    """Map an IR register index to its BEAM y-register operand."""
    return BEAMOperand(BEAMTag.Y, reg_index)


def _x(idx: int) -> BEAMOperand:
    return BEAMOperand(BEAMTag.X, idx)


def _i(value: int) -> BEAMOperand:
    return BEAMOperand(BEAMTag.I, value)


def _u(value: int) -> BEAMOperand:
    return BEAMOperand(BEAMTag.U, value)


def _f(value: int) -> BEAMOperand:
    return BEAMOperand(BEAMTag.F, value)


def _emit_load_imm(builder: _Builder, instr: IrInstruction) -> None:
    if len(instr.operands) != 2:
        raise BEAMBackendError(
            f"LOAD_IMM expects 2 operands, got {len(instr.operands)}"
        )
    dest = _operand_register(instr.operands[0], role="LOAD_IMM dest")
    value = _operand_immediate(instr.operands[1], role="LOAD_IMM value")
    if value < 0:
        msg = (
            f"LOAD_IMM with negative integer ({value}) is not yet supported "
            "by ir-to-beam — the BEAM compact-term encoder rejects negatives"
        )
        raise BEAMBackendError(msg)
    builder.emit(_OP_MOVE, _i(value), _y(dest))


def _emit_arithmetic(builder: _Builder, instr: IrInstruction) -> None:
    """Lower ADD/SUB/MUL/DIV via ``gc_bif2`` over y-register sources."""
    if len(instr.operands) != 3:
        msg = (
            f"{instr.opcode.name} expects 3 operands "
            f"(dest, lhs, rhs), got {len(instr.operands)}"
        )
        raise BEAMBackendError(msg)
    dest = _operand_register(instr.operands[0], role=f"{instr.opcode.name} dest")
    lhs = _operand_register(instr.operands[1], role=f"{instr.opcode.name} lhs")
    rhs = _operand_register(instr.operands[2], role=f"{instr.opcode.name} rhs")

    bif_module, bif_name, bif_arity = _ARITHMETIC_BIF[instr.opcode]
    module_atom_idx = builder.atoms.add(bif_module)
    fn_atom_idx = builder.atoms.add(bif_name)
    bif_import_idx = builder.imports.add(module_atom_idx, fn_atom_idx, bif_arity)

    builder.emit(
        _OP_GC_BIF2,
        _f(0),                              # fail label = 0
        _u(0),                              # live x-regs = 0 (we use y for state)
        _u(bif_import_idx - 1),             # BIF index (0-based here)
        _y(lhs),
        _y(rhs),
        _y(dest),
    )


def _emit_add_imm(builder: _Builder, instr: IrInstruction) -> None:
    """Lower ADD_IMM dst, src, imm.

    Used by Twig compilers as a "MOV" idiom (``ADD_IMM dst, src, 0``).
    Lower to a single ``move`` when ``imm == 0``; otherwise lower
    via ``gc_bif2 +/2`` with the immediate as the second operand.
    """
    if len(instr.operands) != 3:
        msg = f"ADD_IMM expects 3 operands, got {len(instr.operands)}"
        raise BEAMBackendError(msg)
    dest = _operand_register(instr.operands[0], role="ADD_IMM dest")
    src = _operand_register(instr.operands[1], role="ADD_IMM src")
    imm = _operand_immediate(instr.operands[2], role="ADD_IMM imm")
    if imm == 0:
        # Pure move — no BIF needed.  Compact and avoids importing
        # ``erlang:+/2`` for non-arithmetic uses.
        if dest == src:
            return  # no-op
        builder.emit(_OP_MOVE, _y(src), _y(dest))
        return

    if imm < 0:
        msg = (
            f"ADD_IMM with negative immediate ({imm}) is not yet supported "
            "by ir-to-beam"
        )
        raise BEAMBackendError(msg)

    module_atom_idx = builder.atoms.add("erlang")
    fn_atom_idx = builder.atoms.add("+")
    bif_import_idx = builder.imports.add(module_atom_idx, fn_atom_idx, 2)

    builder.emit(
        _OP_GC_BIF2,
        _f(0),
        _u(0),
        _u(bif_import_idx - 1),
        _y(src),
        _i(imm),
        _y(dest),
    )


def _emit_jump(
    builder: _Builder,
    instr: IrInstruction,
    label_for: dict[str, int],
) -> None:
    if len(instr.operands) != 1:
        msg = f"JUMP expects 1 operand, got {len(instr.operands)}"
        raise BEAMBackendError(msg)
    target = _operand_label(instr.operands[0], role="JUMP target")
    if target not in label_for:
        msg = f"JUMP target {target!r} has no corresponding LABEL"
        raise BEAMBackendError(msg)
    builder.emit(_OP_JUMP, _f(label_for[target]))


def _emit_branch(
    builder: _Builder,
    instr: IrInstruction,
    label_for: dict[str, int],
) -> None:
    """Lower BRANCH_Z / BRANCH_NZ to is_ne_exact / is_eq_exact.

    BEAM's ``is_X`` opcodes have "falls through if condition holds,
    else jumps to fail-label" semantics.  We use is_ne_exact for
    BRANCH_Z (jump when reg==0): is_ne_exact's fall-through-if-not-
    equal flips to "jump when equal".  Symmetric for BRANCH_NZ.
    """
    if len(instr.operands) != 2:
        msg = (
            f"{instr.opcode.name} expects 2 operands "
            f"(reg, target), got {len(instr.operands)}"
        )
        raise BEAMBackendError(msg)
    reg = _operand_register(instr.operands[0], role=f"{instr.opcode.name} reg")
    target = _operand_label(instr.operands[1], role=f"{instr.opcode.name} target")
    if target not in label_for:
        msg = f"{instr.opcode.name} target {target!r} has no corresponding LABEL"
        raise BEAMBackendError(msg)

    target_label = _f(label_for[target])
    if instr.opcode is IrOp.BRANCH_Z:
        # Jump when reg == 0.
        builder.emit(_OP_IS_NE_EXACT, target_label, _y(reg), _i(0))
    elif instr.opcode is IrOp.BRANCH_NZ:
        # Jump when reg != 0.
        builder.emit(_OP_IS_EQ_EXACT, target_label, _y(reg), _i(0))
    else:  # pragma: no cover — caller dispatches by opcode
        msg = f"unexpected branch opcode {instr.opcode.name}"
        raise BEAMBackendError(msg)


def _emit_cmp(
    builder: _Builder,
    instr: IrInstruction,
) -> None:
    """Lower CMP_EQ / CMP_LT / CMP_GT to is_X-then-set-bool pattern.

    The pattern emits two synthetic labels (an "else" label and an
    "end" label) and 5 instructions::

        is_X false_label, src1, src2  ; falls through if comparison holds
        move {integer, 1}, dst
        jump end_label
      false_label:
        move {integer, 0}, dst
      end_label:

    For ``CMP_GT`` we swap the operands and use ``is_lt`` (BEAM
    has no ``is_gt`` opcode).
    """
    if len(instr.operands) != 3:
        msg = (
            f"{instr.opcode.name} expects 3 operands "
            f"(dest, lhs, rhs), got {len(instr.operands)}"
        )
        raise BEAMBackendError(msg)
    dest = _operand_register(instr.operands[0], role=f"{instr.opcode.name} dest")
    lhs = _operand_register(instr.operands[1], role=f"{instr.opcode.name} lhs")
    rhs = _operand_register(instr.operands[2], role=f"{instr.opcode.name} rhs")

    false_label = builder.fresh_label()
    end_label = builder.fresh_label()

    if instr.opcode is IrOp.CMP_EQ:
        builder.emit(_OP_IS_EQ_EXACT, _f(false_label), _y(lhs), _y(rhs))
    elif instr.opcode is IrOp.CMP_LT:
        builder.emit(_OP_IS_LT, _f(false_label), _y(lhs), _y(rhs))
    elif instr.opcode is IrOp.CMP_GT:
        # BEAM has no is_gt — use is_lt with swapped operands.
        builder.emit(_OP_IS_LT, _f(false_label), _y(rhs), _y(lhs))
    else:  # pragma: no cover — caller dispatches by opcode
        msg = f"unexpected compare opcode {instr.opcode.name}"
        raise BEAMBackendError(msg)

    # Fall-through path (condition true): set dest = 1, jump to end.
    builder.emit(_OP_MOVE, _i(1), _y(dest))
    builder.emit(_OP_JUMP, _f(end_label))

    # False path: set dest = 0.
    builder.emit(_OP_LABEL, _u(false_label))
    builder.emit(_OP_MOVE, _i(0), _y(dest))

    # Common continuation.
    builder.emit(_OP_LABEL, _u(end_label))


def _emit_label_in_body(
    builder: _Builder,
    instr: IrInstruction,
    label_for: dict[str, int],
) -> None:
    """An internal LABEL appearing in the body emits a ``label N`` opcode."""
    name = _operand_label(instr.operands[0], role="LABEL operand")
    if name not in label_for:
        msg = f"internal LABEL {name!r} has no allocated BEAM label number"
        raise BEAMBackendError(msg)
    builder.emit(_OP_LABEL, _u(label_for[name]))


def _emit_call(
    builder: _Builder,
    instr: IrInstruction,
    label_for: dict[str, int],
    arity_for: dict[str, int],
) -> None:
    """Lower CALL with the Twig calling convention.

    Twig stages args in IR registers ``r2..r{arity+1}``.  BEAM
    expects them in ``x0..x{arity-1}``.  We bridge by emitting one
    ``move {y, 2+i}, {x, i}`` per arg before the ``call``, then
    one ``move {x, 0}, {y, 1}`` after to copy the return value
    back into the IR's ``_REG_HALT_RESULT`` slot.
    """
    target = _operand_label(instr.operands[0], role="CALL target")
    if target not in label_for:
        msg = (
            f"CALL target {target!r} has no corresponding LABEL — "
            "missing region in the IR"
        )
        raise BEAMBackendError(msg)
    arity = arity_for.get(target, 0)

    for i in range(arity):
        # Move the staged arg from y{2+i} to x{i}.
        builder.emit(_OP_MOVE, _y(_REG_PARAM_BASE + i), _x(i))

    builder.emit(_OP_CALL, _u(arity), _f(label_for[target]))

    # Copy the return value (BEAM x0) into the IR's HALT-result reg.
    builder.emit(_OP_MOVE, _x(0), _y(_REG_HALT_RESULT))


def _emit_return(
    builder: _Builder,
    instr: IrInstruction,
    *,
    y_reg_count: int,
) -> None:
    """Lower RET as: copy y1 → x0; deallocate K; return."""
    if instr.operands:
        msg = f"RET takes no operands, got {len(instr.operands)}"
        raise BEAMBackendError(msg)
    builder.emit(_OP_MOVE, _y(_REG_HALT_RESULT), _x(0))
    builder.emit(_OP_DEALLOCATE, _u(y_reg_count))
    builder.emit(_OP_RETURN)


_HANDLERS: Final[set[IrOp]] = {
    IrOp.LOAD_IMM,
    IrOp.ADD,
    IrOp.SUB,
    IrOp.MUL,
    IrOp.DIV,
    IrOp.ADD_IMM,
    IrOp.RET,
    IrOp.CALL,
    IrOp.JUMP,
    IrOp.BRANCH_Z,
    IrOp.BRANCH_NZ,
    IrOp.CMP_EQ,
    IrOp.CMP_LT,
    IrOp.CMP_GT,
    IrOp.LABEL,  # internal labels in body
}


def _supported_op_summary() -> str:
    return ", ".join(sorted(op.name for op in _HANDLERS))


# ---------------------------------------------------------------------------
# Module-info synthesis — unchanged from the BEAM01 v1
# ---------------------------------------------------------------------------


def _emit_module_info_pair(
    builder: _Builder,
    *,
    module_atom_idx: int,
    module_info_atom_idx: int,
    label_module_info_0: int,
    label_module_info_1: int,
    label_after_func_info_0: int,
) -> None:
    erlang_atom_idx = builder.atoms.add("erlang")
    gmi_atom_idx = builder.atoms.add("get_module_info")
    gmi1_import = builder.imports.add(erlang_atom_idx, gmi_atom_idx, 1)
    gmi2_import = builder.imports.add(erlang_atom_idx, gmi_atom_idx, 2)

    builder.emit(
        _OP_FUNC_INFO,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.A, module_info_atom_idx),
        _u(0),
    )
    builder.emit(_OP_LABEL, _u(label_module_info_0))
    builder.emit(_OP_MOVE, BEAMOperand(BEAMTag.A, module_atom_idx), _x(0))
    builder.emit(_OP_CALL_EXT_ONLY, _u(1), _u(gmi1_import - 1))

    builder.emit(_OP_LABEL, _u(label_after_func_info_0))
    builder.emit(
        _OP_FUNC_INFO,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.A, module_info_atom_idx),
        _u(1),
    )
    builder.emit(_OP_LABEL, _u(label_module_info_1))
    builder.emit(_OP_MOVE, _x(0), _x(1))
    builder.emit(_OP_MOVE, BEAMOperand(BEAMTag.A, module_atom_idx), _x(0))
    builder.emit(_OP_CALL_EXT_ONLY, _u(2), _u(gmi2_import - 1))


# ---------------------------------------------------------------------------
# Top-level entry point
# ---------------------------------------------------------------------------


def lower_ir_to_beam(
    program: IrProgram,
    config: BEAMBackendConfig,
) -> BEAMModule:
    """Lower ``program`` into a ``BEAMModule`` ready for encoding."""
    if not config.module_name:
        raise BEAMBackendError("BEAMBackendConfig.module_name must not be empty")

    builder = _Builder(
        atoms=_AtomTable.starting_with_module(config.module_name),
        imports=_ImportTable.empty(),
    )
    module_atom_idx = 1  # by construction, atoms[0] == config.module_name
    module_info_atom_idx = builder.atoms.add("module_info")

    callable_names = _discover_callable_names(program)
    regions = _split_callable_regions(program, callable_names)
    if not regions:
        raise BEAMBackendError(
            "IrProgram contains no callable regions; need at least one LABEL"
        )

    # Pre-pass 1: y-register count for every function body.
    # Simplest: one program-wide value (the highest IR register
    # index used anywhere + 1).  Wasteful for small functions but
    # correct.
    y_reg_count = (
        config.y_register_count
        if config.y_register_count is not None
        else _max_register_index(program) + 1
    )

    # Pre-pass 2: allocate a BEAM label number for every IR label.
    # Callable regions get TWO each (one for the func_info opcode,
    # one for the call target).  Internal labels get ONE each.
    func_info_label_for: dict[str, int] = {}
    label_for: dict[str, int] = {}
    arity_for: dict[str, int] = {}
    for name, _body in regions:
        func_info_label_for[name] = builder.fresh_label()
        label_for[name] = builder.fresh_label()
        arity_for[name] = config.arity_overrides.get(name, 0)

    # Internal labels — found inside region bodies.
    for _name, body in regions:
        for instr in body:
            if instr.opcode is IrOp.LABEL:
                internal_name = _operand_label(
                    instr.operands[0], role="internal LABEL"
                )
                if internal_name not in label_for:
                    label_for[internal_name] = builder.fresh_label()

    # Pass 3: emit each region's prologue + body + epilogue.
    for name, body in regions:
        function_atom_idx = builder.atoms.add(name)
        builder.emit(_OP_LABEL, _u(func_info_label_for[name]))
        builder.emit(
            _OP_FUNC_INFO,
            BEAMOperand(BEAMTag.A, module_atom_idx),
            BEAMOperand(BEAMTag.A, function_atom_idx),
            _u(arity_for[name]),
        )
        builder.emit(_OP_LABEL, _u(label_for[name]))

        # Function entry: allocate y-register frame + copy args
        # from x-registers into the Twig param slots.
        builder.emit(_OP_ALLOCATE, _u(y_reg_count), _u(arity_for[name]))
        for i in range(arity_for[name]):
            builder.emit(_OP_MOVE, _x(i), _y(_REG_PARAM_BASE + i))

        # Body.
        for instr in body:
            _emit_body_instruction(
                builder,
                instr,
                label_for=label_for,
                arity_for=arity_for,
                y_reg_count=y_reg_count,
            )

        # Each user region is exported (Twig has no module-private
        # functions in v1).
        builder.exports.append(
            BEAMExport(
                function_atom_index=function_atom_idx,
                arity=arity_for[name],
                label=label_for[name],
            )
        )

    # Synthesise module_info/0 and module_info/1.
    label_func_info_mi_0 = builder.fresh_label()
    label_mi_0 = builder.fresh_label()
    label_func_info_mi_1 = builder.fresh_label()
    label_mi_1 = builder.fresh_label()

    builder.emit(_OP_LABEL, _u(label_func_info_mi_0))
    _emit_module_info_pair(
        builder,
        module_atom_idx=module_atom_idx,
        module_info_atom_idx=module_info_atom_idx,
        label_module_info_0=label_mi_0,
        label_module_info_1=label_mi_1,
        label_after_func_info_0=label_func_info_mi_1,
    )

    builder.exports.append(
        BEAMExport(
            function_atom_index=module_info_atom_idx,
            arity=0,
            label=label_mi_0,
        )
    )
    builder.exports.append(
        BEAMExport(
            function_atom_index=module_info_atom_idx,
            arity=1,
            label=label_mi_1,
        )
    )

    builder.emit(_OP_INT_CODE_END)

    _BERT_EMPTY_LIST: bytes = b"\x83\x6a"
    return BEAMModule(
        name=config.module_name,
        atoms=builder.atoms.as_tuple(),
        instructions=tuple(builder.instructions),
        imports=builder.imports.as_tuple(),
        exports=tuple(builder.exports),
        locals_=(),
        label_count=builder.next_label,
        max_opcode=_MIN_RUNTIME_MAX_OPCODE,
        extra_chunks=(
            ("Attr", _BERT_EMPTY_LIST),
            ("CInf", _BERT_EMPTY_LIST),
        ),
    )


def _emit_body_instruction(
    builder: _Builder,
    instr: IrInstruction,
    *,
    label_for: dict[str, int],
    arity_for: dict[str, int],
    y_reg_count: int,
) -> None:
    op = instr.opcode
    if op not in _HANDLERS:
        msg = (
            f"unsupported IR op {op.name} for BEAM TW03 Phase 1 — "
            f"supported ops: {_supported_op_summary()}"
        )
        raise BEAMBackendError(msg)

    if op is IrOp.LABEL:
        _emit_label_in_body(builder, instr, label_for)
        return
    if op is IrOp.LOAD_IMM:
        _emit_load_imm(builder, instr)
        return
    if op in (IrOp.ADD, IrOp.SUB, IrOp.MUL, IrOp.DIV):
        _emit_arithmetic(builder, instr)
        return
    if op is IrOp.ADD_IMM:
        _emit_add_imm(builder, instr)
        return
    if op is IrOp.JUMP:
        _emit_jump(builder, instr, label_for)
        return
    if op in (IrOp.BRANCH_Z, IrOp.BRANCH_NZ):
        _emit_branch(builder, instr, label_for)
        return
    if op in (IrOp.CMP_EQ, IrOp.CMP_LT, IrOp.CMP_GT):
        _emit_cmp(builder, instr)
        return
    if op is IrOp.CALL:
        _emit_call(builder, instr, label_for, arity_for)
        return
    if op is IrOp.RET:
        _emit_return(builder, instr, y_reg_count=y_reg_count)
        return
    msg = f"internal: missing handler dispatch for {op.name}"
    raise BEAMBackendError(msg)
