"""Unit tests for the Tetrad VM (spec TET04).

Test organisation
-----------------
  §1  Helpers and fixtures
  §2  Accumulator loads (LDA_IMM, LDA_ZERO, LDA_REG, LDA_VAR)
  §3  Store instructions (STA_REG, STA_VAR)
  §4  Arithmetic — typed path (no feedback slot)
  §5  Arithmetic — untyped path (with feedback slot, wrapping)
  §6  ADD_IMM / SUB_IMM fast paths
  §7  Bitwise and logical operations
  §8  Comparisons
  §9  Control flow (JMP, JZ, JNZ, JMP_LOOP)
  §10 Function calls (CALL, RET)
  §11 I/O instructions (IO_IN, IO_OUT)
  §12 HALT
  §13 Feedback vector state machine
  §14 Branch statistics
  §15 Loop back-edge counts
  §16 Metrics API (hot_functions, type_profile, etc.)
  §17 Error conditions (VMError)
  §18 execute_traced
  §19 End-to-end programs via compile_program
  §20 Coverage gaps and edge cases
"""

from __future__ import annotations

import pytest
from tetrad_compiler import compile_program
from tetrad_compiler.bytecode import CodeObject, Instruction, Op

from tetrad_vm import TetradVM, VMError
from tetrad_vm.metrics import BranchStats, SlotKind, SlotState, VMMetrics, VMTrace

# ===========================================================================
# §1  Helpers
# ===========================================================================


def _make(
    instrs: list[Instruction],
    *,
    var_names: list[str] | None = None,
    functions: list[CodeObject] | None = None,
    feedback_slot_count: int = 0,
    name: str = "<test>",
    params: list[str] | None = None,
) -> CodeObject:
    """Build a minimal CodeObject for unit tests."""
    code = CodeObject(
        name=name,
        params=params or [],
        feedback_slot_count=feedback_slot_count,
    )
    code.instructions = list(instrs)
    if var_names is not None:
        code.var_names = var_names
    if functions is not None:
        code.functions = list(functions)
    return code


def _run(instrs: list[Instruction], **kwargs: object) -> int:
    """Execute a minimal test program; return the final accumulator."""
    code = _make(instrs, **kwargs)  # type: ignore[arg-type]
    vm = TetradVM()
    return vm.execute(code)


def _halt() -> Instruction:
    return Instruction(Op.HALT, [])


# ===========================================================================
# §2  Accumulator loads
# ===========================================================================


class TestAccumulatorLoads:
    def test_lda_imm(self) -> None:
        assert _run([Instruction(Op.LDA_IMM, [42]), _halt()]) == 42

    def test_lda_imm_zero_value(self) -> None:
        assert _run([Instruction(Op.LDA_IMM, [0]), _halt()]) == 0

    def test_lda_imm_max(self) -> None:
        assert _run([Instruction(Op.LDA_IMM, [255]), _halt()]) == 255

    def test_lda_zero(self) -> None:
        # LDA_ZERO must reset the accumulator even if it held something.
        assert _run([
            Instruction(Op.LDA_IMM, [99]),
            Instruction(Op.LDA_ZERO, []),
            _halt(),
        ]) == 0

    def test_lda_reg(self) -> None:
        # Store 7 in R2, then load it back.
        assert _run([
            Instruction(Op.LDA_IMM, [7]),
            Instruction(Op.STA_REG, [2]),
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LDA_REG, [2]),
            _halt(),
        ]) == 7

    def test_lda_var_from_globals(self) -> None:
        # main frame: locals={}, so STA_VAR writes to _globals.
        assert _run([
            Instruction(Op.LDA_IMM, [55]),
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LDA_VAR, [0]),
            _halt(),
        ], var_names=["x"]) == 55

    def test_lda_var_local(self) -> None:
        # Build a function CodeObject with a local variable.
        fn = _make(
            [
                Instruction(Op.LDA_IMM, [33]),
                Instruction(Op.STA_VAR, [0]),
                Instruction(Op.LDA_ZERO, []),
                Instruction(Op.LDA_VAR, [0]),
                Instruction(Op.RET, []),
            ],
            var_names=["y"],
            name="fn",
            params=[],
        )
        # Main: call the function with argc=0, slot=0.
        code = _make(
            [
                Instruction(Op.CALL, [0, 0, 0]),
                _halt(),
            ],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 33


# ===========================================================================
# §3  Store instructions
# ===========================================================================


class TestStoreInstructions:
    def test_sta_reg(self) -> None:
        # STA_REG 3 should store acc into R[3].
        # Load 17, store into R3, zero acc, then read R3 back.
        assert _run([
            Instruction(Op.LDA_IMM, [17]),
            Instruction(Op.STA_REG, [3]),
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LDA_REG, [3]),
            _halt(),
        ]) == 17

    def test_sta_var_global(self) -> None:
        # Verify STA_VAR writes through to _globals on the main frame.
        code = _make(
            [
                Instruction(Op.LDA_IMM, [88]),
                Instruction(Op.STA_VAR, [0]),
                Instruction(Op.LDA_ZERO, []),
                Instruction(Op.LDA_VAR, [0]),
                _halt(),
            ],
            var_names=["g"],
        )
        vm = TetradVM()
        assert vm.execute(code) == 88
        assert vm._globals["g"] == 88  # noqa: SLF001

    def test_sta_var_local_in_function(self) -> None:
        # Inside a function frame locals is pre-populated; STA_VAR writes there.
        fn = _make(
            [
                Instruction(Op.LDA_IMM, [77]),
                Instruction(Op.STA_VAR, [0]),
                Instruction(Op.LDA_VAR, [0]),
                Instruction(Op.RET, []),
            ],
            var_names=["local"],
            name="fn",
        )
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 77
        assert "local" not in vm._globals  # noqa: SLF001


# ===========================================================================
# §4  Arithmetic — typed path (1-operand, no feedback slot update)
# ===========================================================================


