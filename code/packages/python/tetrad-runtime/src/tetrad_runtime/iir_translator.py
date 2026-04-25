"""Tetrad-bytecode → InterpreterIR translator.

This module is the bridge between the legacy ``tetrad-compiler`` (which
emits Tetrad-specific ``CodeObject`` / ``Instruction`` / ``Op``) and the
generic LANG pipeline (``IIRModule`` / ``IIRFunction`` / ``IIRInstr``).

Mental model
------------
Tetrad is an **accumulator machine** with eight register slots and a flat
variable namespace.  IIR is **SSA-by-name** — every value lives in a named
slot in the per-frame register file.

The translation models the accumulator as a single SSA name, ``_acc``,
that is reused everywhere (since it is conceptually a single physical
register, not many distinct values).  Likewise three scratch SSA names are
reused for short pieces of compound translations:

- ``_t``  — temporary value (the "other" operand on binary ops)
- ``_b``  — bool result of a comparison before casting back to u8
- ``_a0``, ``_a1``, … — argument values being marshalled for a CALL

Tetrad's eight register slots map to the lowest eight slots of the IIR
``RegisterFile`` and are referenced positionally through ``load_reg`` /
``store_reg`` with the literal index.

Variables
---------
Tetrad has a flat ``var_names`` list per function.  After compilation:

- For ``main``: every entry in ``var_names`` is a **global** introduced
  via ``GlobalDecl``.
- For each sub-function: every entry is either a **parameter** (first
  ``len(params)``) or a **let-bound local**.

Locals stay inside the IIR frame's named-register file: we emit a
custom ``tetrad.move`` opcode (a typed copy with no side effects) to
move values between ``_acc`` and the named slot.

Globals live in a process-wide dict that the runtime maintains; access
goes through builtins ``__get_global`` and ``__set_global`` which the
runtime registers on the VM.

Branches and labels
-------------------
Tetrad branches use byte offsets relative to the next instruction.  IIR
uses **named labels**.  The translator does a first pass over the
function's instructions, computes the absolute target index of every
branch, and assigns each unique target an ``L<idx>`` label name.  The
second pass emits a ``label L<idx>`` instruction before any instruction
that is a branch target.

The reason this works: vm-core's ``label`` opcode is a no-op at runtime,
so inserting them anywhere is safe.

Truthiness conversion
---------------------
Tetrad comparisons return u8 (0 or 1).  IIR comparisons return ``bool``.
We use ``cast _acc, [_b, "u8"], "u8"`` to convert back so subsequent
arithmetic on the comparison result works exactly as in Tetrad.

Why this design over a custom Tetrad opcode set
-----------------------------------------------
Using standard IIR opcodes (``add``, ``cmp_eq``, ``jmp_if_true``, …) means
``jit-core`` can specialise the IIR via its existing passes, ``aot-core``
can compile it, and any future LANG tool (debugger, profiler, LSP) sees
familiar opcodes.  Tetrad-specific extensions are limited to two:

- ``tetrad.move`` — typed copy ``dest := resolve(src)`` (used for local
  variable reads/writes that are not register-indexed).
- (No others are needed; I/O and globals go through builtins.)
"""

from __future__ import annotations

from collections.abc import Iterable

from interpreter_ir import IIRFunction, IIRInstr, IIRModule
from interpreter_ir.function import FunctionTypeStatus as IIRFunctionTypeStatus
from tetrad_compiler.bytecode import CodeObject, Instruction, Op
from tetrad_type_checker.types import FunctionTypeStatus as TetradFunctionTypeStatus

__all__ = ["code_object_to_iir", "TETRAD_OPCODE_EXTENSIONS", "ENTRY_FN_NAME"]

# Names of the SSA slots used internally by translation output.
_ACC = "_acc"
_TMP = "_t"
_BOOL = "_b"

# Tetrad's eight register slots are named SSA variables in the IIR — *not*
# vm-core's positional register file.  This avoids the aliasing bug where
# ``store_reg [0, _acc]`` would write to whatever slot ``_acc`` happens to
# occupy in vm-core's name_to_reg.  Named slots are guaranteed distinct.
def _reg_name(i: int) -> str:
    return f"_r{i}"

