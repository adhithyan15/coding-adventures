"""Tests for vm_type_suggestions.suggest() — the main entry point."""

from __future__ import annotations

import json

import pytest

from interpreter_ir import IIRFunction, IIRInstr

from vm_type_suggestions import suggest
from vm_type_suggestions.types import Confidence, SuggestionReport

from tests.conftest import make_function, make_load_mem


class TestSuggestEmpty:
    def test_empty_fn_list(self):
        report = suggest([])
        assert isinstance(report, SuggestionReport)
        assert report.suggestions == []
        assert report.total_calls == 0

    def test_default_program_name(self):
        report = suggest([])
        assert report.program_name == "program"

    def test_custom_program_name(self):
        report = suggest([], program_name="my_app")
        assert report.program_name == "my_app"

    def test_function_with_no_params(self):
        fn = make_function("no_params", params=[], instrs=[])
        report = suggest([fn])
        assert report.suggestions == []

    def test_function_with_no_instructions(self):
        fn = make_function("empty_fn", params=[("x", "any")], instrs=[])
        report = suggest([fn])
        assert len(report.suggestions) == 1
        assert report.suggestions[0].confidence == Confidence.NO_DATA


class TestSuggestCertain:
    def test_single_certain_param(self, fibonacci_fn):
        report = suggest([fibonacci_fn])
        assert len(report.suggestions) == 1
        s = report.suggestions[0]
        assert s.function == "fibonacci"
        assert s.param_name == "n"
        assert s.param_index == 0
        assert s.observed_type == "u8"
        assert s.call_count == 1_048_576
        assert s.confidence == Confidence.CERTAIN
        assert s.suggestion == "declare 'n: u8'"

    def test_two_certain_params(self, add_fn):
        report = suggest([add_fn])
        assert len(report.suggestions) == 2
        assert all(s.confidence == Confidence.CERTAIN for s in report.suggestions)
        assert report.suggestions[0].param_name == "a"
        assert report.suggestions[1].param_name == "b"

    def test_total_calls_accumulates_certain(self, add_fn):
        report = suggest([add_fn])
        # Both params have 1,000,000 observations each
        assert report.total_calls == 2_000_000

    def test_suggestion_text_includes_type(self, fibonacci_fn):
        report = suggest([fibonacci_fn])
        assert report.suggestions[0].suggestion == "declare 'n: u8'"

    def test_actionable_returns_certain_only(self, add_fn):
        report = suggest([add_fn])
        actionable = report.actionable()
        assert len(actionable) == 2

    def test_different_types(self):
        fn = make_function(
            "greet",
            params=[("msg", "any")],
            instrs=[make_load_mem(0, observed_type="str", count=50)],
        )
        report = suggest([fn])
        s = report.suggestions[0]
        assert s.observed_type == "str"
        assert s.suggestion == "declare 'msg: str'"


class TestSuggestMixed:
    def test_mixed_param_is_classified(self, mixed_fn):
        report = suggest([mixed_fn])
        assert len(report.suggestions) == 1
        s = report.suggestions[0]
        assert s.confidence == Confidence.MIXED
        assert s.suggestion is None
        assert s.observed_type == "polymorphic"

    def test_mixed_param_not_in_actionable(self, mixed_fn):
        report = suggest([mixed_fn])
        assert report.actionable() == []

    def test_mixed_call_count_recorded(self, mixed_fn):
        report = suggest([mixed_fn])
        assert report.suggestions[0].call_count == 3

    def test_mixed_does_not_add_to_total_calls(self, mixed_fn):
        report = suggest([mixed_fn])
        # MIXED does not contribute to total_calls (only CERTAIN does)
        assert report.total_calls == 0


class TestSuggestNoData:
    def test_never_called_fn_is_no_data(self, never_called_fn):
        report = suggest([never_called_fn])
        assert len(report.suggestions) == 1
        s = report.suggestions[0]
        assert s.confidence == Confidence.NO_DATA
        assert s.call_count == 0
        assert s.suggestion is None
        assert s.observed_type is None

    def test_no_loader_fn_is_no_data(self, no_loader_fn):
        report = suggest([no_loader_fn])
        assert len(report.suggestions) == 1
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_no_data_not_in_actionable(self, never_called_fn):
        report = suggest([never_called_fn])
        assert report.actionable() == []


class TestSuggestTypedParams:
    def test_typed_params_skipped(self, typed_fn):
        report = suggest([typed_fn])
        assert report.suggestions == []

    def test_mixed_typed_and_untyped(self):
        fn = make_function(
            "semi_typed",
            params=[("a", "u8"), ("b", "any")],
            instrs=[
                IIRInstr("load_mem", "%r0", ["arg[0]"], "u8"),  # typed — skipped
                make_load_mem(1, observed_type="u8", count=100),  # untyped — included
            ],
        )
        report = suggest([fn])
        assert len(report.suggestions) == 1
        assert report.suggestions[0].param_name == "b"
        assert report.suggestions[0].param_index == 1