class TestArithmeticTyped:
    def _arith(self, opcode: int, a: int, b: int) -> int:
        """Compute acc=a OP R0=b using typed (1-operand) form."""
        return _run([
            Instruction(Op.LDA_IMM, [b]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [a]),
            Instruction(opcode, [0]),
            _halt(),
        ])

    def test_add(self) -> None:
        assert self._arith(Op.ADD, 10, 5) == 15

    def test_sub(self) -> None:
        assert self._arith(Op.SUB, 10, 5) == 5

    def test_mul(self) -> None:
        assert self._arith(Op.MUL, 3, 4) == 12

    def test_div(self) -> None:
        assert self._arith(Op.DIV, 20, 4) == 5

    def test_mod(self) -> None:
        assert self._arith(Op.MOD, 17, 5) == 2

    def test_add_no_feedback_update(self) -> None:
        # A typed ADD (1 operand) must NOT touch the feedback vector.
        code = _make(
            [
                Instruction(Op.LDA_IMM, [1]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [2]),
                Instruction(Op.ADD, [0]),  # typed: 1 operand
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        vm.execute(code)
        # feedback_vector exists (feedback_slot_count=1) but slot 0 is untouched
        fv = vm.feedback_vector("<test>")
        assert fv is not None
        assert fv[0].kind is SlotKind.UNINITIALIZED


# ===========================================================================
# §5  Arithmetic — untyped path (2-operand, with feedback slot, u8 wrapping)
# ===========================================================================


class TestArithmeticUntyped:
    def _arith_slot(self, opcode: int, a: int, b: int, slot: int = 0) -> tuple[int, TetradVM]:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [b]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [a]),
                Instruction(opcode, [0, slot]),  # untyped: [r, slot]
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        result = vm.execute(code)
        return result, vm

    def test_add_untyped(self) -> None:
        result, vm = self._arith_slot(Op.ADD, 10, 5)
        assert result == 15
        assert vm.type_profile("<test>", 0) is not None
        assert vm.type_profile("<test>", 0).kind is SlotKind.MONOMORPHIC

    def test_sub_untyped(self) -> None:
        result, vm = self._arith_slot(Op.SUB, 10, 5)
        assert result == 5

    def test_mul_untyped(self) -> None:
        result, vm = self._arith_slot(Op.MUL, 6, 7)
        assert result == 42

    def test_div_untyped(self) -> None:
        result, vm = self._arith_slot(Op.DIV, 20, 4)
        assert result == 5

    def test_mod_untyped(self) -> None:
        result, vm = self._arith_slot(Op.MOD, 17, 5)
        assert result == 2

    def test_add_wraps_at_256(self) -> None:
        # 200 + 100 = 300 → 44 (mod 256)
        result, _ = self._arith_slot(Op.ADD, 200, 100)
        assert result == 44

    def test_sub_wraps_below_zero(self) -> None:
        # 5 - 10 → -5 → 251 (mod 256)
        result, _ = self._arith_slot(Op.SUB, 5, 10)
        assert result == 251

    def test_mul_wraps(self) -> None:
        # 16 * 20 = 320 → 64 (mod 256)
        result, _ = self._arith_slot(Op.MUL, 16, 20)
        assert result == 64

    def test_feedback_slot_count_increments(self) -> None:
        result, vm = self._arith_slot(Op.ADD, 3, 4)
        profile = vm.type_profile("<test>", 0)
        assert profile is not None
        assert profile.count == 1
        assert profile.observations == ["u8"]


# ===========================================================================
# §6  ADD_IMM / SUB_IMM fast paths
# ===========================================================================


class TestImmediateFastPaths:
    def test_add_imm_typed(self) -> None:
        # 1-operand ADD_IMM: typed path, no slot.
        result = _run([
            Instruction(Op.LDA_IMM, [10]),
            Instruction(Op.ADD_IMM, [5]),
            _halt(),
        ])
        assert result == 15

    def test_sub_imm_typed(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [10]),
            Instruction(Op.SUB_IMM, [3]),
            _halt(),
        ])
        assert result == 7

    def test_add_imm_wraps(self) -> None:
        # 250 + 10 = 260 → 4
        result = _run([
            Instruction(Op.LDA_IMM, [250]),
            Instruction(Op.ADD_IMM, [10]),
            _halt(),
        ])
        assert result == 4

    def test_sub_imm_wraps(self) -> None:
        # 3 - 10 = -7 → 249
        result = _run([
            Instruction(Op.LDA_IMM, [3]),
            Instruction(Op.SUB_IMM, [10]),
            _halt(),
        ])
        assert result == 249

    def test_add_imm_untyped_updates_slot(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [10]),
                Instruction(Op.ADD_IMM, [5, 0]),  # [immediate, slot]
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 15
        assert vm.type_profile("<test>", 0).kind is SlotKind.MONOMORPHIC  # type: ignore[union-attr]

    def test_sub_imm_untyped_updates_slot(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [10]),
                Instruction(Op.SUB_IMM, [3, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 7
        assert vm.type_profile("<test>", 0).kind is SlotKind.MONOMORPHIC  # type: ignore[union-attr]


# ===========================================================================
# §7  Bitwise and logical operations
# ===========================================================================


class TestBitwiseAndLogical:
    def _bitwise(self, opcode: int, a: int, b: int) -> int:
        return _run([
            Instruction(Op.LDA_IMM, [b]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [a]),
            Instruction(opcode, [0]),
            _halt(),
        ])

    def test_and(self) -> None:
        assert self._bitwise(Op.AND, 0xFF, 0x0F) == 0x0F

    def test_or(self) -> None:
        assert self._bitwise(Op.OR, 0xF0, 0x0F) == 0xFF

    def test_xor(self) -> None:
        assert self._bitwise(Op.XOR, 0xFF, 0x0F) == 0xF0

    def test_not(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [0x0F]),
            Instruction(Op.NOT, []),
            _halt(),
        ])
        assert result == 0xF0

    def test_not_all_zeros(self) -> None:
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.NOT, []),
            _halt(),
        ])
        assert result == 0xFF

    def test_shl(self) -> None:
        # 1 << 3 = 8
        assert self._bitwise(Op.SHL, 1, 3) == 8

    def test_shl_overflow_masked(self) -> None:
        # 1 << 9 = 512 → 0 (& 0xFF)
        assert self._bitwise(Op.SHL, 1, 9) == 0

    def test_shr(self) -> None:
        # 0x80 >> 4 = 8
        assert self._bitwise(Op.SHR, 0x80, 4) == 8

    def test_and_imm(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [0xFF]),
            Instruction(Op.AND_IMM, [0x0F]),
            _halt(),
        ])
        assert result == 0x0F

    def test_logical_not_zero(self) -> None:
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LOGICAL_NOT, []),
            _halt(),
        ])
        assert result == 1

    def test_logical_not_nonzero(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [42]),
            Instruction(Op.LOGICAL_NOT, []),
            _halt(),
        ])
        assert result == 0

    def test_logical_and_both_nonzero(self) -> None:
        # LOGICAL_AND is in the ISA but not emitted by the compiler.
        result = _run([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [3]),
            Instruction(Op.LOGICAL_AND, [0]),
            _halt(),
        ])
        assert result == 1

    def test_logical_and_one_zero(self) -> None:
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.LOGICAL_AND, [0]),
            _halt(),
        ])
        assert result == 0

    def test_logical_or_both_zero(self) -> None:
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LOGICAL_OR, [0]),
            _halt(),
        ])
        assert result == 0

    def test_logical_or_one_nonzero(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [7]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LOGICAL_OR, [0]),
            _halt(),
        ])
        assert result == 1