# Static type used everywhere — Tetrad is u8-only.
_U8 = "u8"
_BOOL_T = "bool"

# Name of the synthetic top-level wrapper.  Tetrad's "<main>" CodeObject
# only initialises globals (per the compiler) and falls into a HALT — it
# does NOT call the user's `fn main()`.  The legacy TetradJIT wraps user's
# `fn main()` explicitly to make `execute_with_jit` produce the result the
# user expects.  ``code_object_to_iir`` follows the same pattern: the
# top-level becomes a wrapper that runs global initialisers, calls the
# user's main if it exists, and returns whatever that returns.
ENTRY_FN_NAME = "__entry__"


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def code_object_to_iir(
    main: CodeObject, *, module_name: str = "tetrad-program"
) -> IIRModule:
    """Translate a fully-compiled Tetrad ``CodeObject`` to an ``IIRModule``.

    Parameters
    ----------
    main:
        The top-level ``CodeObject`` returned by ``tetrad_compiler.compile_program``.
        Its ``functions`` list contains the user-defined functions.
    module_name:
        Human-readable module name (typically the source file path).

    Returns
    -------
    IIRModule
        A module whose ``entry_point`` is :data:`ENTRY_FN_NAME` — a
        synthetic wrapper that runs global initialisers and (if present)
        calls the user's ``main`` function.  All user-defined functions
        appear in the module's ``functions`` list verbatim, translated
        one-for-one.

    The returned module also has an attribute ``tetrad_globals`` (a
    ``list[str]``) added dynamically so the runtime can map memory
    addresses back to global names for ``globals_snapshot``.
    """
    iir_functions: list[IIRFunction] = []

    # The Tetrad "<main>" CodeObject holds top-level globals + a final HALT.
    # We translate it as the synthetic ENTRY_FN_NAME wrapper, then — if the
    # user defined `fn main()` — append a call to it before the final
    # ``ret`` so the wrapper's return value is the user's main's value.
    #
    # Each top-level global is assigned a stable address (its position in
    # main.var_names); IIR uses ``load_mem`` / ``store_mem`` against the
    # vm-core memory dict to access them.  See ``runtime.globals_snapshot``
    # for how the addresses are mapped back to names after a run.
    main_globals = list(main.var_names)
    globals_address = {name: idx for idx, name in enumerate(main_globals)}
    has_user_main = any(fn.name == "main" for fn in main.functions)

    iir_functions.append(
        _translate_function(
            code=main,
            iir_name=ENTRY_FN_NAME,
            is_main=True,
            globals_address=globals_address,
            sibling_functions=main.functions,
            append_user_main_call=has_user_main,
        )
    )

    # Each user-defined function is translated independently; sub-functions
    # never see globals (Tetrad's compiler rejects that at compile time), so
    # globals_address is empty for them.
    for fn in main.functions:
        iir_functions.append(
            _translate_function(
                code=fn,
                iir_name=fn.name,
                is_main=False,
                globals_address={},
                sibling_functions=main.functions,
                append_user_main_call=False,
            )
        )

    module = IIRModule(
        name=module_name,
        functions=iir_functions,
        entry_point=ENTRY_FN_NAME,
        language="tetrad",
    )
    # Attach the globals manifest for the runtime's globals_snapshot.
    # IIRModule does not have a typed slot for language-specific metadata,
    # so we use a dynamic attribute and document it on the public API.
    module.tetrad_globals = main_globals  # type: ignore[attr-defined]
    return module


# ---------------------------------------------------------------------------
# Per-function translation
# ---------------------------------------------------------------------------


