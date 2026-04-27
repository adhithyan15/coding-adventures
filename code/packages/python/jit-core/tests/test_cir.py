"""Tests for CIRInstr."""

from __future__ import annotations

from jit_core.cir import CIRInstr


class TestCIRInstrStr:
    def test_str_with_dest(self):
        instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
        s = str(instr)
        assert "v0 =" in s
        assert "add_u8" in s
        assert "a, b" in s
        assert "[u8]" in s

    def test_str_without_dest(self):
        instr = CIRInstr(op="ret_void", dest=None, srcs=[], type="void")
        s = str(instr)
        assert "=" not in s
        assert "ret_void" in s

    def test_str_with_deopt(self):
        instr = CIRInstr(op="type_assert", dest=None, srcs=["x", "u8"], type="void", deopt_to=3)
        s = str(instr)
        assert "deopt→3" in s

    def test_str_without_deopt(self):
        instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
        s = str(instr)
        assert "deopt" not in s


class TestIsTypeGuard:
    def test_type_assert_with_deopt_is_guard(self):
        instr = CIRInstr(op="type_assert", dest=None, srcs=["x", "u8"], type="void", deopt_to=5)
        assert instr.is_type_guard() is True

    def test_type_assert_without_deopt_is_not_guard(self):
        instr = CIRInstr(op="type_assert", dest=None, srcs=["x", "u8"], type="void")
        assert instr.is_type_guard() is False

    def test_non_type_assert_with_deopt_is_not_guard(self):
        instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8", deopt_to=1)
        assert instr.is_type_guard() is False

    def test_plain_instr_is_not_guard(self):
        instr = CIRInstr(op="ret_u8", dest=None, srcs=["v0"], type="u8")
        assert instr.is_type_guard() is False


class TestIsGeneric:
    def test_call_runtime_is_generic(self):
        instr = CIRInstr(op="call_runtime", dest="v0", srcs=["generic_add", "a", "b"], type="any")
        assert instr.is_generic() is True

    def test_typed_op_is_not_generic(self):
        instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
        assert instr.is_generic() is False

    def test_ret_is_not_generic(self):
        instr = CIRInstr(op="ret_u8", dest=None, srcs=["v0"], type="u8")
        assert instr.is_generic() is False