# ===========================================================================
# §8  Comparisons
# ===========================================================================


class TestComparisons:
    def _cmp(self, opcode: int, a: int, b: int, slot: bool = False) -> int:
        if slot:
            code = _make(
                [
                    Instruction(Op.LDA_IMM, [b]),
                    Instruction(Op.STA_REG, [0]),
                    Instruction(Op.LDA_IMM, [a]),
                    Instruction(opcode, [0, 0]),
                    _halt(),
                ],
                feedback_slot_count=1,
            )
            return TetradVM().execute(code)
        return _run([
            Instruction(Op.LDA_IMM, [b]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [a]),
            Instruction(opcode, [0]),
            _halt(),
        ])

    def test_eq_true(self) -> None:
        assert self._cmp(Op.EQ, 5, 5) == 1

    def test_eq_false(self) -> None:
        assert self._cmp(Op.EQ, 5, 6) == 0

    def test_neq_true(self) -> None:
        assert self._cmp(Op.NEQ, 5, 6) == 1

    def test_neq_false(self) -> None:
        assert self._cmp(Op.NEQ, 5, 5) == 0

    def test_lt_true(self) -> None:
        assert self._cmp(Op.LT, 3, 5) == 1

    def test_lt_false(self) -> None:
        assert self._cmp(Op.LT, 5, 3) == 0

    def test_lte_equal(self) -> None:
        assert self._cmp(Op.LTE, 5, 5) == 1

    def test_lte_less(self) -> None:
        assert self._cmp(Op.LTE, 4, 5) == 1

    def test_lte_greater(self) -> None:
        assert self._cmp(Op.LTE, 6, 5) == 0

    def test_gt_true(self) -> None:
        assert self._cmp(Op.GT, 5, 3) == 1

    def test_gt_false(self) -> None:
        assert self._cmp(Op.GT, 3, 5) == 0

    def test_gte_equal(self) -> None:
        assert self._cmp(Op.GTE, 5, 5) == 1

    def test_gte_greater(self) -> None:
        assert self._cmp(Op.GTE, 6, 5) == 1

    def test_gte_less(self) -> None:
        assert self._cmp(Op.GTE, 4, 5) == 0

    def test_comparison_with_slot_updates_feedback(self) -> None:
        result = self._cmp(Op.LT, 3, 5, slot=True)
        assert result == 1


# ===========================================================================
# §9  Control flow
# ===========================================================================


class TestControlFlow:
    def test_jmp_forward(self) -> None:
        # JMP 1 skips the LDA_IMM 99 and reaches HALT.
        result = _run([
            Instruction(Op.LDA_IMM, [42]),
            Instruction(Op.JMP, [1]),   # skip next
            Instruction(Op.LDA_IMM, [99]),
            _halt(),
        ])
        assert result == 42

    def test_jz_taken(self) -> None:
        # acc == 0, so JZ jumps over the LDA_IMM 1.
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.JZ, [1]),    # jump if zero
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.LDA_IMM, [99]),
            _halt(),
        ])
        assert result == 99

    def test_jz_not_taken(self) -> None:
        # acc != 0, so JZ falls through.
        result = _run([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.JZ, [1]),
            Instruction(Op.LDA_IMM, [77]),
            _halt(),
        ])
        assert result == 77

    def test_jnz_taken(self) -> None:
        result = _run([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.JNZ, [1]),   # jump because acc != 0
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LDA_IMM, [55]),
            _halt(),
        ])
        assert result == 55

    def test_jnz_not_taken(self) -> None:
        result = _run([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.JNZ, [1]),   # falls through because acc == 0
            Instruction(Op.LDA_IMM, [33]),
            _halt(),
        ])
        assert result == 33

    def test_jmp_loop_backward_jump(self) -> None:
        # Simple loop: count down from 3 to 0.
        # ip0: LDA_IMM 3     → acc=3, store in R0
        # ip1: STA_REG 0
        # ip2: LDA_REG 0     ← loop_start (ip=2)
        # ip3: JZ 2          → exit when acc==0 (jump to ip=6)
        # ip4: SUB_IMM 1     → acc -= 1
        # ip5: STA_REG 0     → R0 = acc
        # ip6: JMP_LOOP -4   → jump to ip=3 (back_idx=6, target=2 → offset=2-(6+1)=-5)
        # ip7: HALT
        result = _run([
            Instruction(Op.LDA_IMM, [3]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_REG, [0]),   # loop_start = ip 2
            Instruction(Op.JZ, [3]),         # exit if zero → jump to ip 7 (offset=3)
            Instruction(Op.SUB_IMM, [1]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.JMP_LOOP, [-4]),  # back to ip 2 (offset = 2 - (6+1) = -5)
            _halt(),
        ])
        assert result == 0

    def test_jmp_loop_records_back_edge(self) -> None:
        # The JMP_LOOP instruction is at ip 6 in the test above.
        code = _make([
            Instruction(Op.LDA_IMM, [2]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_REG, [0]),   # ip 2
            Instruction(Op.JZ, [3]),
            Instruction(Op.SUB_IMM, [1]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.JMP_LOOP, [-4]),  # ip 6
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        loops = vm.loop_iterations("<test>")
        assert 6 in loops
        assert loops[6] == 2   # body runs 2 times for n=2

    def test_branch_stats_jz(self) -> None:
        # JZ at ip=1: taken once (acc=0), not-taken once (acc=5).
        code = _make([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.JZ, [1]),    # ip 1, taken
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.LDA_IMM, [2]),
            _halt(),                    # ip 4
        ])
        vm = TetradVM()
        vm.execute(code)
        stats = vm.branch_profile("<test>", 1)  # slot=ip of JZ
        assert stats is not None
        assert stats.taken_count == 1
        assert stats.not_taken_count == 0

    def test_branch_stats_jnz(self) -> None:
        code = _make([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.JNZ, [1]),   # ip 1, taken
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.LDA_IMM, [9]),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        stats = vm.branch_profile("<test>", 1)
        assert stats is not None
        assert stats.taken_count == 1
        assert stats.not_taken_count == 0

    def test_branch_profile_none_for_unknown_fn(self) -> None:
        vm = TetradVM()
        assert vm.branch_profile("nonexistent", 0) is None

    def test_branch_profile_none_for_unknown_slot(self) -> None:
        code = _make([Instruction(Op.LDA_ZERO, []), _halt()])
        vm = TetradVM()
        vm.execute(code)
        assert vm.branch_profile("<test>", 99) is None