def _translate_function(
    *,
    code: CodeObject,
    iir_name: str,
    is_main: bool,
    globals_address: dict[str, int],
    sibling_functions: list[CodeObject],
    append_user_main_call: bool,
) -> IIRFunction:
    """Translate one Tetrad ``CodeObject`` into one ``IIRFunction``.

    ``append_user_main_call`` is only ever True for the synthetic
    ``__entry__`` wrapper.  It tells the translator to drop the trailing
    HALT, splice in a call to the user's ``main``, and ``ret`` the result.

    Populates two LANG17 PR4 side tables on the returned function:

    - ``source_map``: ``(iir_index, tetrad_ip, 0)`` per translated Tetrad
      instruction, so ``TetradRuntime`` can re-project ``branch_profile``
      / ``loop_iterations`` lookups from Tetrad-IP to IIR-IP.
    - ``feedback_slots``: ``slot_index → iir_instr_index`` for every
      Tetrad instruction that carried a feedback slot.  The IIR index is
      the **value-producing** IIR instruction emitted for that Tetrad
      op (typically the last instruction in its translation), so the
      profiler's observation on that instr is what the slot's caller
      sees.
    """
    instructions = code.instructions
    branch_targets = _collect_branch_targets(instructions)

    iir_params: list[tuple[str, str]] = [(p, _U8) for p in code.params]

    body: list[IIRInstr] = []
    # LANG17 PR4 side tables, populated below.
    source_map: list[tuple[int, int, int]] = []
    feedback_slots: dict[int, int] = {}

    # Pre-bind the function's parameters into the named register slots
    # ``_r0`` … ``_r{N-1}``.  Tetrad's preamble does ``LDA_REG i`` to
    # read its own arguments, but vm-core's ``handle_call`` writes them
    # only to the *positional* register file slots that the param names
    # already occupy.  Without this copy, ``LDA_REG i`` would read an
    # uninitialised ``_r{i}`` slot.
    for i, (pname, _) in enumerate(iir_params):
        body.append(IIRInstr("tetrad.move", _reg_name(i), [pname], _U8))

    # Optionally drop the trailing HALT so we can splice in the call to
    # user-defined main and a `ret` instruction.  Tetrad's compile_checked
    # always appends a single HALT after all global initialisers; strip it
    # so we can replace it with a real function call + return.
    body_instructions = list(instructions)
    if (
        append_user_main_call
        and body_instructions
        and body_instructions[-1].opcode == Op.HALT
    ):
        body_instructions.pop()

    for ip, instr in enumerate(body_instructions):
        if ip in branch_targets:
            body.append(_label(branch_targets[ip]))

        # Record the IIR index where this Tetrad instruction's
        # translation begins — used by ``TetradRuntime.branch_profile``
        # / ``loop_iterations`` to re-project Tetrad-IP lookups.
        iir_start = len(body)
        source_map.append((iir_start, ip, 0))

        translated = _translate_instr(
            instr=instr,
            ip=ip,
            instructions=body_instructions,
            branch_targets=branch_targets,
            var_names=code.var_names,
            globals_address=globals_address,
            sibling_functions=sibling_functions,
        )
        body.extend(translated)

        # If this Tetrad instruction had a feedback slot, the
        # value-producing IIR instruction is the *last* one we just
        # emitted (translation produces the result-bearing op last).
        # Map slot index → that IIR instr index so
        # ``TetradRuntime.feedback_vector(fn)`` can reconstruct the
        # legacy slot-indexed list.
        slot_idx = _extract_slot_index(instr)
        if slot_idx is not None and translated:
            feedback_slots[slot_idx] = len(body) - 1

    if append_user_main_call:
        # Call user's main() with no args, return its result.
        body.append(IIRInstr("call", _ACC, ["main"], _U8))
        body.append(IIRInstr("ret", None, [_ACC], _U8))

    return IIRFunction(
        name=iir_name,
        params=iir_params,
        return_type=_U8 if (not is_main or append_user_main_call) else "void",
        instructions=body,
        register_count=max(code.register_count, 8, len(iir_params) or 1),
        type_status=_map_type_status(code.type_status),
        feedback_slots=feedback_slots,
        source_map=source_map,
    )


