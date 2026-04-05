"""test_instructions.py --- Tests for individual WASM instruction handlers.

Tests numeric i32/i64/f32/f64 instructions, conversions, parametric
(drop/select), and variable (local.get/set/tee, global.get/set) by
directly invoking handlers through the GenericVM.

Strategy: We register the handlers on a GenericVM instance, push operands,
then execute a single instruction to check the result on the stack.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Any

import pytest
from virtual_machine.generic_vm import GenericVM
from virtual_machine.vm import CodeObject, Instruction
from wasm_types import FuncType, GlobalType, ValueType

from wasm_execution.host_interface import TrapError
from wasm_execution.instructions.control import register_control
from wasm_execution.instructions.dispatch import register_all_instructions
from wasm_execution.types import ControlTarget, Label, WasmExecutionContext
from wasm_execution.values import as_f32, as_f64, as_i32, as_i64, f32, f64, i32, i64


# ===========================================================================
# Test helper: run a single instruction on a fresh VM
# ===========================================================================


def _run_instruction(
    opcode: int,
    operand: Any,
    stack: list[Any],
    ctx: WasmExecutionContext | None = None,
) -> GenericVM:
    """Push stack values, execute one instruction, return the VM."""
    vm = GenericVM()
    register_all_instructions(vm)
    register_control(vm)

    for val in stack:
        vm.push_typed(val)

    if ctx is None:
        ctx = WasmExecutionContext(
            memory=None,
            tables=[],
            globals=[],
            global_types=[],
            func_types=[],
            func_bodies=[],
            host_functions=[],
            typed_locals=[],
            label_stack=[],
            control_flow_map={},
            saved_frames=[],
        )

    instr = Instruction(opcode=opcode, operand=operand)
    code = CodeObject(instructions=[instr], constants=[], names=[])
    vm.execute_with_context(code, ctx)
    return vm


# ===========================================================================
# i32 numeric instructions
# ===========================================================================


class TestI32Numeric:
    def test_i32_const(self) -> None:
        vm = _run_instruction(0x41, 42, [])
        assert as_i32(vm.pop_typed()) == 42

    def test_i32_eqz_true(self) -> None:
        vm = _run_instruction(0x45, None, [i32(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_eqz_false(self) -> None:
        vm = _run_instruction(0x45, None, [i32(5)])
        assert as_i32(vm.pop_typed()) == 0

    def test_i32_eq(self) -> None:
        vm = _run_instruction(0x46, None, [i32(3), i32(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_ne(self) -> None:
        vm = _run_instruction(0x47, None, [i32(3), i32(4)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_lt_s(self) -> None:
        vm = _run_instruction(0x48, None, [i32(-1), i32(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_lt_u(self) -> None:
        # -1 unsigned is 0xFFFFFFFF, which is > 0
        vm = _run_instruction(0x49, None, [i32(-1), i32(0)])
        assert as_i32(vm.pop_typed()) == 0

    def test_i32_gt_s(self) -> None:
        vm = _run_instruction(0x4A, None, [i32(5), i32(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_gt_u(self) -> None:
        vm = _run_instruction(0x4B, None, [i32(-1), i32(1)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_le_s(self) -> None:
        vm = _run_instruction(0x4C, None, [i32(3), i32(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_le_u(self) -> None:
        vm = _run_instruction(0x4D, None, [i32(0), i32(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_ge_s(self) -> None:
        vm = _run_instruction(0x4E, None, [i32(5), i32(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_ge_u(self) -> None:
        vm = _run_instruction(0x4F, None, [i32(-1), i32(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_add(self) -> None:
        vm = _run_instruction(0x6A, None, [i32(10), i32(20)])
        assert as_i32(vm.pop_typed()) == 30

    def test_i32_add_overflow(self) -> None:
        vm = _run_instruction(0x6A, None, [i32(2**31 - 1), i32(1)])
        assert as_i32(vm.pop_typed()) == -(2**31)

    def test_i32_sub(self) -> None:
        vm = _run_instruction(0x6B, None, [i32(30), i32(10)])
        assert as_i32(vm.pop_typed()) == 20

    def test_i32_mul(self) -> None:
        vm = _run_instruction(0x6C, None, [i32(6), i32(7)])
        assert as_i32(vm.pop_typed()) == 42

    def test_i32_div_s(self) -> None:
        vm = _run_instruction(0x6D, None, [i32(-7), i32(2)])
        assert as_i32(vm.pop_typed()) == -3

    def test_i32_div_s_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x6D, None, [i32(1), i32(0)])

    def test_i32_div_s_overflow(self) -> None:
        with pytest.raises(TrapError, match="overflow"):
            _run_instruction(0x6D, None, [i32(-2147483648), i32(-1)])

    def test_i32_div_u(self) -> None:
        vm = _run_instruction(0x6E, None, [i32(-1), i32(2)])
        # -1 unsigned is 0xFFFFFFFF = 4294967295
        assert as_i32(vm.pop_typed()) == i32(4294967295 // 2).value

    def test_i32_div_u_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x6E, None, [i32(1), i32(0)])

    def test_i32_rem_s(self) -> None:
        vm = _run_instruction(0x6F, None, [i32(-7), i32(2)])
        assert as_i32(vm.pop_typed()) == -1

    def test_i32_rem_s_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x6F, None, [i32(1), i32(0)])

    def test_i32_rem_s_min_div_neg1(self) -> None:
        """INT32_MIN % -1 should be 0 per spec."""
        vm = _run_instruction(0x6F, None, [i32(-2147483648), i32(-1)])
        assert as_i32(vm.pop_typed()) == 0

    def test_i32_rem_u(self) -> None:
        vm = _run_instruction(0x70, None, [i32(7), i32(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_rem_u_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x70, None, [i32(1), i32(0)])

    def test_i32_and(self) -> None:
        vm = _run_instruction(0x71, None, [i32(0xFF), i32(0x0F)])
        assert as_i32(vm.pop_typed()) == 0x0F

    def test_i32_or(self) -> None:
        vm = _run_instruction(0x72, None, [i32(0xF0), i32(0x0F)])
        assert as_i32(vm.pop_typed()) == 0xFF

    def test_i32_xor(self) -> None:
        vm = _run_instruction(0x73, None, [i32(0xFF), i32(0x0F)])
        assert as_i32(vm.pop_typed()) == 0xF0

    def test_i32_shl(self) -> None:
        vm = _run_instruction(0x74, None, [i32(1), i32(4)])
        assert as_i32(vm.pop_typed()) == 16

    def test_i32_shr_s(self) -> None:
        vm = _run_instruction(0x75, None, [i32(-16), i32(2)])
        assert as_i32(vm.pop_typed()) == -4

    def test_i32_shr_u(self) -> None:
        vm = _run_instruction(0x76, None, [i32(-1), i32(1)])
        # unsigned -1 >> 1 = 0x7FFFFFFF
        assert as_i32(vm.pop_typed()) == 0x7FFFFFFF

    def test_i32_rotl(self) -> None:
        vm = _run_instruction(0x77, None, [i32(1), i32(1)])
        assert as_i32(vm.pop_typed()) == 2

    def test_i32_rotr(self) -> None:
        vm = _run_instruction(0x78, None, [i32(1), i32(1)])
        # rotr(1, 1) = 0x80000000
        assert as_i32(vm.pop_typed()) == i32(0x80000000).value

    def test_i32_clz_zero(self) -> None:
        vm = _run_instruction(0x67, None, [i32(0)])
        assert as_i32(vm.pop_typed()) == 32

    def test_i32_clz_one(self) -> None:
        vm = _run_instruction(0x67, None, [i32(1)])
        assert as_i32(vm.pop_typed()) == 31

    def test_i32_ctz_zero(self) -> None:
        vm = _run_instruction(0x68, None, [i32(0)])
        assert as_i32(vm.pop_typed()) == 32

    def test_i32_ctz(self) -> None:
        vm = _run_instruction(0x68, None, [i32(8)])
        assert as_i32(vm.pop_typed()) == 3

    def test_i32_popcnt(self) -> None:
        vm = _run_instruction(0x69, None, [i32(0x0F)])
        assert as_i32(vm.pop_typed()) == 4


# ===========================================================================
# i64 numeric instructions
# ===========================================================================


class TestI64Numeric:
    def test_i64_const(self) -> None:
        vm = _run_instruction(0x42, 99, [])
        assert as_i64(vm.pop_typed()) == 99

    def test_i64_eqz_true(self) -> None:
        vm = _run_instruction(0x50, None, [i64(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_eqz_false(self) -> None:
        vm = _run_instruction(0x50, None, [i64(1)])
        assert as_i32(vm.pop_typed()) == 0

    def test_i64_eq(self) -> None:
        vm = _run_instruction(0x51, None, [i64(10), i64(10)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_ne(self) -> None:
        vm = _run_instruction(0x52, None, [i64(10), i64(20)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_lt_s(self) -> None:
        vm = _run_instruction(0x53, None, [i64(-1), i64(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_lt_u(self) -> None:
        vm = _run_instruction(0x54, None, [i64(-1), i64(0)])
        assert as_i32(vm.pop_typed()) == 0

    def test_i64_gt_s(self) -> None:
        vm = _run_instruction(0x55, None, [i64(5), i64(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_gt_u(self) -> None:
        vm = _run_instruction(0x56, None, [i64(-1), i64(1)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_le_s(self) -> None:
        vm = _run_instruction(0x57, None, [i64(3), i64(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_le_u(self) -> None:
        vm = _run_instruction(0x58, None, [i64(0), i64(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_ge_s(self) -> None:
        vm = _run_instruction(0x59, None, [i64(5), i64(3)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_ge_u(self) -> None:
        vm = _run_instruction(0x5A, None, [i64(-1), i64(0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i64_add(self) -> None:
        vm = _run_instruction(0x7C, None, [i64(10), i64(20)])
        assert as_i64(vm.pop_typed()) == 30

    def test_i64_sub(self) -> None:
        vm = _run_instruction(0x7D, None, [i64(30), i64(10)])
        assert as_i64(vm.pop_typed()) == 20

    def test_i64_mul(self) -> None:
        vm = _run_instruction(0x7E, None, [i64(6), i64(7)])
        assert as_i64(vm.pop_typed()) == 42

    def test_i64_div_s(self) -> None:
        vm = _run_instruction(0x7F, None, [i64(-7), i64(2)])
        assert as_i64(vm.pop_typed()) == -3

    def test_i64_div_s_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x7F, None, [i64(1), i64(0)])

    def test_i64_div_s_overflow(self) -> None:
        with pytest.raises(TrapError, match="overflow"):
            _run_instruction(0x7F, None, [i64(-(2**63)), i64(-1)])

    def test_i64_div_u(self) -> None:
        vm = _run_instruction(0x80, None, [i64(10), i64(3)])
        assert as_i64(vm.pop_typed()) == 3

    def test_i64_div_u_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x80, None, [i64(1), i64(0)])

    def test_i64_rem_s(self) -> None:
        vm = _run_instruction(0x81, None, [i64(-7), i64(2)])
        assert as_i64(vm.pop_typed()) == -1

    def test_i64_rem_s_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x81, None, [i64(1), i64(0)])

    def test_i64_rem_s_min_div_neg1(self) -> None:
        vm = _run_instruction(0x81, None, [i64(-(2**63)), i64(-1)])
        assert as_i64(vm.pop_typed()) == 0

    def test_i64_rem_u(self) -> None:
        vm = _run_instruction(0x82, None, [i64(7), i64(3)])
        assert as_i64(vm.pop_typed()) == 1

    def test_i64_rem_u_by_zero(self) -> None:
        with pytest.raises(TrapError, match="divide by zero"):
            _run_instruction(0x82, None, [i64(1), i64(0)])

    def test_i64_and(self) -> None:
        vm = _run_instruction(0x83, None, [i64(0xFF), i64(0x0F)])
        assert as_i64(vm.pop_typed()) == 0x0F

    def test_i64_or(self) -> None:
        vm = _run_instruction(0x84, None, [i64(0xF0), i64(0x0F)])
        assert as_i64(vm.pop_typed()) == 0xFF

    def test_i64_xor(self) -> None:
        vm = _run_instruction(0x85, None, [i64(0xFF), i64(0x0F)])
        assert as_i64(vm.pop_typed()) == 0xF0

    def test_i64_shl(self) -> None:
        vm = _run_instruction(0x86, None, [i64(1), i64(4)])
        assert as_i64(vm.pop_typed()) == 16

    def test_i64_shr_s(self) -> None:
        vm = _run_instruction(0x87, None, [i64(-16), i64(2)])
        assert as_i64(vm.pop_typed()) == -4

    def test_i64_shr_u(self) -> None:
        vm = _run_instruction(0x88, None, [i64(-1), i64(1)])
        assert as_i64(vm.pop_typed()) == i64(0x7FFFFFFFFFFFFFFF).value

    def test_i64_rotl(self) -> None:
        vm = _run_instruction(0x89, None, [i64(1), i64(1)])
        assert as_i64(vm.pop_typed()) == 2

    def test_i64_rotr(self) -> None:
        vm = _run_instruction(0x8A, None, [i64(1), i64(1)])
        assert as_i64(vm.pop_typed()) == i64(0x8000000000000000).value

    def test_i64_clz(self) -> None:
        vm = _run_instruction(0x79, None, [i64(0)])
        assert as_i64(vm.pop_typed()) == 64

    def test_i64_clz_one(self) -> None:
        vm = _run_instruction(0x79, None, [i64(1)])
        assert as_i64(vm.pop_typed()) == 63

    def test_i64_ctz(self) -> None:
        vm = _run_instruction(0x7A, None, [i64(0)])
        assert as_i64(vm.pop_typed()) == 64

    def test_i64_ctz_nonzero(self) -> None:
        vm = _run_instruction(0x7A, None, [i64(8)])
        assert as_i64(vm.pop_typed()) == 3

    def test_i64_popcnt(self) -> None:
        vm = _run_instruction(0x7B, None, [i64(0x0F)])
        assert as_i64(vm.pop_typed()) == 4


# ===========================================================================
# f32 numeric instructions
# ===========================================================================


class TestF32Numeric:
    def test_f32_const(self) -> None:
        vm = _run_instruction(0x43, 3.14, [])
        assert as_f32(vm.pop_typed()) == pytest.approx(3.14, abs=1e-5)

    def test_f32_eq(self) -> None:
        vm = _run_instruction(0x5B, None, [f32(1.0), f32(1.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_ne(self) -> None:
        vm = _run_instruction(0x5C, None, [f32(1.0), f32(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_lt(self) -> None:
        vm = _run_instruction(0x5D, None, [f32(1.0), f32(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_gt(self) -> None:
        vm = _run_instruction(0x5E, None, [f32(3.0), f32(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_le(self) -> None:
        vm = _run_instruction(0x5F, None, [f32(2.0), f32(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_ge(self) -> None:
        vm = _run_instruction(0x60, None, [f32(2.0), f32(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f32_abs(self) -> None:
        vm = _run_instruction(0x8B, None, [f32(-5.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(5.0)

    def test_f32_neg(self) -> None:
        vm = _run_instruction(0x8C, None, [f32(3.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(-3.0)

    def test_f32_ceil(self) -> None:
        vm = _run_instruction(0x8D, None, [f32(2.3)])
        assert as_f32(vm.pop_typed()) == pytest.approx(3.0)

    def test_f32_floor(self) -> None:
        vm = _run_instruction(0x8E, None, [f32(2.7)])
        assert as_f32(vm.pop_typed()) == pytest.approx(2.0)

    def test_f32_trunc(self) -> None:
        vm = _run_instruction(0x8F, None, [f32(-2.7)])
        assert as_f32(vm.pop_typed()) == pytest.approx(-2.0)

    def test_f32_nearest(self) -> None:
        vm = _run_instruction(0x90, None, [f32(2.5)])
        assert as_f32(vm.pop_typed()) == pytest.approx(2.0)

    def test_f32_sqrt(self) -> None:
        vm = _run_instruction(0x91, None, [f32(4.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(2.0)

    def test_f32_sqrt_negative(self) -> None:
        vm = _run_instruction(0x91, None, [f32(-1.0)])
        assert math.isnan(as_f32(vm.pop_typed()))

    def test_f32_add(self) -> None:
        vm = _run_instruction(0x92, None, [f32(1.0), f32(2.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(3.0)

    def test_f32_sub(self) -> None:
        vm = _run_instruction(0x93, None, [f32(5.0), f32(2.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(3.0)

    def test_f32_mul(self) -> None:
        vm = _run_instruction(0x94, None, [f32(3.0), f32(4.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(12.0)

    def test_f32_div(self) -> None:
        vm = _run_instruction(0x95, None, [f32(10.0), f32(4.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(2.5)

    def test_f32_min(self) -> None:
        vm = _run_instruction(0x96, None, [f32(1.0), f32(2.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(1.0)

    def test_f32_min_nan(self) -> None:
        vm = _run_instruction(0x96, None, [f32(float("nan")), f32(1.0)])
        assert math.isnan(as_f32(vm.pop_typed()))

    def test_f32_max(self) -> None:
        vm = _run_instruction(0x97, None, [f32(1.0), f32(2.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(2.0)

    def test_f32_max_nan(self) -> None:
        vm = _run_instruction(0x97, None, [f32(1.0), f32(float("nan"))])
        assert math.isnan(as_f32(vm.pop_typed()))

    def test_f32_copysign(self) -> None:
        vm = _run_instruction(0x98, None, [f32(3.0), f32(-1.0)])
        assert as_f32(vm.pop_typed()) == pytest.approx(-3.0)


# ===========================================================================
# f64 numeric instructions
# ===========================================================================


class TestF64Numeric:
    def test_f64_const(self) -> None:
        vm = _run_instruction(0x44, 2.718, [])
        assert as_f64(vm.pop_typed()) == pytest.approx(2.718)

    def test_f64_eq(self) -> None:
        vm = _run_instruction(0x61, None, [f64(1.0), f64(1.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_ne(self) -> None:
        vm = _run_instruction(0x62, None, [f64(1.0), f64(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_lt(self) -> None:
        vm = _run_instruction(0x63, None, [f64(1.0), f64(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_gt(self) -> None:
        vm = _run_instruction(0x64, None, [f64(3.0), f64(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_le(self) -> None:
        vm = _run_instruction(0x65, None, [f64(2.0), f64(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_ge(self) -> None:
        vm = _run_instruction(0x66, None, [f64(2.0), f64(2.0)])
        assert as_i32(vm.pop_typed()) == 1

    def test_f64_abs(self) -> None:
        vm = _run_instruction(0x99, None, [f64(-5.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(5.0)

    def test_f64_neg(self) -> None:
        vm = _run_instruction(0x9A, None, [f64(3.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(-3.0)

    def test_f64_ceil(self) -> None:
        vm = _run_instruction(0x9B, None, [f64(2.3)])
        assert as_f64(vm.pop_typed()) == pytest.approx(3.0)

    def test_f64_floor(self) -> None:
        vm = _run_instruction(0x9C, None, [f64(2.7)])
        assert as_f64(vm.pop_typed()) == pytest.approx(2.0)

    def test_f64_trunc(self) -> None:
        vm = _run_instruction(0x9D, None, [f64(-2.7)])
        assert as_f64(vm.pop_typed()) == pytest.approx(-2.0)

    def test_f64_nearest(self) -> None:
        vm = _run_instruction(0x9E, None, [f64(2.5)])
        assert as_f64(vm.pop_typed()) == pytest.approx(2.0)

    def test_f64_sqrt(self) -> None:
        vm = _run_instruction(0x9F, None, [f64(9.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(3.0)

    def test_f64_add(self) -> None:
        vm = _run_instruction(0xA0, None, [f64(1.0), f64(2.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(3.0)

    def test_f64_sub(self) -> None:
        vm = _run_instruction(0xA1, None, [f64(5.0), f64(2.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(3.0)

    def test_f64_mul(self) -> None:
        vm = _run_instruction(0xA2, None, [f64(3.0), f64(4.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(12.0)

    def test_f64_div(self) -> None:
        vm = _run_instruction(0xA3, None, [f64(10.0), f64(4.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(2.5)

    def test_f64_min(self) -> None:
        vm = _run_instruction(0xA4, None, [f64(1.0), f64(2.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(1.0)

    def test_f64_max(self) -> None:
        vm = _run_instruction(0xA5, None, [f64(1.0), f64(2.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(2.0)

    def test_f64_copysign(self) -> None:
        vm = _run_instruction(0xA6, None, [f64(3.0), f64(-1.0)])
        assert as_f64(vm.pop_typed()) == pytest.approx(-3.0)


# ===========================================================================
# Conversion instructions
# ===========================================================================


class TestConversions:
    def test_i32_wrap_i64(self) -> None:
        vm = _run_instruction(0xA7, None, [i64(0x1_00000001)])
        assert as_i32(vm.pop_typed()) == 1

    def test_i32_trunc_f32_s(self) -> None:
        vm = _run_instruction(0xA8, None, [f32(3.9)])
        assert as_i32(vm.pop_typed()) == 3

    def test_i32_trunc_f32_s_nan_traps(self) -> None:
        with pytest.raises(TrapError, match="invalid conversion"):
            _run_instruction(0xA8, None, [f32(float("nan"))])

    def test_i32_trunc_f32_u(self) -> None:
        vm = _run_instruction(0xA9, None, [f32(3.9)])
        assert as_i32(vm.pop_typed()) == 3

    def test_i32_trunc_f32_u_negative_traps(self) -> None:
        with pytest.raises(TrapError, match="overflow"):
            _run_instruction(0xA9, None, [f32(-1.0)])

    def test_i32_trunc_f64_s(self) -> None:
        vm = _run_instruction(0xAA, None, [f64(-3.9)])
        assert as_i32(vm.pop_typed()) == -3

    def test_i32_trunc_f64_u(self) -> None:
        vm = _run_instruction(0xAB, None, [f64(3.9)])
        assert as_i32(vm.pop_typed()) == 3

    def test_i64_extend_i32_s(self) -> None:
        vm = _run_instruction(0xAC, None, [i32(-1)])
        assert as_i64(vm.pop_typed()) == -1

    def test_i64_extend_i32_u(self) -> None:
        vm = _run_instruction(0xAD, None, [i32(-1)])
        assert as_i64(vm.pop_typed()) == 0xFFFFFFFF

    def test_i64_trunc_f32_s(self) -> None:
        vm = _run_instruction(0xAE, None, [f32(3.9)])
        assert as_i64(vm.pop_typed()) == 3

    def test_i64_trunc_f32_u(self) -> None:
        vm = _run_instruction(0xAF, None, [f32(3.9)])
        assert as_i64(vm.pop_typed()) == 3

    def test_i64_trunc_f64_s(self) -> None:
        vm = _run_instruction(0xB0, None, [f64(-3.9)])
        assert as_i64(vm.pop_typed()) == -3

    def test_i64_trunc_f64_u(self) -> None:
        vm = _run_instruction(0xB1, None, [f64(3.9)])
        assert as_i64(vm.pop_typed()) == 3

    def test_f32_convert_i32_s(self) -> None:
        vm = _run_instruction(0xB2, None, [i32(-1)])
        assert as_f32(vm.pop_typed()) == pytest.approx(-1.0)

    def test_f32_convert_i32_u(self) -> None:
        vm = _run_instruction(0xB3, None, [i32(-1)])
        assert as_f32(vm.pop_typed()) == pytest.approx(4294967296.0, rel=1e-5)

    def test_f32_convert_i64_s(self) -> None:
        vm = _run_instruction(0xB4, None, [i64(42)])
        assert as_f32(vm.pop_typed()) == pytest.approx(42.0)

    def test_f32_convert_i64_u(self) -> None:
        vm = _run_instruction(0xB5, None, [i64(42)])
        assert as_f32(vm.pop_typed()) == pytest.approx(42.0)

    def test_f32_demote_f64(self) -> None:
        vm = _run_instruction(0xB6, None, [f64(3.14)])
        assert as_f32(vm.pop_typed()) == pytest.approx(3.14, abs=1e-5)

    def test_f64_convert_i32_s(self) -> None:
        vm = _run_instruction(0xB7, None, [i32(-5)])
        assert as_f64(vm.pop_typed()) == pytest.approx(-5.0)

    def test_f64_convert_i32_u(self) -> None:
        vm = _run_instruction(0xB8, None, [i32(-1)])
        assert as_f64(vm.pop_typed()) == pytest.approx(4294967295.0)

    def test_f64_convert_i64_s(self) -> None:
        vm = _run_instruction(0xB9, None, [i64(42)])
        assert as_f64(vm.pop_typed()) == pytest.approx(42.0)

    def test_f64_convert_i64_u(self) -> None:
        vm = _run_instruction(0xBA, None, [i64(42)])
        assert as_f64(vm.pop_typed()) == pytest.approx(42.0)

    def test_f64_promote_f32(self) -> None:
        vm = _run_instruction(0xBB, None, [f32(3.14)])
        assert as_f64(vm.pop_typed()) == pytest.approx(3.14, abs=1e-5)

    def test_i32_reinterpret_f32(self) -> None:
        vm = _run_instruction(0xBC, None, [f32(1.0)])
        # IEEE 754: 1.0f = 0x3F800000
        assert as_i32(vm.pop_typed()) == 0x3F800000

    def test_i64_reinterpret_f64(self) -> None:
        vm = _run_instruction(0xBD, None, [f64(1.0)])
        assert as_i64(vm.pop_typed()) == 0x3FF0000000000000

    def test_f32_reinterpret_i32(self) -> None:
        vm = _run_instruction(0xBE, None, [i32(0x3F800000)])
        assert as_f32(vm.pop_typed()) == pytest.approx(1.0)

    def test_f64_reinterpret_i64(self) -> None:
        vm = _run_instruction(0xBF, None, [i64(0x3FF0000000000000)])
        assert as_f64(vm.pop_typed()) == pytest.approx(1.0)


# ===========================================================================
# Parametric instructions (drop, select)
# ===========================================================================


class TestParametric:
    def test_drop(self) -> None:
        vm = _run_instruction(0x1A, None, [i32(42)])
        assert len(vm.typed_stack) == 0

    def test_select_true(self) -> None:
        vm = _run_instruction(0x1B, None, [i32(10), i32(20), i32(1)])
        assert as_i32(vm.pop_typed()) == 10

    def test_select_false(self) -> None:
        vm = _run_instruction(0x1B, None, [i32(10), i32(20), i32(0)])
        assert as_i32(vm.pop_typed()) == 20


# ===========================================================================
# Variable instructions (local.get/set/tee, global.get/set)
# ===========================================================================


class TestVariable:
    def _make_ctx(self) -> WasmExecutionContext:
        return WasmExecutionContext(
            memory=None,
            tables=[],
            globals=[i32(100), i32(200)],
            global_types=[
                GlobalType(value_type=ValueType.I32, mutable=True),
                GlobalType(value_type=ValueType.I32, mutable=True),
            ],
            func_types=[],
            func_bodies=[],
            host_functions=[],
            typed_locals=[i32(10), i32(20), i32(30)],
            label_stack=[],
            control_flow_map={},
            saved_frames=[],
        )

    def test_local_get(self) -> None:
        ctx = self._make_ctx()
        vm = _run_instruction(0x20, 1, [], ctx)
        assert as_i32(vm.pop_typed()) == 20

    def test_local_set(self) -> None:
        ctx = self._make_ctx()
        vm = _run_instruction(0x21, 0, [i32(99)], ctx)
        assert ctx.typed_locals[0].value == 99

    def test_local_tee(self) -> None:
        ctx = self._make_ctx()
        vm = _run_instruction(0x22, 2, [i32(77)], ctx)
        assert ctx.typed_locals[2].value == 77
        # tee leaves value on stack
        assert as_i32(vm.pop_typed()) == 77

    def test_global_get(self) -> None:
        ctx = self._make_ctx()
        vm = _run_instruction(0x23, 0, [], ctx)
        assert as_i32(vm.pop_typed()) == 100

    def test_global_set(self) -> None:
        ctx = self._make_ctx()
        vm = _run_instruction(0x24, 1, [i32(999)], ctx)
        assert ctx.globals[1].value == 999