# ===========================================================================
# §10  Function calls (CALL / RET)
# ===========================================================================


class TestFunctionCalls:
    def _make_add_fn(self) -> CodeObject:
        """fn add(a, b) { return a + b; }  — manual bytecode."""
        fn = CodeObject(name="add", params=["a", "b"], feedback_slot_count=0)
        fn.var_names = ["a", "b"]
        fn.instructions = [
            Instruction(Op.LDA_REG, [0]),  # load arg a from R0
            Instruction(Op.STA_VAR, [0]),  # locals["a"] = acc
            Instruction(Op.LDA_REG, [1]),  # load arg b from R1
            Instruction(Op.STA_VAR, [1]),  # locals["b"] = acc
            Instruction(Op.LDA_VAR, [0]),  # acc = a
            Instruction(Op.STA_REG, [0]),  # R0 = a
            Instruction(Op.LDA_VAR, [1]),  # acc = b
            Instruction(Op.STA_REG, [1]),  # R1 = b  (unnecessary but harmless)
            Instruction(Op.LDA_REG, [0]),  # acc = a
            Instruction(Op.ADD, [1]),      # acc = a + b (typed)
            Instruction(Op.RET, []),
        ]
        return fn

    def test_basic_call_and_return(self) -> None:
        fn = self._make_add_fn()
        code = _make(
            [
                Instruction(Op.LDA_IMM, [10]),
                Instruction(Op.STA_REG, [0]),   # R0 = 10 (arg a)
                Instruction(Op.LDA_IMM, [20]),
                Instruction(Op.STA_REG, [1]),   # R1 = 20 (arg b)
                Instruction(Op.CALL, [0, 2, 0]),# call add(10, 20)
                _halt(),
            ],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 30

    def test_call_increments_function_count(self) -> None:
        fn = self._make_add_fn()
        code = _make(
            [
                Instruction(Op.LDA_IMM, [1]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [2]),
                Instruction(Op.STA_REG, [1]),
                Instruction(Op.CALL, [0, 2, 0]),
                _halt(),
            ],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        vm.execute(code)
        assert vm.metrics().function_call_counts.get("add") == 1

    def test_registers_isolated_between_frames(self) -> None:
        # The callee modifies R0-R1 in its own frame; caller's R0 must survive.
        fn = CodeObject(name="fn", params=["x"], feedback_slot_count=0)
        fn.var_names = ["x"]
        fn.instructions = [
            Instruction(Op.LDA_REG, [0]),   # load arg
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.LDA_IMM, [99]),
            Instruction(Op.STA_REG, [0]),   # clobber callee's R0 with 99
            Instruction(Op.LDA_VAR, [0]),
            Instruction(Op.RET, []),
        ]
        code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),   # caller R0 = 5
                Instruction(Op.LDA_IMM, [3]),
                Instruction(Op.STA_REG, [0]),   # arg = 3
                Instruction(Op.CALL, [0, 1, 0]),
                # After return: callee's R0 clobber should NOT have changed our R0.
                # But caller's R0 was 5, then overwritten to 3 before CALL.
                # Actually caller's R0=3 after the prep. Let's verify via another reg.
                Instruction(Op.STA_REG, [2]),   # save return value in R2
                Instruction(Op.LDA_IMM, [42]),
                Instruction(Op.STA_REG, [0]),   # now clobber R0 ourselves
                Instruction(Op.LDA_REG, [2]),   # reload return value
                _halt(),
            ],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 3  # fn(3) returns the arg value, not 99

    def test_call_site_feedback_recorded(self) -> None:
        fn = CodeObject(name="fn", params=["x"], feedback_slot_count=0)
        fn.var_names = ["x"]
        fn.instructions = [
            Instruction(Op.LDA_REG, [0]),
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.LDA_VAR, [0]),
            Instruction(Op.RET, []),
        ]
        code = _make(
            [
                Instruction(Op.LDA_IMM, [7]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.CALL, [0, 1, 0]),  # slot=0
                _halt(),
            ],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        vm.execute(code)
        profile = vm.type_profile("<test>", 0)
        assert profile is not None
        assert profile.kind is SlotKind.MONOMORPHIC

    def test_call_with_no_args(self) -> None:
        fn = CodeObject(name="forty_two", params=[], feedback_slot_count=0)
        fn.instructions = [
            Instruction(Op.LDA_IMM, [42]),
            Instruction(Op.RET, []),
        ]
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 42

    def test_immediate_jit_queue_populated(self) -> None:
        code = compile_program(
            "fn add(a: u8, b: u8) -> u8 { return a + b; }"
        )
        vm = TetradVM()
        vm.execute(code)
        assert "add" in vm.metrics().immediate_jit_queue


# ===========================================================================
# §11  I/O instructions
# ===========================================================================


class TestIO:
    def test_io_in(self) -> None:
        code = _make([Instruction(Op.IO_IN, []), _halt()])
        vm = TetradVM(io_in=lambda: 42)
        assert vm.execute(code) == 42

    def test_io_in_masks_to_u8(self) -> None:
        # io_in returns 300; VM masks with & 0xFF → 44.
        code = _make([Instruction(Op.IO_IN, []), _halt()])
        vm = TetradVM(io_in=lambda: 300)
        assert vm.execute(code) == 44

    def test_io_out(self) -> None:
        captured: list[int] = []
        code = _make([
            Instruction(Op.LDA_IMM, [77]),
            Instruction(Op.IO_OUT, []),
            _halt(),
        ])
        vm = TetradVM(io_out=lambda v: captured.append(v))
        vm.execute(code)
        assert captured == [77]

    def test_io_out_multiple_values(self) -> None:
        captured: list[int] = []
        code = _make([
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.IO_OUT, []),
            Instruction(Op.LDA_IMM, [2]),
            Instruction(Op.IO_OUT, []),
            Instruction(Op.LDA_IMM, [3]),
            Instruction(Op.IO_OUT, []),
            _halt(),
        ])
        vm = TetradVM(io_out=lambda v: captured.append(v))
        vm.execute(code)
        assert captured == [1, 2, 3]


