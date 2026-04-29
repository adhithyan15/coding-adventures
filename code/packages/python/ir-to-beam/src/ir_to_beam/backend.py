"""Lower a ``compiler-ir`` ``IrProgram`` into a ``BEAMModule``.

This is BEAM01 Phase 3 — see
``code/specs/BEAM01-twig-on-real-erl.md``.

Pipeline
========

The package consumes already-lowered ``IrProgram`` objects (the
output of ``ir-optimizer`` or, for tests, of a hand-built
program).  It produces a ``BEAMModule`` ready for
``beam_bytecode_encoder.encode_beam`` to serialize.

Calling convention
==================

BEAM is register-based with two register files:

* ``x`` registers — function arguments and scratch (caller-saves).
  Args to a call go in ``x0..x{arity-1}``; the return value lives
  in ``x0``.
* ``y`` registers — stack-allocated locals (callee-saves).  Live
  across calls; require an ``allocate``/``deallocate`` framing
  pair.

Our v1 lowering uses **only** ``x`` registers and emits no
``allocate``/``deallocate`` instructions.  This is correct for the
BEAM01 Phase 3 op set (``LABEL``, ``LOAD_IMM``, ``ADD``, ``SUB``,
``MUL``, ``DIV``, ``CALL``, ``RET``) because none of them need
values to survive a call boundary — argument registers are set
right before the ``call`` and read right after on return.

When ``BRANCH``/``JUMP`` and tail-recursive ``call_only`` join the
op set (v2), proper ``allocate`` framing becomes necessary.

Module shape
============

For an ``IrProgram`` with N callable regions:

```
{label, 1}.
{func_info, {atom, mod}, {atom, fn1}, arity}.
{label, 2}.   <--- call-target label for fn1
... fn1 body ...
{return}.
{label, 3}.
{func_info, {atom, mod}, {atom, fn2}, arity}.
{label, 4}.
... fn2 body ...
{return}.
... module_info/0 + module_info/1 ...
{int_code_end}.
```

Real ``erlc`` always inserts ``module_info/0`` and
``module_info/1`` exports that delegate to
``erlang:get_module_info/{1,2}`` — the BEAM loader rejects modules
without those two exports.  We do the same.
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
_OP_CALL_EXT_ONLY: Final[int] = 78  # tail-call to imported BIF
_OP_RETURN: Final[int] = 19
_OP_MOVE: Final[int] = 64
_OP_GC_BIF2: Final[int] = 125

# Minimum ``max_opcode`` value the modern Erlang loader accepts.
# The loader uses this field as a "what BEAM dialect was this
# module compiled against" declaration, not as a real "highest
# opcode used" value.  OTP 25 introduced opcode 178 (``call_fun2``)
# and rejects anything that doesn't advertise at least that level
# with the cryptic "compiled for an old version of the runtime
# system" error.  We always declare 178 even though our v1 op set
# tops out at 125; that's exactly what ``erlc`` does for modules
# that don't actually use the OTP-25-specific opcodes.
_MIN_RUNTIME_MAX_OPCODE: Final[int] = 178

# Erlang BIFs we may need.  Keys are ``(module_atom, fn_atom, arity)``,
# values are display names — used for atom-table population only.
_BIF_PLUS: Final[tuple[str, str, int]] = ("erlang", "+", 2)
_BIF_MINUS: Final[tuple[str, str, int]] = ("erlang", "-", 2)
_BIF_MUL: Final[tuple[str, str, int]] = ("erlang", "*", 2)
_BIF_DIV: Final[tuple[str, str, int]] = ("erlang", "div", 2)
_BIF_GET_MODULE_INFO_1: Final[tuple[str, str, int]] = ("erlang", "get_module_info", 1)
_BIF_GET_MODULE_INFO_2: Final[tuple[str, str, int]] = ("erlang", "get_module_info", 2)

_ARITHMETIC_BIF: dict[IrOp, tuple[str, str, int]] = {
    IrOp.ADD: _BIF_PLUS,
    IrOp.SUB: _BIF_MINUS,
    IrOp.MUL: _BIF_MUL,
    IrOp.DIV: _BIF_DIV,
}


class BEAMBackendError(ValueError):
    """Raised when an IR program cannot be lowered to a BEAM module."""


@dataclass(frozen=True)
class BEAMBackendConfig:
    """Knobs for the lowering.

    ``module_name`` is the Erlang module name (must be a valid
    atom).  ``arity_overrides`` lets the caller declare the arity
    of specific entry points; if not present, arity defaults to 0
    for every region (matching how Twig top-level functions and
    the synthesised ``_start`` look in IR before TW04 lands an
    explicit arity per region).
    """

    module_name: str
    arity_overrides: dict[str, int] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Lowering driver
# ---------------------------------------------------------------------------


@dataclass
class _AtomTable:
    """Insertion-ordered atom table that maps to 1-based BEAM indices.

    Atom 1 is the module name (BEAM mandates this).  Atoms 2+ are
    inserted lazily as the lowering runs into them.
    """

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
    """Insertion-ordered ``ImpT`` row builder.

    Each unique ``(module, function, arity)`` triple gets a 1-based
    import index.  The ``BEAMImport`` row references atom-table
    indices, so callers must make sure those atoms are added first.
    """

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


def _split_callable_regions(
    program: IrProgram,
) -> list[tuple[str, list[IrInstruction]]]:
    """Group an IR instruction stream into ``(label_name, body)`` regions.

    Each region begins at an ``IrOp.LABEL`` and ends at the
    instruction before the next ``IrOp.LABEL`` (or end-of-stream).
    The region's body excludes the leading LABEL itself.
    """
    regions: list[tuple[str, list[IrInstruction]]] = []
    current_name: str | None = None
    current_body: list[IrInstruction] = []
    for instr in program.instructions:
        if instr.opcode is IrOp.LABEL:
            if current_name is not None:
                regions.append((current_name, current_body))
            label_arg = instr.operands[0]
            if not isinstance(label_arg, IrLabel):
                msg = (
                    "LABEL instruction must carry an IrLabel operand; "
                    f"got {label_arg!r}"
                )
                raise BEAMBackendError(msg)
            current_name = label_arg.name
            current_body = []
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


def _operand_register(value: object, *, role: str) -> int:
    if not isinstance(value, IrRegister):
        msg = f"expected IR register for {role}, got {value!r}"
        raise BEAMBackendError(msg)
    if value.index < 0:
        msg = f"register index must be non-negative, got {value.index}"
        raise BEAMBackendError(msg)
    return value.index


def _operand_immediate(value: object, *, role: str) -> int:
    """Pull an integer out of an IR immediate operand.

    The compiler-ir module exposes immediates as either a bare
    Python int or as a small ``IrImmediate`` dataclass — handle
    both and reject everything else with a clear message.
    """
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
# Per-instruction lowering
# ---------------------------------------------------------------------------


def _emit_load_imm(builder: _Builder, instr: IrInstruction) -> None:
    if len(instr.operands) != 2:
        raise BEAMBackendError(f"LOAD_IMM expects 2 operands, got {len(instr.operands)}")
    dest = _operand_register(instr.operands[0], role="LOAD_IMM dest")
    value = _operand_immediate(instr.operands[1], role="LOAD_IMM value")
    if value < 0:
        msg = (
            f"LOAD_IMM with negative integer ({value}) is not yet supported "
            "by ir-to-beam — the BEAM compact-term encoder rejects negatives"
        )
        raise BEAMBackendError(msg)
    builder.emit(
        _OP_MOVE,
        BEAMOperand(BEAMTag.I, value),
        BEAMOperand(BEAMTag.X, dest),
    )


def _emit_arithmetic(builder: _Builder, instr: IrInstruction) -> None:
    """Lower ADD/SUB/MUL/DIV via ``gc_bif2``.

    ``gc_bif2`` operands: fail-label, live-regs, bif-import-idx,
    src1, src2, dest.  We pass fail-label = 0 (no failure handler;
    integer arithmetic on integer operands cannot fail under BEAM
    semantics) and live = 0 (we don't allocate any y-registers in
    v1, so no live state to preserve across the BIF call).
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

    bif_module, bif_name, bif_arity = _ARITHMETIC_BIF[instr.opcode]
    module_atom_idx = builder.atoms.add(bif_module)
    fn_atom_idx = builder.atoms.add(bif_name)
    bif_import_idx = builder.imports.add(module_atom_idx, fn_atom_idx, bif_arity)

    builder.emit(
        _OP_GC_BIF2,
        BEAMOperand(BEAMTag.F, 0),                # fail label = 0 (no failure)
        BEAMOperand(BEAMTag.U, 0),                # live regs = 0
        BEAMOperand(BEAMTag.U, bif_import_idx - 1),  # BIF index is 0-based here
        BEAMOperand(BEAMTag.X, lhs),
        BEAMOperand(BEAMTag.X, rhs),
        BEAMOperand(BEAMTag.X, dest),
    )