# Tetrad opcodes whose untyped variant carries a feedback slot in
# operand position 1.  Matches TET03's "two-path compilation" rule:
# arithmetic / comparison are slotted when the operands are not both
# statically u8.  The ADD_IMM and SUB_IMM optimisations are also
# slotted because the left operand may be untyped.
_SLOTTED_ARITH_OPS = frozenset({
    Op.ADD, Op.SUB, Op.MUL, Op.DIV, Op.MOD,
    Op.ADD_IMM, Op.SUB_IMM,
    Op.EQ, Op.NEQ, Op.LT, Op.LTE, Op.GT, Op.GTE,
})


def _extract_slot_index(instr: Instruction) -> int | None:
    """Return this Tetrad instruction's feedback-slot index, or ``None``.

    Three cases:
    - CALL always carries a slot at ``operands[2]``.
    - Slotted arith / comparison ops carry a slot at ``operands[1]``
      when their length is 2 (untyped path; typed path is single-operand).
    - Anything else has no slot.
    """
    if instr.opcode == Op.CALL:
        # operands = [func_idx, argc, slot]
        return instr.operands[2] if len(instr.operands) >= 3 else None
    if instr.opcode in _SLOTTED_ARITH_OPS and len(instr.operands) >= 2:
        return instr.operands[1]
    return None


def _map_type_status(status: TetradFunctionTypeStatus) -> IIRFunctionTypeStatus:
    """Map Tetrad's FunctionTypeStatus enum onto the IIR enum."""
    if status is TetradFunctionTypeStatus.FULLY_TYPED:
        return IIRFunctionTypeStatus.FULLY_TYPED
    if status is TetradFunctionTypeStatus.PARTIALLY_TYPED:
        return IIRFunctionTypeStatus.PARTIALLY_TYPED
    return IIRFunctionTypeStatus.UNTYPED


# ---------------------------------------------------------------------------
# Branch-target collection
# ---------------------------------------------------------------------------


def _collect_branch_targets(instructions: list[Instruction]) -> dict[int, str]:
    """Return ``{target_ip: label_name}`` for every branch destination.

    A "branch destination" is the absolute instruction index reached by any
    JMP / JZ / JNZ / JMP_LOOP; offsets are relative to ``ip + 1`` per the
    Tetrad spec.
    """
    targets: dict[int, str] = {}
    for ip, instr in enumerate(instructions):
        if instr.opcode in (Op.JMP, Op.JZ, Op.JNZ, Op.JMP_LOOP):
            offset = instr.operands[0]
            target = ip + 1 + offset
            if target not in targets:
                targets[target] = f"L{target}"
    return targets


# ---------------------------------------------------------------------------
# Per-instruction translation
# ---------------------------------------------------------------------------


