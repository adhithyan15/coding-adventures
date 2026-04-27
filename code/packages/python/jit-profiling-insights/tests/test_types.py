"""Tests for jit_profiling_insights.types — DispatchCost, TypeSite, ProfilingReport."""

from __future__ import annotations

import json

import pytest

from jit_profiling_insights.types import DispatchCost, ProfilingReport, TypeSite


class TestDispatchCost:
    def test_values(self):
        assert DispatchCost.NONE.value == "none"
        assert DispatchCost.GUARD.value == "guard"
        assert DispatchCost.GENERIC_CALL.value == "generic"
        assert DispatchCost.DEOPT.value == "deopt"

    def test_weights(self):
        assert DispatchCost.NONE.weight == 0
        assert DispatchCost.GUARD.weight == 1
        assert DispatchCost.GENERIC_CALL.weight == 10
        assert DispatchCost.DEOPT.weight == 100

    def test_str_mixin(self):
        # DispatchCost(str, Enum) — the value is usable as a string.
        assert DispatchCost.GUARD == "guard"
        assert DispatchCost.GENERIC_CALL == "generic"

    def test_ordering_weights(self):
        # Weights must be monotonically increasing with severity.
        weights = [
            DispatchCost.NONE.weight,
            DispatchCost.GUARD.weight,
            DispatchCost.GENERIC_CALL.weight,
            DispatchCost.DEOPT.weight,
        ]
        assert weights == sorted(weights)

    def test_json_serialisable(self):
        # Should not raise — the str mixin makes this work directly.
        payload = {"cost": DispatchCost.GUARD.value}
        assert json.dumps(payload) == '{"cost": "guard"}'


class TestTypeSite:
    @pytest.fixture
    def site(self):
        return TypeSite(
            function="fibonacci",
            instruction_op="type_assert",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GUARD,
            call_count=1_048_576,
            deopt_count=0,
            savings_description="would eliminate 1 type_assert per call",
        )

    def test_impact_guard(self, site):
        assert site.impact == 1_048_576 * 1  # GUARD weight = 1

    def test_impact_generic_call(self):
        site = TypeSite(
            function="main",
            instruction_op="call_runtime",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GENERIC_CALL,
            call_count=100,
            deopt_count=0,
            savings_description="...",
        )
        assert site.impact == 100 * 10

    def test_impact_deopt(self):
        site = TypeSite(
            function="foo",
            instruction_op="add",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.DEOPT,
            call_count=50,
            deopt_count=5,
            savings_description="...",
        )
        assert site.impact == 50 * 100

    def test_impact_none(self):
        site = TypeSite(
            function="foo",
            instruction_op="add",
            source_register="%r0",
            observed_type="u8",
            type_hint="u8",
            dispatch_cost=DispatchCost.NONE,
            call_count=100_000,
            deopt_count=0,
            savings_description="no overhead",
        )
        assert site.impact == 0

    def test_to_dict_keys(self, site):
        d = site.to_dict()
        assert "function" in d
        assert "instruction_op" in d
        assert "source_register" in d
        assert "observed_type" in d
        assert "type_hint" in d
        assert "dispatch_cost" in d
        assert "call_count" in d
        assert "deopt_count" in d
        assert "savings_description" in d
        assert "impact" in d

    def test_to_dict_dispatch_cost_is_string(self, site):
        d = site.to_dict()
        assert d["dispatch_cost"] == "guard"
        assert isinstance(d["dispatch_cost"], str)

    def test_to_dict_impact_matches_property(self, site):
        d = site.to_dict()
        assert d["impact"] == site.impact

    def test_to_dict_json_serialisable(self, site):
        d = site.to_dict()
        json_str = json.dumps(d)
        roundtrip = json.loads(json_str)
        assert roundtrip["function"] == "fibonacci"
        assert roundtrip["dispatch_cost"] == "guard"