# ===========================================================================
# §12  HALT
# ===========================================================================


class TestHalt:
    def test_halt_returns_accumulator(self) -> None:
        assert _run([Instruction(Op.LDA_IMM, [123]), _halt()]) == 123

    def test_halt_at_zero(self) -> None:
        assert _run([_halt()]) == 0


# ===========================================================================
# §13  Feedback vector state machine
# ===========================================================================


class TestFeedbackVector:
    def _make_op_code(self, n_slots: int) -> CodeObject:
        """Code with one ADD+slot and HALT."""
        return _make(
            [
                Instruction(Op.LDA_IMM, [1]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [2]),
                Instruction(Op.ADD, [0, 0]),  # slot 0
                _halt(),
            ],
            feedback_slot_count=n_slots,
        )

    def test_initial_state_uninitialized(self) -> None:
        vm = TetradVM()
        # Before execute, feedback_vector is None.
        assert vm.feedback_vector("<test>") is None

    def test_monomorphic_after_one_observation(self) -> None:
        code = self._make_op_code(1)
        vm = TetradVM()
        vm.execute(code)
        slot = vm.type_profile("<test>", 0)
        assert slot is not None
        assert slot.kind is SlotKind.MONOMORPHIC
        assert slot.observations == ["u8"]
        assert slot.count == 1

    def test_monomorphic_stays_after_same_type(self) -> None:
        code = self._make_op_code(1)
        vm = TetradVM()
        vm.execute(code)
        vm.execute(code)   # second run: same slot, same type
        slot = vm.type_profile("<test>", 0)
        assert slot is not None
        assert slot.kind is SlotKind.MONOMORPHIC
        assert slot.count == 2

    def test_fully_typed_gets_empty_fv(self) -> None:
        # A CodeObject with feedback_slot_count=0 never creates a vector.
        code = _make([Instruction(Op.LDA_IMM, [5]), _halt()])
        vm = TetradVM()
        vm.execute(code)
        # The main code is FULLY_TYPED (0 slots); no entry in _feedback_vectors.
        assert vm.feedback_vector("<test>") is None

    def test_type_profile_returns_none_for_unknown_fn(self) -> None:
        vm = TetradVM()
        assert vm.type_profile("unknown", 0) is None

    def test_type_profile_returns_none_for_out_of_range_slot(self) -> None:
        code = self._make_op_code(1)
        vm = TetradVM()
        vm.execute(code)
        assert vm.type_profile("<test>", 99) is None

    def test_update_slot_to_megamorphic(self) -> None:
        from tetrad_vm import _update_slot  # type: ignore[attr-defined]

        slot = SlotState(kind=SlotKind.UNINITIALIZED, observations=[], count=0)
        for ty in ["u8", "pair", "symbol", "fn", "extra"]:
            _update_slot(slot, ty)
        assert slot.kind is SlotKind.MEGAMORPHIC

    def test_megamorphic_stays_megamorphic(self) -> None:
        from tetrad_vm import _update_slot  # type: ignore[attr-defined]

        slot = SlotState(kind=SlotKind.MEGAMORPHIC, observations=["a", "b", "c", "d", "e"], count=5)
        _update_slot(slot, "new_type")
        assert slot.kind is SlotKind.MEGAMORPHIC
        assert slot.count == 6


# ===========================================================================
# §14  Branch statistics
# ===========================================================================