def _translate_instr(
    *,
    instr: Instruction,
    ip: int,
    instructions: list[Instruction],
    branch_targets: dict[int, str],
    var_names: list[str],
    globals_address: dict[str, int],
    sibling_functions: list[CodeObject],
) -> list[IIRInstr]:
    """Translate one Tetrad instruction into one or more IIR instructions."""
    op = instr.opcode

    # ----- Loads --------------------------------------------------------
    if op == Op.LDA_IMM:
        return [IIRInstr("const", _ACC, [instr.operands[0]], _U8)]
    if op == Op.LDA_ZERO:
        return [IIRInstr("const", _ACC, [0], _U8)]
    if op == Op.LDA_REG:
        return [IIRInstr("tetrad.move", _ACC, [_reg_name(instr.operands[0])], _U8)]
    if op == Op.LDA_VAR:
        name = var_names[instr.operands[0]]
        if name in globals_address:
            return [
                IIRInstr("load_mem", _ACC, [globals_address[name]], _U8)
            ]
        return [IIRInstr("tetrad.move", _ACC, [name], _U8)]

    # ----- Stores -------------------------------------------------------
    if op == Op.STA_REG:
        return [IIRInstr("tetrad.move", _reg_name(instr.operands[0]), [_ACC], _U8)]
    if op == Op.STA_VAR:
        name = var_names[instr.operands[0]]
        if name in globals_address:
            return [
                IIRInstr("store_mem", None, [globals_address[name], _ACC], _U8)
            ]
        return [IIRInstr("tetrad.move", name, [_ACC], _U8)]

    # ----- Arithmetic ---------------------------------------------------
    if op in (Op.ADD, Op.SUB, Op.MUL, Op.DIV, Op.MOD):
        op_str = {
            Op.ADD: "add",
            Op.SUB: "sub",
            Op.MUL: "mul",
            Op.DIV: "div",
            Op.MOD: "mod",
        }[op]
        rname = _reg_name(instr.operands[0])
        return [IIRInstr(op_str, _ACC, [_ACC, rname], _U8)]
    if op == Op.ADD_IMM:
        return [IIRInstr("add", _ACC, [_ACC, instr.operands[0]], _U8)]
    if op == Op.SUB_IMM:
        return [IIRInstr("sub", _ACC, [_ACC, instr.operands[0]], _U8)]

    # ----- Bitwise ------------------------------------------------------
    if op in (Op.AND, Op.OR, Op.XOR, Op.SHL, Op.SHR):
        op_str = {
            Op.AND: "and",
            Op.OR: "or",
            Op.XOR: "xor",
            Op.SHL: "shl",
            Op.SHR: "shr",
        }[op]
        # SHL must mask to u8 to match Tetrad semantics; standard `shl`
        # in vm-core does not wrap.  The runtime registers a Tetrad-flavoured
        # `shl` that masks to 8 bits; see TETRAD_OPCODE_EXTENSIONS.
        rname = _reg_name(instr.operands[0])
        return [IIRInstr(op_str, _ACC, [_ACC, rname], _U8)]
    if op == Op.NOT:
        return [
            IIRInstr("not", _ACC, [_ACC], _U8),
            IIRInstr("and", _ACC, [_ACC, 0xFF], _U8),
        ]
    if op == Op.AND_IMM:
        return [IIRInstr("and", _ACC, [_ACC, instr.operands[0]], _U8)]

    # ----- Comparisons --------------------------------------------------
    if op in (Op.EQ, Op.NEQ, Op.LT, Op.LTE, Op.GT, Op.GTE):
        cmp_op = {
            Op.EQ: "cmp_eq",
            Op.NEQ: "cmp_ne",
            Op.LT: "cmp_lt",
            Op.LTE: "cmp_le",
            Op.GT: "cmp_gt",
            Op.GTE: "cmp_ge",
        }[op]
        rname = _reg_name(instr.operands[0])
        return [
            IIRInstr(cmp_op, _BOOL, [_ACC, rname], _BOOL_T),
            IIRInstr("cast", _ACC, [_BOOL, _U8], _U8),
        ]

    # ----- Logical ------------------------------------------------------
    if op == Op.LOGICAL_NOT:
        return [
            IIRInstr("cmp_eq", _BOOL, [_ACC, 0], _BOOL_T),
            IIRInstr("cast", _ACC, [_BOOL, _U8], _U8),
        ]
    if op == Op.LOGICAL_AND:
        rname = _reg_name(instr.operands[0])
        return [
            IIRInstr("cmp_ne", "_b1", [_ACC, 0], _BOOL_T),
            IIRInstr("cmp_ne", "_b2", [rname, 0], _BOOL_T),
            IIRInstr("and", _BOOL, ["_b1", "_b2"], _BOOL_T),
            IIRInstr("cast", _ACC, [_BOOL, _U8], _U8),
        ]
    if op == Op.LOGICAL_OR:
        rname = _reg_name(instr.operands[0])
        return [
            IIRInstr("cmp_ne", "_b1", [_ACC, 0], _BOOL_T),
            IIRInstr("cmp_ne", "_b2", [rname, 0], _BOOL_T),
            IIRInstr("or", _BOOL, ["_b1", "_b2"], _BOOL_T),
            IIRInstr("cast", _ACC, [_BOOL, _U8], _U8),
        ]

    # ----- Branches -----------------------------------------------------
    if op == Op.JMP or op == Op.JMP_LOOP:
        target = ip + 1 + instr.operands[0]
        return [IIRInstr("jmp", None, [branch_targets[target]], "any")]
    if op == Op.JZ:
        target = ip + 1 + instr.operands[0]
        return [
            IIRInstr("cmp_eq", _BOOL, [_ACC, 0], _BOOL_T),
            IIRInstr("jmp_if_true", None, [_BOOL, branch_targets[target]], "any"),
        ]
    if op == Op.JNZ:
        target = ip + 1 + instr.operands[0]
        return [
            IIRInstr("cmp_eq", _BOOL, [_ACC, 0], _BOOL_T),
            IIRInstr("jmp_if_false", None, [_BOOL, branch_targets[target]], "any"),
        ]

    # ----- Calls --------------------------------------------------------
    if op == Op.CALL:
        func_idx, argc, _slot = instr.operands
        callee = sibling_functions[func_idx]
        return list(_translate_call(callee_name=callee.name, argc=argc))
    if op == Op.RET:
        return [IIRInstr("ret", None, [_ACC], _U8)]

    # ----- I/O ----------------------------------------------------------
    if op == Op.IO_IN:
        return [IIRInstr("call_builtin", _ACC, ["__io_in"], _U8)]
    if op == Op.IO_OUT:
        return [IIRInstr("call_builtin", None, ["__io_out", _ACC], _U8)]

    # ----- VM control ---------------------------------------------------
    if op == Op.HALT:
        # vm-core has no HALT; ret_void terminates the current frame, and
        # since main is the root frame, the dispatch loop returns.
        return [IIRInstr("ret_void", None, [], "void")]

    raise ValueError(f"Tetrad opcode {op:#04x} has no IIR translation")


