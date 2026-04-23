"""Tests for vm_type_suggestions.types — Confidence, ParamSuggestion, SuggestionReport."""

from __future__ import annotations

import json

import pytest

from vm_type_suggestions.types import Confidence, ParamSuggestion, SuggestionReport


def _make_certain(function="add", param_name="a", param_index=0, count=1_000_000):
    return ParamSuggestion(
        function=function,
        param_name=param_name,
        param_index=param_index,
        observed_type="u8",
        call_count=count,
        confidence=Confidence.CERTAIN,
        suggestion=f"declare '{param_name}: u8'",
    )


def _make_mixed(function="fmt", param_name="s", param_index=0):
    return ParamSuggestion(
        function=function,
        param_name="s",
        param_index=0,
        observed_type="polymorphic",
        call_count=10,
        confidence=Confidence.MIXED,
        suggestion=None,
    )


def _make_no_data(function="unused", param_name="x", param_index=0):
    return ParamSuggestion(
        function=function,
        param_name=param_name,
        param_index=param_index,
        observed_type=None,
        call_count=0,
        confidence=Confidence.NO_DATA,
        suggestion=None,
    )


class TestConfidence:
    def test_values(self):
        assert Confidence.CERTAIN.value == "certain"
        assert Confidence.MIXED.value == "mixed"
        assert Confidence.NO_DATA.value == "no_data"

    def test_str_mixin(self):
        assert Confidence.CERTAIN == "certain"
        assert Confidence.MIXED == "mixed"

    def test_json_serialisable(self):
        payload = {"confidence": Confidence.CERTAIN.value}
        assert json.dumps(payload) == '{"confidence": "certain"}'


class TestParamSuggestion:
    def test_to_dict_certain(self):
        s = _make_certain()
        d = s.to_dict()
        assert d["function"] == "add"
        assert d["param_name"] == "a"
        assert d["param_index"] == 0
        assert d["observed_type"] == "u8"
        assert d["call_count"] == 1_000_000
        assert d["confidence"] == "certain"
        assert d["suggestion"] == "declare 'a: u8'"

    def test_to_dict_mixed(self):
        s = _make_mixed()
        d = s.to_dict()
        assert d["confidence"] == "mixed"
        assert d["suggestion"] is None
        assert d["observed_type"] == "polymorphic"

    def test_to_dict_no_data(self):
        s = _make_no_data()
        d = s.to_dict()
        assert d["confidence"] == "no_data"
        assert d["observed_type"] is None
        assert d["call_count"] == 0

    def test_to_dict_json_serialisable(self):
        s = _make_certain()
        json_str = json.dumps(s.to_dict())
        roundtrip = json.loads(json_str)
        assert roundtrip["function"] == "add"
        assert roundtrip["confidence"] == "certain"


class TestSuggestionReport:
    @pytest.fixture
    def full_report(self):
        return SuggestionReport(
            program_name="test",
            total_calls=1_000_000,
            suggestions=[
                _make_certain("add", "a", 0),
                _make_certain("add", "b", 1),
                _make_mixed("fmt", "s", 0),
                _make_no_data("unused", "x", 0),
            ],
        )

    def test_actionable_returns_only_certain(self, full_report):
        actionable = full_report.actionable()
        assert len(actionable) == 2
        assert all(s.confidence == Confidence.CERTAIN for s in actionable)

    def test_actionable_empty_when_no_certain(self):
        report = SuggestionReport(
            program_name="test",
            total_calls=0,
            suggestions=[_make_mixed(), _make_no_data()],
        )
        assert report.actionable() == []

    def test_by_function_groups_correctly(self, full_report):
        grouped = full_report.by_function()
        assert "add" in grouped
        assert len(grouped["add"]) == 2
        assert "fmt" in grouped
        assert len(grouped["fmt"]) == 1

    def test_by_function_preserves_order(self):
        a = _make_certain("first", "x", 0)
        b = _make_certain("second", "y", 0)
        report = SuggestionReport(program_name="t", total_calls=0, suggestions=[a, b])
        keys = list(report.by_function().keys())
        assert keys == ["first", "second"]

    def test_empty_report(self):
        report = SuggestionReport(program_name="empty", total_calls=0)
        assert report.actionable() == []
        assert report.by_function() == {}

    # ------------------------------------------------------------------
    # format_text
    # ------------------------------------------------------------------

    def test_format_text_contains_program_name(self, full_report):
        assert "test" in full_report.format_text()

    def test_format_text_contains_total_calls(self, full_report):
        assert "1,000,000" in full_report.format_text()

    def test_format_text_certain_shows_checkmark(self, full_report):
        text = full_report.format_text()
        assert "✅" in text

    def test_format_text_certain_shows_suggestion(self, full_report):
        text = full_report.format_text()
        assert "declare 'a: u8'" in text
        assert "declare 'b: u8'" in text

    def test_format_text_mixed_shows_warning(self, full_report):
        text = full_report.format_text()
        assert "⚠️" in text
        assert "mixed types" in text

    def test_format_text_no_data_shows_info(self, full_report):
        text = full_report.format_text()
        assert "ℹ️" in text or "no profiling data" in text

    def test_format_text_summary_line(self, full_report):
        text = full_report.format_text()
        assert "Summary:" in text
        assert "2 of 4" in text

    def test_format_text_empty_report(self):
        report = SuggestionReport(program_name="empty", total_calls=0)
        text = report.format_text()
        assert "✅" in text
        assert "everything is already typed" in text

    def test_format_text_singular_noun(self):
        report = SuggestionReport(
            program_name="t",
            total_calls=1,
            suggestions=[_make_certain("f", "a", 0, 1)],
        )
        text = report.format_text()
        assert "1 of 1 untyped parameter can" in text

    def test_format_text_function_shows_call_count(self, full_report):
        text = full_report.format_text()
        assert "add" in text
        assert "1,000,000" in text

    def test_format_text_no_data_zero_count(self):
        report = SuggestionReport(
            program_name="t",
            total_calls=0,
            suggestions=[_make_no_data()],
        )
        text = report.format_text()
        assert "0 calls" in text or "no profiling" in text

    # ------------------------------------------------------------------
    # format_json
    # ------------------------------------------------------------------

    def test_format_json_valid(self, full_report):
        data = json.loads(full_report.format_json())
        assert isinstance(data, dict)

    def test_format_json_program_name(self, full_report):
        data = json.loads(full_report.format_json())
        assert data["program_name"] == "test"

    def test_format_json_total_calls(self, full_report):
        data = json.loads(full_report.format_json())
        assert data["total_calls"] == 1_000_000

    def test_format_json_suggestions_list(self, full_report):
        data = json.loads(full_report.format_json())
        assert isinstance(data["suggestions"], list)
        assert len(data["suggestions"]) == 4

    def test_format_json_confidence_is_string(self, full_report):
        data = json.loads(full_report.format_json())
        for s in data["suggestions"]:
            assert isinstance(s["confidence"], str)

    def test_format_json_empty(self):
        report = SuggestionReport(program_name="empty", total_calls=0)
        data = json.loads(report.format_json())
        assert data["suggestions"] == []
