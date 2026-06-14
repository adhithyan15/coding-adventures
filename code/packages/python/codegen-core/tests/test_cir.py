"""Tests for CIRInstr — the codegen-core CompilerIR instruction type.

Covers construction, string representation, predicate methods, and
default field values.
"""

from __future__ import annotations

from codegen_core.cir import CIRInstr


class TestCIRInstrConstruction:
    """CIRInstr can be constructed with various field combinations."""

    def test_minimal_construction(self) -> None:
        """op and dest are required; srcs, type, deopt_to have defaults."""
        instr = CIRInstr(op="ret_void", dest=None)
        assert instr.op == "ret_void"
        assert instr.dest is None
        assert instr.srcs == []
        assert instr.type == "any"
        assert instr.deopt_to is None

    def test_full_construction(self) -> None:
        instr = CIRInstr(
            op="add_u8", dest="v0", srcs=["a", "b"], type="u8", deopt_to=5
        )
        assert instr.op == "add_u8"
        assert instr.dest == "v0"
        assert instr.srcs == ["a", "b"]
        assert instr.type == "u8"
        assert instr.deopt_to == 5

    def test_literal_srcs(self) -> None:
        """srcs can contain int, float, bool literals."""
        instr = CIRInstr(op="const_u8", dest="k", srcs=[42], type="u8")
        assert instr.srcs == [42]

    def test_srcs_default_independent(self) -> None:
        """Each CIRInstr gets its own srcs list (no shared default)."""
        a = CIRInstr(op="nop", dest=None)
        b = CIRInstr(op="nop", dest=None)
        a.srcs.append("x")
        assert "x" not in b.srcs


class TestCIRInstrStr:
    """__str__ renders op, srcs, type, and optional deopt annotation."""

    def test_void_op(self) -> None:
        instr = CIRInstr(op="ret_void", dest=None, srcs=[], type="void")
        assert str(instr) == "ret_void   [void]"

    def test_dest_and_srcs(self) -> None:
        instr = CIRInstr(op="add_u8", dest="v0", srcs=["a", "b"], type="u8")
        assert str(instr) == "v0 = add_u8 a, b  [u8]"

    def test_deopt_annotation(self) -> None:
        instr = CIRInstr(
            op="type_assert", dest=None, srcs=["x", "u8"], type="void", deopt_to=3
        )
        result = str(instr)
        assert "[deopt→3]" in result

    def test_no_deopt_when_none(self) -> None:
        instr = CIRInstr(op="const_u8", dest="k", srcs=[7], type="u8")
        assert "deopt" not in str(instr)


class TestCIRInstrPredicates:
    """is_type_guard() and is_generic() classify instructions correctly."""

    def test_type_guard_true(self) -> None:
        instr = CIRInstr(op="type_assert", dest=None, srcs=["x", "u8"], type="void", deopt_to=2)
        assert instr.is_type_guard()

    def test_type_assert_without_deopt_is_not_guard(self) -> None:
        """type_assert without deopt_to is a hard assertion, not a guard."""
        instr = CIRInstr(op="type_assert", dest=None, srcs=["x", "u8"], type="void")
        assert not instr.is_type_guard()

    def test_non_type_assert_is_not_guard(self) -> None:
        instr = CIRInstr(op="add_u8", dest="v", srcs=["a", "b"], type="u8")
        assert not instr.is_type_guard()

    def test_call_runtime_is_generic(self) -> None:
        instr = CIRInstr(op="call_runtime", dest="r", srcs=["generic_add", "a", "b"], type="any")
        assert instr.is_generic()

    def test_non_call_runtime_is_not_generic(self) -> None:
        instr = CIRInstr(op="add_u8", dest="v", srcs=["a", "b"], type="u8")
        assert not instr.is_generic()