def _translate_call(*, callee_name: str, argc: int) -> Iterable[IIRInstr]:
    """Translate one CALL: marshal _r0.._r{argc-1} into the call's args.

    Tetrad's call convention places argument ``i`` in register ``i`` *before*
    CALL is executed.  In our IIR translation that means argument ``i`` is
    in the named slot ``_r{i}``.  vm-core's ``call`` opcode takes the
    callee name as ``srcs[0]`` and the arg sources as the remaining srcs;
    pass ``_r{i}`` directly — vm-core will resolve them at dispatch time.
    """
    arg_names = [_reg_name(i) for i in range(argc)]
    yield IIRInstr("call", _ACC, [callee_name, *arg_names], _U8)


def _label(name: str) -> IIRInstr:
    """Build a ``label`` instruction (no-op at runtime; branch target marker)."""
    return IIRInstr("label", None, [name], "any")


# ---------------------------------------------------------------------------
# Tetrad-specific opcode extensions
# ---------------------------------------------------------------------------
#
# vm-core accepts a dict of language-specific opcode handlers via the
# ``opcodes`` constructor parameter.  Tetrad needs exactly one extension:
# ``tetrad.move``, a typed copy that resolves srcs[0] from the current
# frame and assigns the value to dest.  This is the IIR analogue of
# Tetrad's "load var → acc; store acc → var" idiom for non-global locals.
#
# The handler signature matches every other vm-core opcode handler:
# ``(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any``.
# ---------------------------------------------------------------------------


def _handle_tetrad_move(vm, frame, instr):  # type: ignore[no-untyped-def]
    """Resolve ``srcs[0]`` to a value and assign to ``dest`` (typed copy)."""
    value = frame.resolve(instr.srcs[0])
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


# vm-core's standard ``shl`` does not mask to u8.  Tetrad's SHL does.
# Override shl in this opcode table to apply the u8 mask.  ``shr`` already
# matches Tetrad semantics under u8_wrap (zero fill, no mask needed).
def _handle_tetrad_shl(vm, frame, instr):  # type: ignore[no-untyped-def]
    """Logical shift left, masked to 8 bits to match Tetrad's u8 semantics."""
    a = frame.resolve(instr.srcs[0])
    n = frame.resolve(instr.srcs[1])
    result = (a << n) & 0xFF
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


TETRAD_OPCODE_EXTENSIONS = {
    "tetrad.move": _handle_tetrad_move,
    # Override standard `shl` to apply u8 mask.
    "shl": _handle_tetrad_shl,
}