class TestProfilingReport:
    @pytest.fixture
    def guard_site(self):
        return TypeSite(
            function="fibonacci",
            instruction_op="type_assert",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GUARD,
            call_count=1_000_000,
            deopt_count=0,
            savings_description="eliminates 1 branch/call",
        )

    @pytest.fixture
    def generic_site(self):
        return TypeSite(
            function="main",
            instruction_op="call_runtime",
            source_register="%r1",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GENERIC_CALL,
            call_count=3,
            deopt_count=0,
            savings_description="eliminates generic dispatch",
        )

    @pytest.fixture
    def deopt_site(self):
        return TypeSite(
            function="hot",
            instruction_op="add",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.DEOPT,
            call_count=100,
            deopt_count=10,
            savings_description="prevents interpreter fallback",
        )

    @pytest.fixture
    def report_with_sites(self, guard_site, generic_site):
        return ProfilingReport(
            program_name="fibonacci",
            total_instructions_executed=8_388_608,
            sites=[guard_site, generic_site],
        )

    def test_top_n_returns_slice(self, report_with_sites):
        assert len(report_with_sites.top_n(1)) == 1
        assert len(report_with_sites.top_n(10)) == 2  # only 2 sites

    def test_top_n_default(self, report_with_sites):
        top = report_with_sites.top_n()
        assert len(top) <= 10

    def test_functions_with_issues(self, report_with_sites, guard_site, generic_site):
        funcs = report_with_sites.functions_with_issues()
        assert "fibonacci" in funcs
        assert "main" in funcs

    def test_functions_with_issues_excludes_none(self):
        none_site = TypeSite(
            function="typed_fn",
            instruction_op="add",
            source_register="%r0",
            observed_type="u8",
            type_hint="u8",
            dispatch_cost=DispatchCost.NONE,
            call_count=100,
            deopt_count=0,
            savings_description="no overhead",
        )
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=100,
            sites=[none_site],
        )
        assert "typed_fn" not in report.functions_with_issues()

    def test_functions_with_issues_order_preserving(self, guard_site, generic_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=100,
            sites=[guard_site, generic_site],
        )
        funcs = report.functions_with_issues()
        assert funcs[0] == "fibonacci"
        assert funcs[1] == "main"

    def test_functions_with_issues_deduplication(self, guard_site):
        guard_site2 = TypeSite(
            function="fibonacci",
            instruction_op="cmp_lt",
            source_register="%r1",
            observed_type="bool",
            type_hint="any",
            dispatch_cost=DispatchCost.GUARD,
            call_count=500_000,
            deopt_count=0,
            savings_description="...",
        )
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=1_000_000,
            sites=[guard_site, guard_site2],
        )
        funcs = report.functions_with_issues()
        assert funcs.count("fibonacci") == 1

    def test_has_deopts_false(self, report_with_sites):
        assert not report_with_sites.has_deopts()

    def test_has_deopts_true(self, deopt_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=1_000,
            sites=[deopt_site],
        )
        assert report.has_deopts()

    def test_empty_report(self):
        report = ProfilingReport(program_name="empty", total_instructions_executed=0)
        assert report.top_n() == []
        assert report.functions_with_issues() == []
        assert not report.has_deopts()

    # ------------------------------------------------------------------
    # format_text
    # ------------------------------------------------------------------

    def test_format_text_contains_program_name(self, report_with_sites):
        text = report_with_sites.format_text()
        assert "fibonacci" in text

    def test_format_text_contains_total_instructions(self, report_with_sites):
        text = report_with_sites.format_text()
        assert "8,388,608" in text

    def test_format_text_contains_function_and_op(self, report_with_sites):
        text = report_with_sites.format_text()
        assert "fibonacci::type_assert" in text

    def test_format_text_high_impact_icon(self, guard_site):
        # 1,000,000 × 1 = 1,000,000 >= 100,000 → HIGH IMPACT
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=10_000_000,
            sites=[guard_site],
        )
        text = report.format_text()
        assert "🔴" in text

    def test_format_text_medium_impact_icon(self):
        site = TypeSite(
            function="foo",
            instruction_op="add",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GUARD,
            call_count=5_000,  # 5000 × 1 = 5000, MEDIUM
            deopt_count=0,
            savings_description="...",
        )
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=1_000_000,
            sites=[site],
        )
        text = report.format_text()
        assert "🟡" in text

    def test_format_text_low_impact_icon(self):
        site = TypeSite(
            function="foo",
            instruction_op="add",
            source_register="%r0",
            observed_type="int",
            type_hint="any",
            dispatch_cost=DispatchCost.GUARD,
            call_count=10,  # 10 × 1 = 10, LOW
            deopt_count=0,
            savings_description="...",
        )
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=1_000_000,
            sites=[site],
        )
        text = report.format_text()
        assert "🟢" in text

    def test_format_text_critical_deopt_icon(self, deopt_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=10_000,
            sites=[deopt_site],
        )
        text = report.format_text()
        assert "🚨" in text

    def test_format_text_no_overhead(self):
        report = ProfilingReport(
            program_name="perfect",
            total_instructions_executed=1_000,
            sites=[],
        )
        text = report.format_text()
        assert "✅" in text
        assert "all hot paths are typed" in text

    def test_format_text_no_deopts_message(self, report_with_sites):
        text = report_with_sites.format_text()
        assert "No deoptimisations occurred" in text

    def test_format_text_deopt_warning(self, deopt_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=10_000,
            sites=[deopt_site],
        )
        text = report.format_text()
        assert "deoptimisation" in text.lower()

    def test_format_text_deopt_count_shown(self, deopt_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=10_000,
            sites=[deopt_site],
        )
        text = report.format_text()
        assert "10" in text  # deopt_count = 10

    def test_format_text_summary_line(self, report_with_sites):
        text = report_with_sites.format_text()
        assert "Summary:" in text

    def test_format_text_zero_total_instructions(self, guard_site):
        report = ProfilingReport(
            program_name="test",
            total_instructions_executed=0,
            sites=[guard_site],
        )
        text = report.format_text()
        # Should not crash or show "% of total" percentages.
        assert "fibonacci" in text

    # ------------------------------------------------------------------
    # format_json
    # ------------------------------------------------------------------

    def test_format_json_valid_json(self, report_with_sites):
        json_str = report_with_sites.format_json()
        data = json.loads(json_str)
        assert isinstance(data, dict)

    def test_format_json_program_name(self, report_with_sites):
        data = json.loads(report_with_sites.format_json())
        assert data["program_name"] == "fibonacci"

    def test_format_json_total_instructions(self, report_with_sites):
        data = json.loads(report_with_sites.format_json())
        assert data["total_instructions_executed"] == 8_388_608

    def test_format_json_sites_list(self, report_with_sites):
        data = json.loads(report_with_sites.format_json())
        assert isinstance(data["sites"], list)
        assert len(data["sites"]) == 2

    def test_format_json_site_fields(self, report_with_sites):
        data = json.loads(report_with_sites.format_json())
        site = data["sites"][0]
        assert "function" in site
        assert "instruction_op" in site
        assert "dispatch_cost" in site
        assert "impact" in site

    def test_format_json_empty_report(self):
        report = ProfilingReport(program_name="empty", total_instructions_executed=0)
        data = json.loads(report.format_json())
        assert data["sites"] == []
