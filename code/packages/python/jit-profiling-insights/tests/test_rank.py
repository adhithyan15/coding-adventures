"""Tests for jit_profiling_insights.rank — rank_sites, total_instructions."""

from __future__ import annotations

import pytest

from interpreter_ir import IIRFunction

from jit_profiling_insights.rank import rank_sites, total_instructions
from jit_profiling_insights.types import DispatchCost, TypeSite

from tests.conftest import make_function, make_instr


def _make_site(
    function: str,
    cost: DispatchCost,
    call_count: int,
    deopt_count: int = 0,
) -> TypeSite:
    return TypeSite(
        function=function,
        instruction_op="add",
        source_register="%r0",
        observed_type="int",
        type_hint="any",
        dispatch_cost=cost,
        call_count=call_count,
        deopt_count=deopt_count,
        savings_description="...",
    )


class TestRankSites:
    def test_empty_list(self):
        assert rank_sites([]) == []

    def test_single_site_unchanged(self):
        site = _make_site("fn", DispatchCost.GUARD, 100)
        result = rank_sites([site])
        assert result == [site]

    def test_higher_impact_first(self):
        low = _make_site("fn", DispatchCost.GUARD, 100)     # impact = 100
        high = _make_site("fn", DispatchCost.GUARD, 10_000)  # impact = 10_000
        result = rank_sites([low, high])
        assert result[0] is high
        assert result[1] is low

    def test_deopt_beats_guard_same_call_count(self):
        # DEOPT × 100 >> GUARD × 1 at same call count
        guard = _make_site("fn", DispatchCost.GUARD, 1_000)    # impact = 1_000
        deopt = _make_site("fn", DispatchCost.DEOPT, 1_000)    # impact = 100_000
        result = rank_sites([guard, deopt])
        assert result[0] is deopt

    def test_generic_call_beats_guard_same_call_count(self):
        guard = _make_site("fn", DispatchCost.GUARD, 1_000)         # impact = 1_000
        generic = _make_site("fn", DispatchCost.GENERIC_CALL, 1_000)  # impact = 10_000
        result = rank_sites([guard, generic])
        assert result[0] is generic

    def test_none_sites_rank_last(self):
        none_site = _make_site("fn", DispatchCost.NONE, 1_000_000)  # impact = 0
        guard = _make_site("fn", DispatchCost.GUARD, 1)               # impact = 1
        result = rank_sites([none_site, guard])
        assert result[0] is guard
        assert result[1] is none_site

    def test_equal_impact_higher_weight_first(self):
        # Two sites with same call_count × weight:
        # GENERIC_CALL × 10 calls = 100
        # DEOPT × 1 call = 100
        # DEOPT has higher weight, so it ranks first.
        generic = _make_site("fn", DispatchCost.GENERIC_CALL, 10)
        deopt = _make_site("fn", DispatchCost.DEOPT, 1)
        result = rank_sites([generic, deopt])
        assert result[0] is deopt

    def test_sorts_in_place(self):
        sites = [
            _make_site("fn", DispatchCost.GUARD, 10),
            _make_site("fn", DispatchCost.GUARD, 100),
        ]
        original_list = sites
        result = rank_sites(sites)
        assert result is original_list  # same list object
        assert result[0].call_count == 100

    def test_three_sites_correct_order(self):
        a = _make_site("fn", DispatchCost.GUARD, 500_000)          # 500_000
        b = _make_site("fn", DispatchCost.GENERIC_CALL, 100_000)   # 1_000_000
        c = _make_site("fn", DispatchCost.DEOPT, 50)               # 5_000
        result = rank_sites([a, b, c])
        assert result[0] is b
        assert result[1] is a
        assert result[2] is c


class TestTotalInstructions:
    def test_empty_function_list(self):
        assert total_instructions([]) == 0

    def test_empty_function_no_instructions(self):
        fn = make_function("empty", [])
        assert total_instructions([fn]) == 0

    def test_sums_observation_counts(self):
        instrs = [
            make_instr("add", "%r0", [], "any", observation_count=100),
            make_instr("add", "%r1", [], "any", observation_count=200),
            make_instr("ret", None, [], "any", observation_count=50),
        ]
        fn = make_function("fn", instrs)
        assert total_instructions([fn]) == 350

    def test_sums_across_multiple_functions(self):
        fn1 = make_function("a", [
            make_instr("add", "%r0", [], "any", observation_count=1_000),
        ])
        fn2 = make_function("b", [
            make_instr("add", "%r0", [], "any", observation_count=7_000),
        ])
        assert total_instructions([fn1, fn2]) == 8_000

    def test_unobserved_instructions_contribute_zero(self):
        instrs = [
            make_instr("add", "%r0", [], "any", observation_count=0),
            make_instr("add", "%r1", [], "any", observation_count=0),
        ]
        fn = make_function("fn", instrs)
        assert total_instructions([fn]) == 0