class TestSuggestMultipleFunctions:
    def test_multiple_functions(self, fibonacci_fn, add_fn):
        report = suggest([fibonacci_fn, add_fn])
        fn_names = {s.function for s in report.suggestions}
        assert "fibonacci" in fn_names
        assert "add" in fn_names
        assert len(report.suggestions) == 3  # 1 from fibonacci + 2 from add

    def test_mixed_confidence_across_functions(self, fibonacci_fn, mixed_fn, never_called_fn):
        report = suggest([fibonacci_fn, mixed_fn, never_called_fn])
        confidences = {s.confidence for s in report.suggestions}
        assert Confidence.CERTAIN in confidences
        assert Confidence.MIXED in confidences
        assert Confidence.NO_DATA in confidences

    def test_by_function_groups(self, fibonacci_fn, add_fn):
        report = suggest([fibonacci_fn, add_fn])
        grouped = report.by_function()
        assert len(grouped["fibonacci"]) == 1
        assert len(grouped["add"]) == 2

    def test_total_calls_only_from_certain(self, fibonacci_fn, mixed_fn):
        report = suggest([fibonacci_fn, mixed_fn])
        # Only fibonacci's certain param contributes
        assert report.total_calls == 1_048_576


class TestSuggestEdgeCases:
    def test_load_mem_with_non_string_src(self):
        """load_mem whose srcs[0] is an integer literal — should be ignored."""
        instr = IIRInstr("load_mem", "%r0", [42], "any")
        instr.observed_type = "u8"
        instr.observation_count = 100
        fn = make_function("fn", params=[("x", "any")], instrs=[instr])
        report = suggest([fn])
        # The load_mem doesn't match "arg[N]" pattern → NO_DATA
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_load_mem_with_non_arg_src(self):
        """load_mem whose src is a register name, not arg[N]."""
        instr = IIRInstr("load_mem", "%r0", ["%r1"], "any")
        instr.observed_type = "u8"
        instr.observation_count = 100
        fn = make_function("fn", params=[("x", "any")], instrs=[instr])
        report = suggest([fn])
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_load_mem_with_invalid_index(self):
        """load_mem with 'arg[abc]' — non-integer index should be skipped."""
        instr = IIRInstr("load_mem", "%r0", ["arg[abc]"], "any")
        instr.observed_type = "u8"
        instr.observation_count = 100
        fn = make_function("fn", params=[("x", "any")], instrs=[instr])
        report = suggest([fn])
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_duplicate_load_mem_uses_first(self):
        """Two load_mem [arg[0]] instructions — first one wins."""
        first = make_load_mem(0, observed_type="u8", count=1_000)
        second = make_load_mem(0, observed_type="str", count=500)
        fn = make_function("fn", params=[("x", "any")], instrs=[first, second])
        report = suggest([fn])
        # Only one suggestion for arg[0], using the first (u8)
        assert len(report.suggestions) == 1
        assert report.suggestions[0].observed_type == "u8"

    def test_param_index_out_of_range(self):
        """load_mem for arg[5] when function only has 1 param — ignored."""
        instr = IIRInstr("load_mem", "%r0", ["arg[5]"], "any")
        instr.observed_type = "u8"
        instr.observation_count = 100
        fn = make_function("fn", params=[("x", "any")], instrs=[instr])
        report = suggest([fn])
        # arg[5] doesn't correspond to any parameter
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_empty_srcs_on_load_mem(self):
        """load_mem with no srcs at all — skipped."""
        instr = IIRInstr("load_mem", "%r0", [], "any")
        instr.observed_type = "u8"
        instr.observation_count = 100
        fn = make_function("fn", params=[("x", "any")], instrs=[instr])
        report = suggest([fn])
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_non_load_mem_instructions_ignored(self):
        """Only load_mem instructions are scanned; others are skipped."""
        store = IIRInstr("store_mem", None, ["arg[0]", "%r0"], "any")
        fn = make_function("fn", params=[("x", "any")], instrs=[store])
        report = suggest([fn])
        assert report.suggestions[0].confidence == Confidence.NO_DATA

    def test_format_text_integration(self, add_fn):
        report = suggest([add_fn], program_name="math")
        text = report.format_text()
        assert "math" in text
        assert "add" in text
        assert "declare 'a: u8'" in text

    def test_format_json_integration(self, fibonacci_fn):
        report = suggest([fibonacci_fn], program_name="fib")
        data = json.loads(report.format_json())
        assert data["program_name"] == "fib"
        assert len(data["suggestions"]) == 1
        assert data["suggestions"][0]["confidence"] == "certain"