def _emit_call(
    builder: _Builder,
    instr: IrInstruction,
    label_for: dict[str, int],
    arity_for: dict[str, int],
) -> None:
    target = _operand_label(instr.operands[0], role="CALL target")
    if target not in label_for:
        msg = (
            f"CALL target {target!r} has no corresponding LABEL — "
            "missing region in the IR"
        )
        raise BEAMBackendError(msg)
    arity = arity_for.get(target, 0)
    builder.emit(
        _OP_CALL,
        BEAMOperand(BEAMTag.U, arity),
        BEAMOperand(BEAMTag.F, label_for[target]),
    )


def _emit_return(builder: _Builder, instr: IrInstruction) -> None:
    if instr.operands:
        msg = f"RET takes no operands, got {len(instr.operands)}"
        raise BEAMBackendError(msg)
    builder.emit(_OP_RETURN)


_HANDLERS: dict[IrOp, str] = {
    IrOp.LOAD_IMM: "_emit_load_imm",
    IrOp.ADD: "_emit_arithmetic",
    IrOp.SUB: "_emit_arithmetic",
    IrOp.MUL: "_emit_arithmetic",
    IrOp.DIV: "_emit_arithmetic",
    IrOp.RET: "_emit_return",
    IrOp.CALL: "_emit_call",
}


