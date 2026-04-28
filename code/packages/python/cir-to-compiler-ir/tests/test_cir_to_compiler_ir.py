"""Tests for cir_to_compiler_ir — CIR-to-IrProgram lowering bridge.

Coverage strategy
-----------------
Every test builds a small ``list[CIRInstr]``, calls ``lower_cir_to_ir_program()``,
and inspects the resulting ``IrProgram``.

Conventions
-----------
- All CIR instruction lists end with ``ret_void`` so the program has a
  proper terminal instruction (HALT).  This also exercises the ``ret_void``
  lowering rule in every test.
- The entry label is ``"_start"`` (the default) unless a test explicitly
  overrides it.
- Register indices are verified by index (0, 1, …) rather than relying on
  variable ordering, so tests are robust to future changes in Pass 1 ordering.

Round-trip tests at the bottom verify that lowered ``IrProgram``s pass
the WASM and JVM validators unchanged — ensuring that any program lowered
from CIR can actually be compiled by the existing backends.
"""

from __future__ import annotations

import pytest
from codegen_core import CIRInstr
from compiler_ir import IrImmediate, IrLabel, IrOp, IrRegister

from cir_to_compiler_ir import (
    CIRLoweringError,
    lower_cir_to_ir_program,
    validate_cir_for_lowering,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _instrs(prog):
    """Return the IrOp list for every instruction in the program."""
    return [i.opcode for i in prog.instructions]


def _ops(*opcodes: IrOp) -> list[IrOp]:
    """Build the expected opcode list with the entry LABEL prepended."""
    return [IrOp.LABEL, *opcodes]


def _reg(prog, idx: int) -> IrRegister:
    """Look up an IrRegister by its index in the program's operands."""
    return IrRegister(index=idx)


# ---------------------------------------------------------------------------
# 1. const_{int_type} → LOAD_IMM
# ---------------------------------------------------------------------------

class TestConstInt:
    def test_const_i32(self):
        instrs = [
            CIRInstr("const_i32", "x", [42], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == _ops(IrOp.LOAD_IMM, IrOp.HALT)
        load = prog.instructions[1]
        assert load.operands[0] == IrRegister(index=0)   # dest
        assert load.operands[1] == IrImmediate(value=42) # literal

    def test_const_u8(self):
        instrs = [
            CIRInstr("const_u8", "b", [255], "u8"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[1].opcode == IrOp.LOAD_IMM
        assert prog.instructions[1].operands[1] == IrImmediate(255)

    def test_const_bool_true(self):
        """True → IrImmediate(1)."""
        instrs = [
            CIRInstr("const_bool", "flag", [True], "bool"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[1].opcode == IrOp.LOAD_IMM
        assert prog.instructions[1].operands[1] == IrImmediate(1)

    def test_const_bool_false(self):
        """False → IrImmediate(0)."""
        instrs = [
            CIRInstr("const_bool", "flag", [False], "bool"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[1].opcode == IrOp.LOAD_IMM
        assert prog.instructions[1].operands[1] == IrImmediate(0)


# ---------------------------------------------------------------------------
# 2. const_f64 → LOAD_F64_IMM
# ---------------------------------------------------------------------------

class TestConstFloat:
    def test_const_f64(self):
        instrs = [
            CIRInstr("const_f64", "pi", [3.14], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[1].opcode == IrOp.LOAD_F64_IMM
        assert prog.instructions[1].operands[0] == IrRegister(0)
        from compiler_ir import IrFloatImmediate
        assert prog.instructions[1].operands[1] == IrFloatImmediate(3.14)


# ---------------------------------------------------------------------------
# 3. Integer arithmetic
# ---------------------------------------------------------------------------

class TestIntArithmetic:
    def _two_var_program(self, op_name: str, ir_op: IrOp, type_suffix: str = "i32"):
        instrs = [
            CIRInstr("const_i32", "x", [10], "i32"),
            CIRInstr("const_i32", "y", [3],  "i32"),
            CIRInstr(f"{op_name}_{type_suffix}", "z", ["x", "y"], type_suffix),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, ir_op, IrOp.HALT
        )
        op_instr = prog.instructions[3]
        assert op_instr.operands[0] == IrRegister(2)  # z is 3rd var
        assert op_instr.operands[1] == IrRegister(0)  # x is 1st
        assert op_instr.operands[2] == IrRegister(1)  # y is 2nd
        return prog

    def test_add_i32(self):
        self._two_var_program("add", IrOp.ADD)

    def test_sub_u8(self):
        self._two_var_program("sub", IrOp.SUB, "u8")

    def test_mul_i32(self):
        self._two_var_program("mul", IrOp.MUL)

    def test_div_i32(self):
        self._two_var_program("div", IrOp.DIV)

    def test_and_u32(self):
        self._two_var_program("and", IrOp.AND, "u32")

    def test_or_u64(self):
        self._two_var_program("or", IrOp.OR, "u64")

    def test_xor_i16(self):
        self._two_var_program("xor", IrOp.XOR, "i16")

    def test_not_i32(self):
        instrs = [
            CIRInstr("const_i32", "x", [5], "i32"),
            CIRInstr("not_i32", "y", ["x"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == _ops(IrOp.LOAD_IMM, IrOp.NOT, IrOp.HALT)
        not_instr = prog.instructions[2]
        assert not_instr.operands[0] == IrRegister(1)  # dest y
        assert not_instr.operands[1] == IrRegister(0)  # src x


# ---------------------------------------------------------------------------
# 4. Negation (synthesised: LOAD_IMM 0 + SUB)
# ---------------------------------------------------------------------------

class TestNegation:
    def test_neg_i32(self):
        """neg_i32 x → LOAD_IMM 0 into scratch, SUB dest scratch x."""
        instrs = [
            CIRInstr("const_i32", "x", [7], "i32"),
            CIRInstr("neg_i32",   "y", ["x"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Expected: LABEL LOAD_IMM(x) LOAD_IMM(0) SUB(y, scratch, x) HALT
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.SUB, IrOp.HALT
        )
        scratch_load = prog.instructions[2]
        assert scratch_load.operands[1] == IrImmediate(0)  # zero constant

        sub_instr = prog.instructions[3]
        # dest is y (register 1), scratch is register 2, src is x (register 0)
        assert sub_instr.operands[0] == IrRegister(1)  # y
        assert sub_instr.operands[2] == IrRegister(0)  # x

    def test_neg_f64(self):
        """neg_f64 → LOAD_F64_IMM 0.0 + F64_SUB."""
        instrs = [
            CIRInstr("const_f64", "x", [1.5], "f64"),
            CIRInstr("neg_f64",   "y", ["x"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == _ops(
            IrOp.LOAD_F64_IMM, IrOp.LOAD_F64_IMM, IrOp.F64_SUB, IrOp.HALT
        )


# ---------------------------------------------------------------------------
# 5. Float arithmetic
# ---------------------------------------------------------------------------

class TestFloatArithmetic:
    def test_f64_add(self):
        instrs = [
            CIRInstr("const_f64", "a", [1.0], "f64"),
            CIRInstr("const_f64", "b", [2.0], "f64"),
            CIRInstr("add_f64",   "c", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == _ops(
            IrOp.LOAD_F64_IMM, IrOp.LOAD_F64_IMM, IrOp.F64_ADD, IrOp.HALT
        )

    def test_f64_sub(self):
        instrs = [
            CIRInstr("const_f64", "a", [5.0], "f64"),
            CIRInstr("const_f64", "b", [3.0], "f64"),
            CIRInstr("sub_f64",   "c", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[3].opcode == IrOp.F64_SUB

    def test_f64_mul(self):
        instrs = [
            CIRInstr("const_f64", "a", [2.0], "f64"),
            CIRInstr("const_f64", "b", [3.0], "f64"),
            CIRInstr("mul_f64",   "c", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[3].opcode == IrOp.F64_MUL

    def test_f64_cmp_lt(self):
        instrs = [
            CIRInstr("const_f64",  "a", [1.0], "f64"),
            CIRInstr("const_f64",  "b", [2.0], "f64"),
            CIRInstr("cmp_lt_f64", "r", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[3].opcode == IrOp.F64_CMP_LT


# ---------------------------------------------------------------------------
# 6. Integer comparisons
# ---------------------------------------------------------------------------

class TestIntComparisons:
    def _cmp(self, rel: str, ir_op: IrOp, extra_ops: list[IrOp] | None = None):
        instrs = [
            CIRInstr("const_i32", "a", [5], "i32"),
            CIRInstr("const_i32", "b", [3], "i32"),
            CIRInstr(f"cmp_{rel}_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        expected_middle = [ir_op] + (extra_ops or [])
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, *expected_middle, IrOp.HALT
        )
        return prog

    def test_cmp_eq(self):
        self._cmp("eq", IrOp.CMP_EQ)

    def test_cmp_ne(self):
        self._cmp("ne", IrOp.CMP_NE)

    def test_cmp_lt(self):
        self._cmp("lt", IrOp.CMP_LT)

    def test_cmp_gt(self):
        self._cmp("gt", IrOp.CMP_GT)

    def test_cmp_le_synthesised(self):
        """cmp_le_i32 → CMP_GT + LOAD_IMM(0) + CMP_EQ (three instructions).

        IrOp.NOT is bitwise complement, not logical NOT.  Bitwise NOT of 0
        gives -1 (0xFFFFFFFF), not 1.  To get a correct 0/1 result we use:
            gt   = CMP_GT(a, b)      # → 0 or 1
            zero = LOAD_IMM(0)
            dest = CMP_EQ(gt, zero)  # → 1 if gt==0 (i.e. a≤b), else 0
        """
        instrs = [
            CIRInstr("const_i32",  "a", [1], "i32"),
            CIRInstr("const_i32",  "b", [2], "i32"),
            CIRInstr("cmp_le_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Instructions: LABEL, LOAD_IMM a, LOAD_IMM b, CMP_GT tmp, LOAD_IMM zero, CMP_EQ r, HALT
        cmp_gt_instr  = prog.instructions[3]
        load_zero     = prog.instructions[4]
        cmp_eq_instr  = prog.instructions[5]

        assert cmp_gt_instr.opcode == IrOp.CMP_GT
        assert cmp_gt_instr.operands[1] == IrRegister(0)  # a
        assert cmp_gt_instr.operands[2] == IrRegister(1)  # b

        assert load_zero.opcode == IrOp.LOAD_IMM
        assert load_zero.operands[1] == IrImmediate(0)

        assert cmp_eq_instr.opcode == IrOp.CMP_EQ
        # CMP_EQ dest = r (register 2), srcs = gt_scratch, zero_scratch
        assert cmp_eq_instr.operands[0] == IrRegister(2)  # r

    def test_cmp_ge_synthesised(self):
        """cmp_ge_i32 → CMP_LT + LOAD_IMM(0) + CMP_EQ (three instructions)."""
        instrs = [
            CIRInstr("const_i32",  "a", [5], "i32"),
            CIRInstr("const_i32",  "b", [3], "i32"),
            CIRInstr("cmp_ge_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        cmp_lt_instr = prog.instructions[3]
        load_zero    = prog.instructions[4]
        cmp_eq_instr = prog.instructions[5]

        assert cmp_lt_instr.opcode == IrOp.CMP_LT
        assert load_zero.opcode == IrOp.LOAD_IMM
        assert load_zero.operands[1] == IrImmediate(0)
        assert cmp_eq_instr.opcode == IrOp.CMP_EQ
        assert cmp_eq_instr.operands[0] == IrRegister(2)  # r

    def test_cmp_le_semantics(self):
        """Verify the truth table for synthesised cmp_le.

        cmp_le(a, b) should be:
          a <= b  →  1   (True)
          a >  b  →  0   (False)

        The lowering is: gt=CMP_GT(a,b); zero=LOAD_IMM(0); dest=CMP_EQ(gt,zero)
          a=1, b=2: CMP_GT=0 → CMP_EQ(0,0)=1 ✓  (1 ≤ 2 is true)
          a=2, b=2: CMP_GT=0 → CMP_EQ(0,0)=1 ✓  (2 ≤ 2 is true)
          a=3, b=2: CMP_GT=1 → CMP_EQ(1,0)=0 ✓  (3 ≤ 2 is false)

        We check the instruction structure — not runtime semantics — since
        we don't execute IR here.
        """
        instrs = [
            CIRInstr("const_i32",  "a", [1], "i32"),
            CIRInstr("const_i32",  "b", [2], "i32"),
            CIRInstr("cmp_le_i32", "r", ["a", "b"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        cmp_gt_instr = prog.instructions[3]
        load_zero    = prog.instructions[4]
        cmp_eq_instr = prog.instructions[5]

        assert cmp_gt_instr.opcode == IrOp.CMP_GT
        assert cmp_gt_instr.operands[1] == IrRegister(0)  # a
        assert cmp_gt_instr.operands[2] == IrRegister(1)  # b

        assert load_zero.opcode == IrOp.LOAD_IMM
        assert load_zero.operands[1] == IrImmediate(0)

        assert cmp_eq_instr.opcode == IrOp.CMP_EQ
        # CMP_EQ compares gt_scratch against zero_scratch
        gt_scratch   = cmp_gt_instr.operands[0]
        zero_scratch = load_zero.operands[0]
        assert cmp_eq_instr.operands[1] == gt_scratch
        assert cmp_eq_instr.operands[2] == zero_scratch


# ---------------------------------------------------------------------------
# 7. Float comparisons (all six — IrOp has direct variants)
# ---------------------------------------------------------------------------

class TestFloatComparisons:
    def test_f64_cmp_eq(self):
        instrs = [
            CIRInstr("const_f64",  "a", [1.0], "f64"),
            CIRInstr("const_f64",  "b", [1.0], "f64"),
            CIRInstr("cmp_eq_f64", "r", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[3].opcode == IrOp.F64_CMP_EQ

    def test_f64_cmp_le(self):
        instrs = [
            CIRInstr("const_f64",  "a", [1.0], "f64"),
            CIRInstr("const_f64",  "b", [2.0], "f64"),
            CIRInstr("cmp_le_f64", "r", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # f64 has a direct CMP_LE — no synthesisation needed
        assert prog.instructions[3].opcode == IrOp.F64_CMP_LE

    def test_f64_cmp_ge(self):
        instrs = [
            CIRInstr("const_f64",  "a", [2.0], "f64"),
            CIRInstr("const_f64",  "b", [1.0], "f64"),
            CIRInstr("cmp_ge_f64", "r", ["a", "b"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[3].opcode == IrOp.F64_CMP_GE


# ---------------------------------------------------------------------------
# 8. Control flow
# ---------------------------------------------------------------------------

class TestControlFlow:
    def test_label_and_jmp(self):
        instrs = [
            CIRInstr("label", None, ["loop_start"], "void"),
            CIRInstr("jmp",   None, ["loop_start"], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Entry LABEL + loop LABEL + JUMP
        assert _instrs(prog) == [IrOp.LABEL, IrOp.LABEL, IrOp.JUMP]
        # The loop LABEL at index 1 has id=-1 (pseudo-instruction)
        assert prog.instructions[1].id == -1
        # The JUMP at index 2 has a real id ≥ 0
        assert prog.instructions[2].operands[0] == IrLabel("loop_start")
        assert prog.instructions[2].id >= 0

    def test_jmp_has_label_operand(self):
        instrs = [
            CIRInstr("jmp", None, ["exit"], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        jump = prog.instructions[1]
        assert jump.opcode == IrOp.JUMP
        assert jump.operands[0] == IrLabel("exit")

    def test_branch_nz(self):
        instrs = [
            CIRInstr("const_bool", "cond", [True], "bool"),
            CIRInstr("jmp_if_true", None, ["cond", "target"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        branch = prog.instructions[2]
        assert branch.opcode == IrOp.BRANCH_NZ
        assert branch.operands[0] == IrRegister(0)     # cond variable
        assert branch.operands[1] == IrLabel("target")

    def test_branch_z(self):
        instrs = [
            CIRInstr("const_bool", "cond", [False], "bool"),
            CIRInstr("jmp_if_false", None, ["cond", "done"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        branch = prog.instructions[2]
        assert branch.opcode == IrOp.BRANCH_Z
        assert branch.operands[1] == IrLabel("done")

    def test_br_true_bool(self):
        instrs = [
            CIRInstr("const_bool", "c", [True], "bool"),
            CIRInstr("br_true_bool", None, ["c", "yes"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[2].opcode == IrOp.BRANCH_NZ

    def test_br_false_bool(self):
        instrs = [
            CIRInstr("const_bool", "c", [False], "bool"),
            CIRInstr("br_false_bool", None, ["c", "no"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[2].opcode == IrOp.BRANCH_Z


# ---------------------------------------------------------------------------
# 9. Call
# ---------------------------------------------------------------------------

class TestCall:
    def test_call(self):
        instrs = [
            CIRInstr("call", None, ["my_func"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        call = prog.instructions[1]
        assert call.opcode == IrOp.CALL
        assert call.operands[0] == IrLabel("my_func")


# ---------------------------------------------------------------------------
# 10. Return / halt
# ---------------------------------------------------------------------------

class TestReturn:
    def test_ret_void_emits_halt(self):
        instrs = [CIRInstr("ret_void", None, [], "void")]
        prog = lower_cir_to_ir_program(instrs)
        assert _instrs(prog) == [IrOp.LABEL, IrOp.HALT]

    def test_ret_i32_emits_halt(self):
        instrs = [
            CIRInstr("const_i32", "r", [0], "i32"),
            CIRInstr("ret_i32",   None, ["r"], "i32"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[-1].opcode == IrOp.HALT


# ---------------------------------------------------------------------------
# 11. Type guards → COMMENT
# ---------------------------------------------------------------------------

class TestTypeAssert:
    def test_type_assert_emits_comment(self):
        instrs = [
            CIRInstr("type_assert", None, ["x", "i32"], "void", deopt_to=0),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        comment = prog.instructions[1]
        assert comment.opcode == IrOp.COMMENT
        assert "type_assert" in str(comment.operands[0])
        assert "x" in str(comment.operands[0])
        assert "i32" in str(comment.operands[0])


# ---------------------------------------------------------------------------
# 11b. Memory access  (load_mem / store_mem)
# ---------------------------------------------------------------------------
#
# Brainfuck (and any byte-tape language) compiles to ``load_mem`` and
# ``store_mem`` against a single pointer operand — the data pointer.  These
# are passthrough ops in jit-core's specialiser (they keep their bare
# names; the type lives in CIRInstr.type) and lower to the static IR's
# three-operand byte-access ops with a synthesised ``base = 0`` register.

class TestMemoryAccess:
    def test_load_mem_u8_emits_zero_base_then_load_byte(self):
        instrs = [
            CIRInstr("const_u32", "ptr", [0], "u32"),
            CIRInstr("load_mem", "v", ["ptr"], "u8"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Expected:
        #   LABEL _start
        #   LOAD_IMM ptr=0           ; const_u32 ptr=0
        #   LOAD_IMM base_scratch=0  ; synthesised zero-base
        #   LOAD_BYTE v, base, ptr   ; v = mem[base + ptr]
        #   HALT
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.LOAD_BYTE, IrOp.HALT
        )
        load_byte = prog.instructions[3]
        # operands[0] = dest (v, register 1), operands[1] = base, operands[2] = offset (ptr)
        assert load_byte.operands[0] == IrRegister(index=1)  # dest = v
        assert load_byte.operands[2] == IrRegister(index=0)  # offset = ptr (first var)

    def test_store_mem_u8_emits_zero_base_then_store_byte(self):
        instrs = [
            CIRInstr("const_u32", "ptr", [0], "u32"),
            CIRInstr("const_u8",  "v",   [42], "u8"),
            CIRInstr("store_mem", None, ["ptr", "v"], "u8"),
            CIRInstr("ret_void",  None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Expected:
        #   LABEL _start
        #   LOAD_IMM ptr
        #   LOAD_IMM v
        #   LOAD_IMM base_scratch=0
        #   STORE_BYTE v, base, ptr
        #   HALT
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.STORE_BYTE, IrOp.HALT
        )
        store_byte = prog.instructions[4]
        # operands[0] = src value (v, register 1), [2] = offset (ptr, register 0)
        assert store_byte.operands[0] == IrRegister(index=1)  # src = v
        assert store_byte.operands[2] == IrRegister(index=0)  # offset = ptr

    def test_load_mem_with_immediate_pointer_materialises_into_scratch(self):
        # Constant folding could in principle leave a literal in srcs[0].
        # The lowering must materialise it into a register so LOAD_BYTE has
        # a register to read.
        instrs = [
            CIRInstr("load_mem", "v", [3], "u8"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # LOAD_IMM ptr_scratch=3; LOAD_IMM base=0; LOAD_BYTE v, base, ptr_scratch
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.LOAD_BYTE, IrOp.HALT
        )

    def test_store_mem_with_immediate_value_materialises_into_scratch(self):
        instrs = [
            CIRInstr("const_u32", "ptr", [0], "u32"),
            CIRInstr("store_mem", None, ["ptr", 99], "u8"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # const_u32 ptr; LOAD_IMM val_scratch=99; LOAD_IMM base=0; STORE_BYTE
        assert _instrs(prog) == _ops(
            IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.LOAD_IMM, IrOp.STORE_BYTE, IrOp.HALT
        )

    def test_load_mem_rejects_non_integer_type(self):
        instrs = [
            CIRInstr("load_mem", "x", ["ptr"], "f64"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        with pytest.raises(CIRLoweringError, match="load_mem"):
            lower_cir_to_ir_program(instrs)

    def test_store_mem_rejects_non_integer_type(self):
        instrs = [
            CIRInstr("store_mem", None, ["ptr", "v"], "any"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        with pytest.raises(CIRLoweringError, match="store_mem"):
            lower_cir_to_ir_program(instrs)


# ---------------------------------------------------------------------------
# 12. Error cases
# ---------------------------------------------------------------------------

class TestErrorCases:
    def test_call_runtime_raises(self):
        instrs = [CIRInstr("call_runtime", None, ["alloc_list"], "any")]
        with pytest.raises(CIRLoweringError, match="call_runtime"):
            lower_cir_to_ir_program(instrs)

    def test_io_in_raises(self):
        instrs = [CIRInstr("io_in", "x", [], "i32")]
        with pytest.raises(CIRLoweringError, match="io_in"):
            lower_cir_to_ir_program(instrs)

    def test_io_out_raises(self):
        instrs = [CIRInstr("io_out", None, ["x"], "void")]
        with pytest.raises(CIRLoweringError, match="io_out"):
            lower_cir_to_ir_program(instrs)

    def test_unknown_op_raises(self):
        instrs = [CIRInstr("frobnicate_i32", "x", ["y"], "i32")]
        with pytest.raises(CIRLoweringError, match="frobnicate_i32"):
            lower_cir_to_ir_program(instrs)

    def test_unknown_const_suffix_raises(self):
        """const_blob is not a known type suffix."""
        instrs = [CIRInstr("const_blob", "x", [42], "blob")]
        with pytest.raises(CIRLoweringError, match="blob"):
            lower_cir_to_ir_program(instrs)


# ---------------------------------------------------------------------------
# 13. Validator tests
# ---------------------------------------------------------------------------

class TestValidator:
    def test_validate_empty_list(self):
        errors = validate_cir_for_lowering([])
        assert errors == ["empty instruction list"]

    def test_validate_call_runtime(self):
        errors = validate_cir_for_lowering([
            CIRInstr("call_runtime", None, ["alloc"], "any")
        ])
        assert len(errors) == 1
        assert "call_runtime" in errors[0]
        assert "alloc" in errors[0]

    def test_validate_io_in(self):
        errors = validate_cir_for_lowering([
            CIRInstr("io_in", "x", [], "i32")
        ])
        assert len(errors) == 1
        assert "io_in" in errors[0]

    def test_validate_io_out(self):
        errors = validate_cir_for_lowering([
            CIRInstr("io_out", None, ["v"], "void")
        ])
        assert len(errors) == 1
        assert "io_out" in errors[0]

    def test_validate_any_type_on_arith(self):
        """add_any with type='any' is unresolved — validator must catch it."""
        errors = validate_cir_for_lowering([
            CIRInstr("add_any", "r", ["a", "b"], "any")
        ])
        assert len(errors) == 1
        assert "any" in errors[0]

    def test_validate_control_flow_any_type_ok(self):
        """jmp with type='any' is fine — it's a control-flow op."""
        errors = validate_cir_for_lowering([
            CIRInstr("jmp", None, ["somewhere"], "any")
        ])
        assert errors == []

    def test_validate_multiple_errors_collected(self):
        """All errors are returned together, not just the first."""
        errors = validate_cir_for_lowering([
            CIRInstr("call_runtime", None, ["rt1"], "any"),
            CIRInstr("io_in", "x", [], "i32"),
            CIRInstr("call_runtime", None, ["rt2"], "any"),
        ])
        assert len(errors) == 3  # 2 call_runtime + 1 io_in

    def test_validate_valid_list_returns_empty(self):
        errors = validate_cir_for_lowering([
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ])
        assert errors == []

    def test_lower_raises_on_invalid_list(self):
        """lower_cir_to_ir_program raises CIRLoweringError if validation fails."""
        with pytest.raises(CIRLoweringError, match="validation failed"):
            lower_cir_to_ir_program([])


# ---------------------------------------------------------------------------
# 14. Register reuse
# ---------------------------------------------------------------------------

class TestRegisterReuse:
    def test_same_variable_same_register(self):
        """The same variable name must map to the same IrRegister every time."""
        instrs = [
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("const_i32", "y", [2], "i32"),
            CIRInstr("add_i32",   "x", ["x", "y"], "i32"),  # x re-used as dest
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # First LOAD_IMM writes to reg 0 (x), second to reg 1 (y)
        load_x = prog.instructions[1]
        add_instr = prog.instructions[3]

        assert load_x.operands[0] == IrRegister(0)      # x first assigned here
        assert add_instr.operands[0] == IrRegister(0)   # x reused as dest
        assert add_instr.operands[1] == IrRegister(0)   # x reused as src0
        assert add_instr.operands[2] == IrRegister(1)   # y

    def test_different_names_different_registers(self):
        instrs = [
            CIRInstr("const_i32", "a", [1], "i32"),
            CIRInstr("const_i32", "b", [2], "i32"),
            CIRInstr("const_i32", "c", [3], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        regs = {
            i.operands[0]
            for i in prog.instructions
            if i.opcode == IrOp.LOAD_IMM
        }
        assert regs == {IrRegister(0), IrRegister(1), IrRegister(2)}


# ---------------------------------------------------------------------------
# 15. Entry label
# ---------------------------------------------------------------------------

class TestEntryLabel:
    def test_default_entry_label(self):
        instrs = [CIRInstr("ret_void", None, [], "void")]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.entry_label == "_start"
        assert prog.instructions[0].opcode == IrOp.LABEL
        assert prog.instructions[0].operands[0] == IrLabel("_start")
        assert prog.instructions[0].id == -1  # LABEL has id=-1

    def test_custom_entry_label(self):
        instrs = [CIRInstr("ret_void", None, [], "void")]
        prog = lower_cir_to_ir_program(instrs, entry_label="main")
        assert prog.entry_label == "main"
        assert prog.instructions[0].operands[0] == IrLabel("main")

    def test_entry_label_is_first_instruction(self):
        """Entry LABEL must be the very first instruction in the program."""
        instrs = [
            CIRInstr("const_i32", "x", [99], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        assert prog.instructions[0].opcode == IrOp.LABEL


# ---------------------------------------------------------------------------
# 16. Instruction IDs
# ---------------------------------------------------------------------------

class TestInstructionIds:
    def test_real_instructions_have_non_negative_ids(self):
        instrs = [
            CIRInstr("const_i32", "x", [1], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        real = [i for i in prog.instructions if i.opcode != IrOp.LABEL]
        for instr in real:
            assert instr.id >= 0

    def test_label_instructions_have_minus_one_id(self):
        instrs = [
            CIRInstr("label", None, ["loop"], "void"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        labels = [i for i in prog.instructions if i.opcode == IrOp.LABEL]
        for lbl in labels:
            assert lbl.id == -1


# ---------------------------------------------------------------------------
# 17. Round-trip: CIR → IrProgram → backend validator
# ---------------------------------------------------------------------------

class TestTetradMove:
    """tetrad.move — Tetrad VM register-to-register copy instruction.

    The JIT specialiser emits ``tetrad.move`` to copy a value from one
    virtual register to another.  This op is not part of the stable CIR
    opcode set but appears frequently in JIT-specialised output (e.g., for
    the 6 * 7 multiply which needs three register assignments).

    Lowering strategy: ``ADD_IMM dest, src, 0`` (MOV via add-zero).
    When src is a literal, ``LOAD_IMM`` is used instead.
    """

    def test_tetrad_move_register_to_register(self):
        """tetrad.move with a variable source emits ADD_IMM dst, src, 0."""
        instrs = [
            CIRInstr("const_i32", "x", [42], "i32"),
            CIRInstr("tetrad.move", "y", ["x"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Instruction 1 = entry LABEL "_start"
        # Instruction 2 = LOAD_IMM x
        # Instruction 3 = ADD_IMM y, x, 0  (the tetrad.move)
        # Instruction 4 = HALT
        move_instr = prog.instructions[2]
        assert move_instr.opcode == IrOp.ADD_IMM
        dst = move_instr.operands[0]
        src = move_instr.operands[1]
        imm = move_instr.operands[2]
        assert isinstance(dst, IrRegister)
        assert isinstance(src, IrRegister)
        assert isinstance(imm, IrImmediate)
        assert imm.value == 0

    def test_tetrad_move_preserves_value(self):
        """The moved value is accessible in the destination register."""
        instrs = [
            CIRInstr("const_i32", "a", [99], "i32"),
            CIRInstr("tetrad.move", "b", ["a"], "i32"),
            CIRInstr("tetrad.move", "c", ["b"], "i32"),  # chain of moves
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        # Should compile without error (all instructions valid ADD_IMM ops)
        opcodes = [ins.opcode for ins in prog.instructions]
        assert IrOp.ADD_IMM in opcodes

    def test_tetrad_move_literal_source_emits_load_imm(self):
        """tetrad.move with a literal integer source emits LOAD_IMM."""
        instrs = [
            CIRInstr("tetrad.move", "dst", [7], "i32"),  # src is int literal
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        move_instr = prog.instructions[1]  # after LABEL
        assert move_instr.opcode == IrOp.LOAD_IMM
        imm = move_instr.operands[1]
        assert isinstance(imm, IrImmediate)
        assert imm.value == 7

    def test_tetrad_move_wasm_round_trip(self):
        """Programs with tetrad.move pass WASM validation (all ops supported)."""
        from ir_to_wasm_compiler import validate_for_wasm

        instrs = [
            CIRInstr("const_i32", "a", [6], "i32"),
            CIRInstr("tetrad.move", "b", ["a"], "i32"),
            CIRInstr("ret_void", None, [], "void"),
        ]
        prog = lower_cir_to_ir_program(instrs)
        errors = validate_for_wasm(prog)
        assert errors == [], f"WASM validation failed after tetrad.move: {errors}"


class TestRoundTrip:
    """Verify that lowered programs pass the WASM and JVM validators.

    These tests are the integration smoke-tests for the full pipeline:
    CIR → IrProgram → backend-specific validator.

    If the validator returns an empty list, the IrProgram is structurally
    correct for that backend and can be compiled to assembly.
    """

    def _simple_program(self):
        """A minimal CIR program: add two integers and halt."""
        return [
            CIRInstr("const_i32", "x", [40], "i32"),
            CIRInstr("const_i32", "y", [2],  "i32"),
            CIRInstr("add_i32",   "z", ["x", "y"], "i32"),
            CIRInstr("ret_void",  None, [], "void"),
        ]

    def test_round_trip_wasm(self):
        """CIR → IrProgram → validate_for_wasm() == []."""
        from ir_to_wasm_compiler import validate_for_wasm
        prog = lower_cir_to_ir_program(self._simple_program())
        errors = validate_for_wasm(prog)
        assert errors == [], f"WASM validation failed: {errors}"

    def test_round_trip_jvm(self):
        """CIR → IrProgram → validate_for_jvm() == []."""
        from ir_to_jvm_class_file import validate_for_jvm
        prog = lower_cir_to_ir_program(self._simple_program())
        errors = validate_for_jvm(prog)
        assert errors == [], f"JVM validation failed: {errors}"
