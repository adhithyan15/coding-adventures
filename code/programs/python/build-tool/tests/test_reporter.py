"""Tests for the reporter module."""

from __future__ import annotations

from io import StringIO

import pytest

from build_tool.executor import BuildResult
from build_tool.reporter import _format_duration, format_report, print_report


class TestFormatDuration:
    def test_zero_seconds(self):
        assert _format_duration(0.0) == "-"

    def test_tiny_duration(self):
        assert _format_duration(0.001) == "-"

    def test_normal_duration(self):
        assert _format_duration(2.345) == "2.3s"

    def test_large_duration(self):
        assert _format_duration(120.5) == "120.5s"


class TestFormatReport:
    def test_empty_results(self):
        report = format_report({})
        assert "No packages processed" in report

    def test_built_package(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="built",
                duration=2.3,
            ),
        }
        report = format_report(results)
        assert "BUILT" in report
        assert "2.3s" in report
        assert "python/test" in report

    def test_failed_package(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="failed",
                duration=0.5,
            ),
        }
        report = format_report(results)
        assert "FAILED" in report

    def test_skipped_package(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="skipped",
            ),
        }
        report = format_report(results)
        assert "SKIPPED" in report

    def test_dep_skipped_package(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="dep-skipped",
            ),
        }
        report = format_report(results)
        assert "DEP-SKIP" in report
        assert "dep failed" in report

    def test_would_build_package(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="would-build",
            ),
        }
        report = format_report(results)
        assert "WOULD-BUILD" in report

    def test_summary_line(self):
        results = {
            "python/a": BuildResult(package_name="python/a", status="built", duration=1.0),
            "python/b": BuildResult(package_name="python/b", status="skipped"),
            "python/c": BuildResult(package_name="python/c", status="failed", duration=0.5),
            "python/d": BuildResult(package_name="python/d", status="dep-skipped"),
        }
        report = format_report(results)
        assert "4 packages" in report
        assert "1 built" in report
        assert "1 skipped" in report
        assert "1 failed" in report
        assert "1 dep-skipped" in report

    def test_sorted_output(self):
        results = {
            "python/z": BuildResult(package_name="python/z", status="built", duration=1.0),
            "python/a": BuildResult(package_name="python/a", status="built", duration=1.0),
        }
        report = format_report(results)
        lines = report.strip().split("\n")
        # Find the data lines (after header)
        data_lines = [l for l in lines if l.startswith("python/")]
        assert data_lines[0].startswith("python/a")
        assert data_lines[1].startswith("python/z")


class TestPrintReport:
    def test_prints_to_stream(self):
        results = {
            "python/test": BuildResult(
                package_name="python/test",
                status="built",
                duration=1.0,
            ),
        }
        buf = StringIO()
        print_report(results, file=buf)
        output = buf.getvalue()
        assert "python/test" in output
        assert "BUILT" in output