def _supported_op_summary() -> str:
    return ", ".join(sorted(name for op in _HANDLERS for name in [op.name]))


# ---------------------------------------------------------------------------
# Module-info synthesis
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
    """Emit ``module_info/0`` and ``module_info/1`` mirrors of
    what ``erlc`` produces.

    These functions exist for the runtime's introspection
    (``M:module_info()`` and ``M:module_info(Key)``).  Their bodies
    are simply tail-calls to ``erlang:get_module_info/{1,2}`` with
    the module-name atom prepended.
    """
    # Atom indices for the BIF imports.
    erlang_atom_idx = builder.atoms.add("erlang")
    gmi_atom_idx = builder.atoms.add("get_module_info")
    gmi1_import = builder.imports.add(erlang_atom_idx, gmi_atom_idx, 1)
    gmi2_import = builder.imports.add(erlang_atom_idx, gmi_atom_idx, 2)

    # module_info/0 — func_info, label, move atom into x0, tail-call to gmi/1.
    builder.emit(
        _OP_FUNC_INFO,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.A, module_info_atom_idx),
        BEAMOperand(BEAMTag.U, 0),
    )
    builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, label_module_info_0))
    builder.emit(
        _OP_MOVE,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.X, 0),
    )
    builder.emit(
        _OP_CALL_EXT_ONLY,
        BEAMOperand(BEAMTag.U, 1),
        BEAMOperand(BEAMTag.U, gmi1_import - 1),
    )

    # module_info/1 — func_info, label, move-args, tail-call gmi/2.
    builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, label_after_func_info_0))
    builder.emit(
        _OP_FUNC_INFO,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.A, module_info_atom_idx),
        BEAMOperand(BEAMTag.U, 1),
    )
    builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, label_module_info_1))
    builder.emit(
        _OP_MOVE,
        BEAMOperand(BEAMTag.X, 0),
        BEAMOperand(BEAMTag.X, 1),
    )
    builder.emit(
        _OP_MOVE,
        BEAMOperand(BEAMTag.A, module_atom_idx),
        BEAMOperand(BEAMTag.X, 0),
    )
    builder.emit(
        _OP_CALL_EXT_ONLY,
        BEAMOperand(BEAMTag.U, 2),
        BEAMOperand(BEAMTag.U, gmi2_import - 1),
    )
    # No trailing label — ``erlc`` reference output ends
    # ``module_info/1`` with the ``call_ext_only`` opcode and the
    # caller (``lower_ir_to_beam``) appends ``int_code_end``
    # directly after.


# ---------------------------------------------------------------------------
# Top-level entry point
# ---------------------------------------------------------------------------


