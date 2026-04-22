"""Tetrad bytecode → JIT IR translator (TET05).

Walks a Tetrad ``CodeObject`` and produces a list of ``IRInstr`` in SSA form.

The Tetrad VM is accumulator-based: every expression result lands in *acc* and
most binary operations use a scratch register.  The translator lifts this flat
bytecode into SSA by maintaining:

- ``acc``     — name of the SSA variable currently in the accumulator
- ``regs[r]`` — name of the SSA variable currently in register slot *r*

Each ``LDA_*`` instruction creates a fresh virtual variable.  ``STA_REG r``
updates ``regs[r]`` without emitting IR.  ``STA_VAR i`` emits ``store_var``.
Jump offsets are resolved to label names pre-scanned in a first pass.
"""

from __future__ import annotations

from tetrad_compiler.bytecode import CodeObject, Op

from tetrad_jit.ir import IRInstr

__all__ = ["translate", "TranslationError"]


class TranslationError(Exception):
    """Raised when a bytecode sequence cannot be lifted to JIT IR."""


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

_ARITH_NAME: dict[int, str] = {
    Op.ADD: "add",
    Op.SUB: "sub",
    Op.MUL: "mul",
    Op.DIV: "div",
    Op.MOD: "mod",
}

_BIT_NAME: dict[int, str] = {
    Op.AND: "and",
    Op.OR:  "or",
    Op.XOR: "xor",
    Op.SHL: "shl",
    Op.SHR: "shr",
}