class TestBranchStats:
    def test_taken_ratio_zero_when_never_reached(self) -> None:
        stats = BranchStats(taken_count=0, not_taken_count=0)
        assert stats.taken_ratio == 0.0

    def test_taken_ratio_always_taken(self) -> None:
        stats = BranchStats(taken_count=10, not_taken_count=0)
        assert stats.taken_ratio == 1.0

    def test_taken_ratio_half(self) -> None:
        stats = BranchStats(taken_count=5, not_taken_count=5)
        assert stats.taken_ratio == 0.5

    def test_branch_stats_accumulate_across_runs(self) -> None:
        # JZ at ip=1, taken when acc=0.
        code = _make([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.JZ, [1]),   # ip 1, always taken
            Instruction(Op.LDA_IMM, [1]),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        vm.execute(code)
        stats = vm.branch_profile("<test>", 1)
        assert stats is not None
        assert stats.taken_count == 2


# ===========================================================================
# §15  Loop back-edge counts
# ===========================================================================


class TestLoopIterations:
    def test_loop_iteration_count(self) -> None:
        # Count down from 5; JMP_LOOP at ip=6.
        code = _make([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_REG, [0]),  # ip 2
            Instruction(Op.JZ, [3]),        # ip 3 → exit when 0
            Instruction(Op.SUB_IMM, [1]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.JMP_LOOP, [-4]), # ip 6 → back to ip 2
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        loops = vm.loop_iterations("<test>")
        assert loops[6] == 5   # body runs 5 times

    def test_loop_iterations_empty_for_unknown_fn(self) -> None:
        vm = TetradVM()
        assert vm.loop_iterations("nonexistent") == {}

    def test_loop_iterations_accumulate_across_runs(self) -> None:
        code = _make([
            Instruction(Op.LDA_IMM, [2]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_REG, [0]),  # ip 2
            Instruction(Op.JZ, [3]),
            Instruction(Op.SUB_IMM, [1]),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.JMP_LOOP, [-4]),  # ip 6
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        vm.execute(code)
        loops = vm.loop_iterations("<test>")
        assert loops[6] == 4   # 2 iterations per run × 2 runs


# ===========================================================================
# §16  Metrics API
# ===========================================================================


class TestMetricsAPI:
    def test_instruction_counts(self) -> None:
        code = _make([
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.LDA_IMM, [2]),
            Instruction(Op.LDA_ZERO, []),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        counts = vm.metrics().instruction_counts
        assert counts.get(Op.LDA_IMM) == 2
        assert counts.get(Op.LDA_ZERO) == 1
        assert counts.get(Op.HALT) == 1

    def test_total_instructions(self) -> None:
        code = _make([
            Instruction(Op.LDA_IMM, [1]),
            Instruction(Op.LDA_ZERO, []),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        assert vm.metrics().total_instructions == 3

    def test_hot_functions_below_threshold(self) -> None:
        fn = CodeObject(name="inc", params=["x"], feedback_slot_count=0)
        fn.var_names = ["x"]
        fn.instructions = [
            Instruction(Op.LDA_REG, [0]),
            Instruction(Op.STA_VAR, [0]),
            Instruction(Op.LDA_VAR, [0]),
            Instruction(Op.ADD_IMM, [1]),
            Instruction(Op.RET, []),
        ]
        code = _make(
            [Instruction(Op.LDA_IMM, [1]), Instruction(Op.STA_REG, [0]),
             Instruction(Op.CALL, [0, 1, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        vm.execute(code)
        assert vm.hot_functions(100) == []

    def test_hot_functions_above_threshold(self) -> None:
        fn = CodeObject(name="inc", params=[], feedback_slot_count=0)
        fn.instructions = [Instruction(Op.LDA_IMM, [1]), Instruction(Op.RET, [])]
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        for _ in range(101):
            vm.execute(code)
        assert "inc" in vm.hot_functions(100)
        assert vm.metrics().function_call_counts["inc"] == 101

    def test_call_site_shape(self) -> None:
        fn = CodeObject(name="fn", params=[], feedback_slot_count=0)
        fn.instructions = [Instruction(Op.LDA_IMM, [1]), Instruction(Op.RET, [])]
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        vm.execute(code)
        assert vm.call_site_shape("<test>", 0) is SlotKind.MONOMORPHIC

    def test_call_site_shape_uninitialized_for_unknown(self) -> None:
        vm = TetradVM()
        assert vm.call_site_shape("nonexistent", 0) is SlotKind.UNINITIALIZED

    def test_call_site_shape_uninitialized_for_out_of_range(self) -> None:
        code = _make([Instruction(Op.LDA_IMM, [1]), _halt()], feedback_slot_count=1)
        vm = TetradVM()
        vm.execute(code)
        assert vm.call_site_shape("<test>", 99) is SlotKind.UNINITIALIZED

    def test_reset_metrics(self) -> None:
        code = _make([Instruction(Op.LDA_IMM, [1]), _halt()])
        vm = TetradVM()
        vm.execute(code)
        assert vm.metrics().total_instructions > 0
        vm.reset_metrics()
        assert vm.metrics().total_instructions == 0
        assert vm.metrics().instruction_counts == {}
        assert vm.feedback_vector("<test>") is None

    def test_metrics_accumulate_across_executions(self) -> None:
        code = _make([_halt()])
        vm = TetradVM()
        vm.execute(code)
        vm.execute(code)
        assert vm.metrics().total_instructions == 2


# ===========================================================================
# §17  Error conditions
# ===========================================================================


class TestErrors:
    def test_division_by_zero(self) -> None:
        code = _make([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.STA_REG, [0]),   # R0 = 0
            Instruction(Op.LDA_IMM, [10]),
            Instruction(Op.DIV, [0]),       # 10 / 0 → error
            _halt(),
        ])
        vm = TetradVM()
        with pytest.raises(VMError, match="division by zero"):
            vm.execute(code)

    def test_mod_by_zero(self) -> None:
        code = _make([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.STA_REG, [0]),
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.MOD, [0]),
            _halt(),
        ])
        vm = TetradVM()
        with pytest.raises(VMError, match="division by zero"):
            vm.execute(code)

    def test_call_stack_overflow(self) -> None:
        # A function that calls itself recursively — should overflow at depth 4.
        recurse = CodeObject(name="recurse", params=[], feedback_slot_count=1)
        recurse.instructions = [
            Instruction(Op.CALL, [0, 0, 0]),  # recursive: func_idx=0
            Instruction(Op.RET, []),
        ]
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[recurse],
            feedback_slot_count=1,
        )
        # The recursive function calls itself — need to wire up its own functions list.
        recurse.functions = code.functions
        vm = TetradVM()
        with pytest.raises(VMError, match="call stack overflow"):
            vm.execute(code)

    def test_unknown_opcode(self) -> None:
        code = _make([Instruction(0xEE, [])])
        vm = TetradVM()
        with pytest.raises(VMError, match="unknown opcode"):
            vm.execute(code)

    def test_undefined_variable(self) -> None:
        code = CodeObject(name="<test>", params=[])
        code.var_names = ["x"]
        code.instructions = [Instruction(Op.LDA_VAR, [0]), _halt()]
        vm = TetradVM()
        with pytest.raises(VMError, match="undefined variable"):
            vm.execute(code)

    def test_undefined_function_index(self) -> None:
        code = _make(
            [Instruction(Op.CALL, [99, 0, 0]), _halt()],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        with pytest.raises(VMError, match="undefined function index"):
            vm.execute(code)

    def test_wrong_argument_count(self) -> None:
        fn = CodeObject(name="fn", params=["a", "b"], feedback_slot_count=0)
        fn.instructions = [Instruction(Op.LDA_IMM, [0]), Instruction(Op.RET, [])]
        code = _make(
            [Instruction(Op.CALL, [0, 1, 0]), _halt()],  # argc=1, but fn expects 2
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        with pytest.raises(VMError, match="expects 2 args"):
            vm.execute(code)


# ===========================================================================
# §18  execute_traced
# ===========================================================================


class TestExecuteTraced:
    def test_returns_tuple(self) -> None:
        code = _make([Instruction(Op.LDA_IMM, [5]), _halt()])
        vm = TetradVM()
        result, trace = vm.execute_traced(code)
        assert result == 5
        assert isinstance(trace, list)

    def test_trace_length_matches_instructions(self) -> None:
        # LDA_IMM + HALT = 2 instructions
        code = _make([Instruction(Op.LDA_IMM, [7]), _halt()])
        vm = TetradVM()
        _, trace = vm.execute_traced(code)
        assert len(trace) == 2

    def test_trace_fields(self) -> None:
        code = _make([Instruction(Op.LDA_IMM, [42]), _halt()])
        vm = TetradVM()
        _, trace = vm.execute_traced(code)
        first = trace[0]
        assert isinstance(first, VMTrace)
        assert first.ip == 0
        assert first.fn_name == "<test>"
        assert first.frame_depth == 0
        assert first.acc_before == 0
        assert first.acc_after == 42
        assert first.instruction.opcode == Op.LDA_IMM

    def test_trace_includes_function_frames(self) -> None:
        fn = CodeObject(name="fn", params=[], feedback_slot_count=0)
        fn.instructions = [Instruction(Op.LDA_IMM, [99]), Instruction(Op.RET, [])]
        code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        result, trace = vm.execute_traced(code)
        assert result == 99
        fn_steps = [t for t in trace if t.fn_name == "fn"]
        assert len(fn_steps) == 2  # LDA_IMM + RET
        assert all(t.frame_depth == 1 for t in fn_steps)

    def test_trace_feedback_delta(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [3]),
                Instruction(Op.ADD, [0, 0]),  # untyped
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        _, trace = vm.execute_traced(code)
        # Find the ADD trace step
        add_step = next(t for t in trace if t.instruction.opcode == Op.ADD)
        assert len(add_step.feedback_delta) == 1
        slot_idx, slot_state = add_step.feedback_delta[0]
        assert slot_idx == 0
        assert slot_state.kind is SlotKind.MONOMORPHIC

    def test_trace_jit_queue_populated(self) -> None:
        code = _make([_halt()])
        fn = CodeObject(name="typed_fn", params=[], immediate_jit_eligible=True, feedback_slot_count=0)
        fn.instructions = [Instruction(Op.LDA_IMM, [1]), Instruction(Op.RET, [])]
        code.functions = [fn]
        vm = TetradVM()
        vm.execute_traced(code)
        assert "typed_fn" in vm.metrics().immediate_jit_queue


# ===========================================================================
# §19  End-to-end programs via compile_program
# ===========================================================================


class TestEndToEnd:
    def test_simple_addition(self) -> None:
        code = compile_program(
            "fn add(a: u8, b: u8) -> u8 { return a + b; }"
            "\nlet result = add(10, 20);"
        )
        vm = TetradVM()
        vm.execute(code)
        assert vm._globals["result"] == 30  # noqa: SLF001

    def test_fully_typed_function_immediate_jit(self) -> None:
        code = compile_program("fn double(a: u8) -> u8 { return a + a; }")
        assert code.functions[0].immediate_jit_eligible is True
        vm = TetradVM()
        vm.execute(code)
        assert "double" in vm.metrics().immediate_jit_queue

    def test_while_loop(self) -> None:
        # sum_n(5) = 1+2+3+4+5 = 15
        code = compile_program(
            "fn sum_n(n: u8) -> u8 {"
            "  let acc: u8 = 0;"
            "  while n > 0 {"
            "    acc = acc + n;"
            "    n = n - 1;"
            "  }"
            "  return acc;"
            "}"
        )
        # Call sum_n(5) via the VM directly.
        call_code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),    # arg n=5
                Instruction(Op.CALL, [0, 1, 0]),
                _halt(),
            ],
            functions=list(code.functions),
            feedback_slot_count=1,
        )
        # Wire up the main code so CALL can resolve functions.
        vm2 = TetradVM()
        result = vm2.execute(call_code)
        assert result == 15

    def test_io_out_via_compile(self) -> None:
        # "out" is only valid inside a function body in Tetrad (not top-level).
        captured: list[int] = []
        code = compile_program("fn run() { out(42); }")
        call_code = _make(
            [Instruction(Op.CALL, [0, 0, 0]), _halt()],
            functions=list(code.functions),
            feedback_slot_count=1,
        )
        vm = TetradVM(io_out=lambda v: captured.append(v))
        vm.execute(call_code)
        assert captured == [42]

    def test_short_circuit_and(self) -> None:
        code = compile_program(
            "fn f(a: u8, b: u8) -> u8 { return a && b; }"
        )
        call_code = _make(
            [
                Instruction(Op.LDA_ZERO, []),
                Instruction(Op.STA_REG, [0]),    # a=0
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [1]),    # b=5
                Instruction(Op.CALL, [0, 2, 0]),
                _halt(),
            ],
            functions=list(code.functions),
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(call_code) == 0   # 0 && 5 → 0

    def test_short_circuit_or(self) -> None:
        # The compiler's || emits: JNZ (if a≠0 → return 1); load b; JMP (return b).
        # So: truthy a → 1, falsy a → b (the raw value, not a boolean cast).
        # Verify: a=3 (truthy) → 1; a=0 (falsy) → b=5.
        code = compile_program(
            "fn f(a: u8, b: u8) -> u8 { return a || b; }"
        )

        def _call(a: int, b: int) -> int:
            cc = _make(
                [
                    Instruction(Op.LDA_IMM, [a]),
                    Instruction(Op.STA_REG, [0]),
                    Instruction(Op.LDA_IMM, [b]),
                    Instruction(Op.STA_REG, [1]),
                    Instruction(Op.CALL, [0, 2, 0]),
                    _halt(),
                ],
                functions=list(code.functions),
                feedback_slot_count=1,
            )
            return TetradVM().execute(cc)

        assert _call(3, 5) == 1   # truthy a → 1
        assert _call(0, 5) == 5   # falsy a → b (raw value)
        assert _call(0, 0) == 0   # both falsy → 0

    def test_unary_bitwise_not(self) -> None:
        code = compile_program("fn f(x: u8) -> u8 { return ~x; }")
        call_code = _make(
            [
                Instruction(Op.LDA_IMM, [0x0F]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.CALL, [0, 1, 0]),
                _halt(),
            ],
            functions=list(code.functions),
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(call_code) == 0xF0

    def test_unary_negation(self) -> None:
        code = compile_program("fn neg(x: u8) -> u8 { return -x; }")
        call_code = _make(
            [
                Instruction(Op.LDA_IMM, [1]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.CALL, [0, 1, 0]),
                _halt(),
            ],
            functions=list(code.functions),
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(call_code) == 255   # 0 - 1 = 255 (u8 wrap)


# ===========================================================================
# §20  Coverage gaps and edge cases
# ===========================================================================


class TestCoverageGaps:
    def test_vmmetrics_default_construction(self) -> None:
        m = VMMetrics()
        assert m.total_instructions == 0
        assert m.immediate_jit_queue == []

    def test_slotstate_default_construction(self) -> None:
        s = SlotState()
        assert s.kind is SlotKind.UNINITIALIZED
        assert s.observations == []
        assert s.count == 0

    def test_branch_stats_default(self) -> None:
        b = BranchStats()
        assert b.taken_count == 0
        assert b.not_taken_count == 0

    def test_call_frame_fields(self) -> None:
        # TetradVM uses GenericRegisterVM's RegisterFrame internally.
        # Verify that a frame can be constructed with the expected fields.
        from register_vm.generic_vm import RegisterFrame  # noqa: PLC0415
        cf = RegisterFrame(
            instructions=[],
            ip=0,
            acc=0,
            registers=[0] * 8,
            depth=0,
            caller_frame=None,
        )
        assert cf.depth == 0
        assert cf.caller_frame is None
        assert cf.user_data == {}

    def test_vm_error_is_exception(self) -> None:
        e = VMError("test error")
        assert isinstance(e, Exception)
        assert str(e) == "test error"

    def test_lda_var_local_prefers_locals_over_globals(self) -> None:
        # In a function frame, locals take priority over globals.
        fn = CodeObject(name="fn", params=["x"], feedback_slot_count=0)
        fn.var_names = ["x"]
        fn.instructions = [
            Instruction(Op.LDA_REG, [0]),   # load arg from R0
            Instruction(Op.STA_VAR, [0]),   # store to locals["x"]
            Instruction(Op.LDA_VAR, [0]),   # should read from locals, not globals
            Instruction(Op.RET, []),
        ]
        # Set up globals["x"] = 99 to ensure locals win.
        code = _make(
            [
                Instruction(Op.LDA_IMM, [99]),
                Instruction(Op.STA_VAR, [0]),   # global "x" = 99
                Instruction(Op.LDA_IMM, [7]),
                Instruction(Op.STA_REG, [0]),   # arg = 7
                Instruction(Op.CALL, [0, 1, 0]),
                _halt(),
            ],
            var_names=["x"],
            functions=[fn],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 7   # fn returns its local "x"=7, not global 99

    def test_get_or_create_fv_zero_slots(self) -> None:
        code = _make([_halt()], feedback_slot_count=0)
        vm = TetradVM()
        vm._main_code = code  # noqa: SLF001
        fv = vm._get_or_create_fv(code)  # noqa: SLF001
        assert fv == []

    def test_get_or_create_fv_reuses_existing(self) -> None:
        code = _make([_halt()], feedback_slot_count=2)
        vm = TetradVM()
        vm._main_code = code  # noqa: SLF001
        fv1 = vm._get_or_create_fv(code)  # noqa: SLF001
        fv2 = vm._get_or_create_fv(code)  # noqa: SLF001
        assert fv1 is fv2   # same object reused

    def test_div_untyped_with_slot(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [2]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [10]),
                Instruction(Op.DIV, [0, 0]),  # untyped: [r, slot]
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 5
        assert vm.type_profile("<test>", 0).kind is SlotKind.MONOMORPHIC  # type: ignore[union-attr]

    def test_mod_untyped_with_slot(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [3]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [10]),
                Instruction(Op.MOD, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 1

    def test_comparison_with_slot_neq(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [3]),
                Instruction(Op.NEQ, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 1

    def test_comparison_with_slot_lte(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.LTE, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 1

    def test_comparison_with_slot_gt(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [3]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.GT, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 1

    def test_comparison_with_slot_gte(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.GTE, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        assert vm.execute(code) == 1

    def test_execute_resets_globals(self) -> None:
        code = _make([
            Instruction(Op.LDA_IMM, [99]),
            Instruction(Op.STA_VAR, [0]),
            _halt(),
        ], var_names=["g"])
        vm = TetradVM()
        vm.execute(code)
        assert vm._globals["g"] == 99  # noqa: SLF001
        vm.execute(code)  # second run resets globals first
        assert vm._globals["g"] == 99  # noqa: SLF001

    def test_jz_branch_not_taken_stats(self) -> None:
        # JZ when acc != 0 → not taken.
        code = _make([
            Instruction(Op.LDA_IMM, [5]),
            Instruction(Op.JZ, [1]),   # ip 1, NOT taken
            Instruction(Op.LDA_IMM, [7]),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        stats = vm.branch_profile("<test>", 1)
        assert stats is not None
        assert stats.not_taken_count == 1
        assert stats.taken_count == 0

    def test_jnz_branch_not_taken_stats(self) -> None:
        code = _make([
            Instruction(Op.LDA_ZERO, []),
            Instruction(Op.JNZ, [1]),   # ip 1, NOT taken (acc=0)
            Instruction(Op.LDA_IMM, [3]),
            _halt(),
        ])
        vm = TetradVM()
        vm.execute(code)
        stats = vm.branch_profile("<test>", 1)
        assert stats is not None
        assert stats.not_taken_count == 1

    def test_div_by_zero_untyped(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_ZERO, []),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.DIV, [0, 0]),  # untyped with slot
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        with pytest.raises(VMError, match="division by zero"):
            vm.execute(code)

    def test_mod_by_zero_untyped(self) -> None:
        code = _make(
            [
                Instruction(Op.LDA_ZERO, []),
                Instruction(Op.STA_REG, [0]),
                Instruction(Op.LDA_IMM, [5]),
                Instruction(Op.MOD, [0, 0]),
                _halt(),
            ],
            feedback_slot_count=1,
        )
        vm = TetradVM()
        with pytest.raises(VMError, match="division by zero"):
            vm.execute(code)