def lower_ir_to_beam(
    program: IrProgram,
    config: BEAMBackendConfig,
) -> BEAMModule:
    """Lower ``program`` into a ``BEAMModule`` ready for encoding.

    Raises ``BEAMBackendError`` for any structurally invalid IR or
    any IR opcode that isn't yet supported in v1.  See
    ``_HANDLERS`` for the supported set.
    """
    if not config.module_name:
        raise BEAMBackendError("BEAMBackendConfig.module_name must not be empty")

    builder = _Builder(
        atoms=_AtomTable.starting_with_module(config.module_name),
        imports=_ImportTable.empty(),
    )
    module_atom_idx = 1  # by construction, atoms[0] == config.module_name
    module_info_atom_idx = builder.atoms.add("module_info")

    regions = _split_callable_regions(program)
    if not regions:
        raise BEAMBackendError(
            "IrProgram contains no callable regions; need at least one LABEL"
        )

    # First pass: allocate one BEAM label per region's call entry,
    # plus one preceding label for the func_info opcode.  Real
    # ``erlc`` puts func_info BEFORE the call-target label so error
    # tracebacks can find the function definition.
    label_for: dict[str, int] = {}
    func_info_label_for: dict[str, int] = {}
    arity_for: dict[str, int] = {}
    for name, _body in regions:
        func_info_label_for[name] = builder.fresh_label()
        label_for[name] = builder.fresh_label()
        arity_for[name] = config.arity_overrides.get(name, 0)

    # Second pass: emit each region's prologue + body.
    for name, body in regions:
        function_atom_idx = builder.atoms.add(name)
        builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, func_info_label_for[name]))
        builder.emit(
            _OP_FUNC_INFO,
            BEAMOperand(BEAMTag.A, module_atom_idx),
            BEAMOperand(BEAMTag.A, function_atom_idx),
            BEAMOperand(BEAMTag.U, arity_for[name]),
        )
        builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, label_for[name]))
        for instr in body:
            handler_name = _HANDLERS.get(instr.opcode)
            if handler_name is None:
                msg = (
                    f"unsupported IR op {instr.opcode.name} for BEAM v1 — "
                    f"supported ops: {_supported_op_summary()}"
                )
                raise BEAMBackendError(msg)
            if handler_name == "_emit_load_imm":
                _emit_load_imm(builder, instr)
            elif handler_name == "_emit_arithmetic":
                _emit_arithmetic(builder, instr)
            elif handler_name == "_emit_call":
                _emit_call(builder, instr, label_for, arity_for)
            elif handler_name == "_emit_return":
                _emit_return(builder, instr)
            else:  # pragma: no cover — _HANDLERS entries are exhaustive
                msg = f"internal: missing handler dispatch for {handler_name}"
                raise BEAMBackendError(msg)

        # Each user region is exported (Twig has no module-private
        # functions in v1).
        builder.exports.append(
            BEAMExport(
                function_atom_index=function_atom_idx,
                arity=arity_for[name],
                label=label_for[name],
            )
        )

    # Synthesise module_info/0 and module_info/1 after the user
    # functions, before the final int_code_end.
    label_func_info_mi_0 = builder.fresh_label()
    label_mi_0 = builder.fresh_label()
    label_func_info_mi_1 = builder.fresh_label()
    label_mi_1 = builder.fresh_label()

    # Re-shape: the func_info-label/main-label pairs come BEFORE the
    # func_info opcode for each module_info function.  Using the
    # label numbers we just minted, emit the prologue + tail-calls.
    builder.emit(_OP_LABEL, BEAMOperand(BEAMTag.U, label_func_info_mi_0))
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

    # Terminator.
    builder.emit(_OP_INT_CODE_END)

    # Empty Attr / CInf BERT terms — real ``erlc`` always emits
    # these chunks, and Erlang OTP's loader probes for them on
    # some load paths.  An empty proplist in the External Term
    # Format is two bytes: ``83`` (version 131) + ``6a`` (NIL).
    _BERT_EMPTY_LIST: bytes = b"\x83\x6a"

    return BEAMModule(
        name=config.module_name,
        atoms=builder.atoms.as_tuple(),
        instructions=tuple(builder.instructions),
        imports=builder.imports.as_tuple(),
        exports=tuple(builder.exports),
        locals_=(),
        label_count=builder.next_label,  # 1 past the last allocated
        # See ``_MIN_RUNTIME_MAX_OPCODE`` — must be >= 178 for OTP
        # 25+ runtimes, which is the minimum modern Erlang we
        # support.
        max_opcode=_MIN_RUNTIME_MAX_OPCODE,
        extra_chunks=(
            ("Attr", _BERT_EMPTY_LIST),
            ("CInf", _BERT_EMPTY_LIST),
        ),
    )