_CMP_NAME: dict[int, str] = {
    Op.EQ:  "cmp_eq",
    Op.NEQ: "cmp_ne",
    Op.LT:  "cmp_lt",
    Op.LTE: "cmp_le",
    Op.GT:  "cmp_gt",
    Op.GTE: "cmp_ge",
}


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def translate(code: CodeObject) -> list[IRInstr]:
    """Translate a Tetrad ``CodeObject`` to a JIT IR instruction list.

    The function parameters are pre-loaded as ``param`` IR instructions.
    Every ``LDA_*`` emits a fresh SSA variable.  Jumps become ``jmp`` /
    ``jz`` / ``jnz`` referencing pre-assigned label names.

    Parameters
    ----------
    code:
        Compiled Tetrad function (not the top-level ``<main>`` object).

    Returns
    -------
    list[IRInstr]:
        SSA IR ready for optimization passes and code generation.

    Raises
    ------
    TranslationError:
        If an unknown opcode is encountered.
    """
    instrs = code.instructions

    # ------------------------------------------------------------------
    # Pre-pass: assign label names to every jump target instruction index.
    # ------------------------------------------------------------------
    jump_targets: dict[int, str] = {}
    lbl_ctr: list[int] = [0]

    for i, instr in enumerate(instrs):
        if instr.opcode in (Op.JMP, Op.JZ, Op.JNZ, Op.JMP_LOOP):
            offset = instr.operands[0]
            target = i + 1 + offset
            if 0 <= target < len(instrs) and target not in jump_targets:
                jump_targets[target] = f"lbl_{lbl_ctr[0]}"
                lbl_ctr[0] += 1

    # ------------------------------------------------------------------
    # Translation state
    # ------------------------------------------------------------------
    ctr: list[int] = [0]

    def new_var() -> str:
        v = f"v{ctr[0]}"
        ctr[0] += 1
        return v

    # acc tracks the SSA variable currently in the accumulator.
    acc = "_undef"
    # regs tracks SSA variables in Tetrad's virtual registers (R0–R7).
    num_regs = max(code.register_count, 8)
    regs: list[str] = ["_undef"] * num_regs

    ir: list[IRInstr] = []

    def emit(op: str, dst: str | None, srcs: list, ty: str, comment: str = "") -> None:
        ir.append(IRInstr(op=op, dst=dst, srcs=srcs, ty=ty, comment=comment))

    # ------------------------------------------------------------------
    # Emit param instructions — each argument is pre-placed in regs[i].
    # ------------------------------------------------------------------
    for i, pname in enumerate(code.params):
        v = new_var()
        emit("param", v, [i], "u8", f"arg {pname}")
        regs[i] = v

    # ------------------------------------------------------------------
    # Main translation loop
    # ------------------------------------------------------------------
    for ip, instr in enumerate(instrs):
        op = instr.opcode
        ops = instr.operands

        # Emit a label marker before any instruction that is a jump target.
        if ip in jump_targets:
            emit("label", None, [jump_targets[ip]], "")

        # -- Loads -------------------------------------------------------
        if op == Op.LDA_IMM:
            v = new_var()
            emit("const", v, [ops[0]], "u8")
            acc = v

        elif op == Op.LDA_ZERO:
            v = new_var()
            emit("const", v, [0], "u8")
            acc = v

        elif op == Op.LDA_REG:
            r = ops[0]
            acc = regs[r] if r < len(regs) else "_undef"

        elif op == Op.LDA_VAR:
            v = new_var()
            emit("load_var", v, [ops[0]], "u8")
            acc = v

        # -- Stores ------------------------------------------------------
        elif op == Op.STA_REG:
            r = ops[0]
            if r < len(regs):
                regs[r] = acc

        elif op == Op.STA_VAR:
            emit("store_var", None, [ops[0], acc], "u8")

        # -- Arithmetic --------------------------------------------------
        elif op in _ARITH_NAME:
            r = ops[0]
            ty = "u8" if len(ops) == 1 else "unknown"
            v = new_var()
            emit(_ARITH_NAME[op], v, [acc, regs[r]], ty)
            acc = v

        elif op == Op.ADD_IMM:
            v = new_var()
            emit("add", v, [acc, ops[0]], "u8")
            acc = v

        elif op == Op.SUB_IMM:
            v = new_var()
            emit("sub", v, [acc, ops[0]], "u8")
            acc = v

        # -- Bitwise -----------------------------------------------------
        elif op in _BIT_NAME:
            r = ops[0]
            v = new_var()
            emit(_BIT_NAME[op], v, [acc, regs[r]], "u8")
            acc = v

        elif op == Op.AND_IMM:
            v = new_var()
            emit("and", v, [acc, ops[0]], "u8")
            acc = v

        elif op == Op.NOT:
            v = new_var()
            emit("not", v, [acc], "u8")
            acc = v

        # -- Comparisons -------------------------------------------------
        elif op in _CMP_NAME:
            r = ops[0]
            ty = "u8" if len(ops) == 1 else "unknown"
            v = new_var()
            emit(_CMP_NAME[op], v, [acc, regs[r]], ty)
            acc = v

        # -- Logical helpers ---------------------------------------------
        elif op == Op.LOGICAL_NOT:
            v = new_var()
            emit("logical_not", v, [acc], "u8")
            acc = v

        elif op in (Op.LOGICAL_AND, Op.LOGICAL_OR):
            r = ops[0]
            op_name = "and" if op == Op.LOGICAL_AND else "or"
            v = new_var()
            emit(op_name, v, [acc, regs[r]], "u8")
            acc = v

        # -- Control flow -----------------------------------------------
        elif op in (Op.JMP, Op.JMP_LOOP):
            target = ip + 1 + ops[0]
            lbl = jump_targets.get(target, f"lbl_missing_{target}")
            emit("jmp", None, [lbl], "")

        elif op == Op.JZ:
            target = ip + 1 + ops[0]
            lbl = jump_targets.get(target, f"lbl_missing_{target}")
            emit("jz", None, [acc, lbl], "")

        elif op == Op.JNZ:
            target = ip + 1 + ops[0]
            lbl = jump_targets.get(target, f"lbl_missing_{target}")
            emit("jnz", None, [acc, lbl], "")

        # -- Calls -------------------------------------------------------
        elif op == Op.CALL:
            func_idx, argc = ops[0], ops[1]
            args = [regs[i] for i in range(argc)]
            v = new_var()
            emit("call", v, [func_idx] + args, "u8")
            acc = v

        # -- Return / halt -----------------------------------------------
        elif op == Op.RET:
            emit("ret", None, [acc], "u8")

        elif op == Op.IO_IN:
            v = new_var()
            emit("io_in", v, [], "u8")
            acc = v

        elif op == Op.IO_OUT:
            emit("io_out", None, [acc], "u8")

        elif op == Op.HALT:
            break

        else:
            raise TranslationError(
                f"unknown opcode 0x{op:02X} at instruction {ip}"
            )

    return ir
