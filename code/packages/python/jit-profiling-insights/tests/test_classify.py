"""Tests for jit_profiling_insights.classify — _classify_cost, _find_root_register."""

from __future__ import annotations

import pytest

from interpreter_ir import IIRInstr

from jit_profiling_insights.classify import (
    _classify_cost,
    _find_root_register,
    _savings_description,
)
from jit_profiling_insights.types import DispatchCost

from tests.conftest import make_instr


class TestClassifyCost:
    # ------------------------------------------------------------------
    # NONE cases
    # ------------------------------------------------------------------

    def test_typed_hint_is_none(self):
        instr = make_instr("add", "%r0", ["%a", "%b"], "u8",
                           observation_count=1000)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_typed_hint_bool(self):
        instr = make_instr("cmp_lt", "%r1", ["%r0", 2], "bool",
                           observation_count=500)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_typed_hint_str(self):
        instr = make_instr("load_mem", "%r0", ["arg[0]"], "str",
                           observation_count=10)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_unobserved_any_is_none(self):
        # Even with type_hint "any", if it's not a type_assert/call_runtime
        # and has no deopt, it's NONE.
        instr = make_instr("add", "%r0", ["%a", "%b"], "any",
                           observation_count=100)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_call_runtime_without_generic_prefix_is_none(self):
        # call_runtime whose callee does NOT start with "generic_"
        instr = make_instr("call_runtime", "%r0", ["typed_add", "%r1"], "any",
                           observation_count=50)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_call_runtime_non_string_src_is_none(self):
        # src[0] is an integer literal — no generic_ check possible.
        instr = make_instr("call_runtime", "%r0", [42], "any",
                           observation_count=50)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_empty_srcs_is_none(self):
        instr = make_instr("call_runtime", "%r0", [], "any",
                           observation_count=10)
        assert _classify_cost(instr) == DispatchCost.NONE

    # ------------------------------------------------------------------
    # GUARD cases
    # ------------------------------------------------------------------

    def test_type_assert_is_guard(self):
        instr = make_instr("type_assert", None, ["%r0", "int"], "any",
                           observation_count=1_000_000)
        assert _classify_cost(instr) == DispatchCost.GUARD

    def test_type_assert_zero_observations_is_guard(self):
        # op == "type_assert" → GUARD regardless of observation count
        # (the guard IS the instruction, whether or not it ran yet).
        instr = make_instr("type_assert", None, ["%r0", "int"], "any",
                           observation_count=0)
        assert _classify_cost(instr) == DispatchCost.GUARD

    # ------------------------------------------------------------------
    # GENERIC_CALL cases
    # ------------------------------------------------------------------

    def test_call_runtime_generic_prefix_is_generic_call(self):
        instr = make_instr("call_runtime", "%r1",
                           ["generic_add", "%r0", "%r2"], "any",
                           observation_count=200_000)
        assert _classify_cost(instr) == DispatchCost.GENERIC_CALL

    def test_call_runtime_generic_call_callee_with_suffix(self):
        instr = make_instr("call_runtime", "%r1",
                           ["generic_dispatch_table", "%r0"], "any",
                           observation_count=1)
        assert _classify_cost(instr) == DispatchCost.GENERIC_CALL

    # ------------------------------------------------------------------
    # DEOPT cases
    # ------------------------------------------------------------------

    def test_deopt_count_greater_than_zero_is_deopt(self):
        instr = make_instr("add", "%r0", ["%r1"], "any",
                           observation_count=200, deopt_count=5)
        assert _classify_cost(instr) == DispatchCost.DEOPT

    def test_deopt_count_zero_is_not_deopt(self):
        instr = make_instr("add", "%r0", ["%r1"], "any",
                           observation_count=200, deopt_count=0)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_deopt_requires_positive_observation_count(self):
        # deopt_count > 0 but observation_count == 0 → NONE
        instr = make_instr("add", "%r0", ["%r1"], "any",
                           observation_count=0, deopt_count=5)
        assert _classify_cost(instr) == DispatchCost.NONE

    def test_deopt_count_missing_attribute_defaults_zero(self):
        # When deopt_count is NOT set as an attribute, getattr fallback = 0.
        instr = IIRInstr(op="add", dest="%r0", srcs=["%r1"], type_hint="any")
        instr.observation_count = 100
        # Do NOT set deopt_count — test the getattr fallback.
        assert _classify_cost(instr) == DispatchCost.NONE


class TestFindRootRegister:
    def test_no_chain_returns_primary_src(self):
        instr = make_instr("type_assert", None, ["%r0", "int"], "any")
        result = _find_root_register(instr, [instr], 0)
        assert result == "%r0"

    def test_traces_through_load_mem(self):
        # load_mem defines %r0 from arg[0] (untyped).
        load = make_instr("load_mem", "%r0", ["arg[0]"], "any")
        guard = make_instr("type_assert", None, ["%r0", "int"], "any")
        instrs = [load, guard]
        # Root should trace back past %r0 to arg[0].
        result = _find_root_register(guard, instrs, 1)
        assert result == "arg[0]"

    def test_stops_at_typed_def(self):
        # load_mem with a concrete type — not an untyped chain.
        load = make_instr("load_mem", "%r0", ["arg[0]"], "u8")
        guard = make_instr("type_assert", None, ["%r0", "int"], "any")
        instrs = [load, guard]
        # Should stop at %r0 because the defining instruction is typed.
        result = _find_root_register(guard, instrs, 1)
        assert result == "%r0"

    def test_traces_through_load_reg(self):
        load_reg = make_instr("load_reg", "%r1", ["%r0"], "any")
        guard = make_instr("type_assert", None, ["%r1", "int"], "any")
        instrs = [load_reg, guard]
        result = _find_root_register(guard, instrs, 1)
        assert result == "%r0"

    def test_no_srcs_returns_dest(self):
        instr = make_instr("type_assert", "%r0", [], "any")
        result = _find_root_register(instr, [instr], 0)
        assert result == "%r0"

    def test_no_srcs_no_dest_returns_unknown(self):
        instr = IIRInstr(op="ret", dest=None, srcs=[], type_hint="any")
        result = _find_root_register(instr, [instr], 0)
        assert result == "%unknown"

    def test_literal_src_returns_string_of_literal(self):
        instr = make_instr("type_assert", None, [42, "int"], "any")
        result = _find_root_register(instr, [instr], 0)
        assert result == "42"

    def test_avoids_cycle(self):
        # Construct a cycle: %r0 defined by load_mem %r0 (pathological).
        load = make_instr("load_mem", "%r0", ["%r0"], "any")
        guard = make_instr("type_assert", None, ["%r0", "int"], "any")
        instrs = [load, guard]
        # Should not loop forever.
        result = _find_root_register(guard, instrs, 1)
        assert isinstance(result, str)

    def test_stops_at_arithmetic_op(self):
        # Arithmetic op defines %r0 — not a simple load chain.
        add = make_instr("add", "%r0", ["%r1", "%r2"], "any")
        guard = make_instr("type_assert", None, ["%r0", "int"], "any")
        instrs = [add, guard]
        # Should stop at %r0, not trace into the add's operands.
        result = _find_root_register(guard, instrs, 1)
        assert result == "%r0"

    def test_only_looks_before_current_index(self):
        guard = make_instr("type_assert", None, ["%r0", "int"], "any")
        load = make_instr("load_mem", "%r0", ["arg[0]"], "any")
        # load is AFTER guard in the instruction list — should not be visible.
        instrs = [guard, load]
        result = _find_root_register(guard, instrs, 0)
        assert result == "%r0"


class TestSavingsDescription:
    def test_guard(self):
        desc = _savings_description(DispatchCost.GUARD, 1_000_000, "type_assert")
        assert "type_assert" in desc
        assert "1,000,000" in desc

    def test_generic_call(self):
        desc = _savings_description(DispatchCost.GENERIC_CALL, 500, "call_runtime")
        assert "generic" in desc.lower() or "dispatch" in desc
        assert "500" in desc

    def test_deopt(self):
        desc = _savings_description(DispatchCost.DEOPT, 10, "add")
        assert "fallback" in desc or "interpreter" in desc
        assert "10" in desc

    def test_none(self):
        desc = _savings_description(DispatchCost.NONE, 100, "add")
        assert "no overhead" in desc
