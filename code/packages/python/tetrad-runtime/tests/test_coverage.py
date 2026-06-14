"""End-to-end coverage tests (LANG18).

These tests verify the full coverage pipeline:

    Tetrad source
         │  TetradRuntime.run_with_coverage()
         ▼  → compile_with_debug → (IIRModule, sidecar)
         │  → VMCore with coverage mode on
         │  → vm.coverage_data()  (IIR instruction indices)
         ▼  → build_report(iir_cov, sidecar)
    LineCoverageReport
         │  .lines_for_file(path)      → covered line numbers
         │  .total_lines_covered()     → int
         │  .covered_lines             → list[CoveredLine]

Source programs are written with one statement per line so the mapping
from line number to "was this line executed" is unambiguous.
"""

from __future__ import annotations

import pytest

from tetrad_runtime import CoveredLine, LineCoverageReport, TetradRuntime
from tetrad_runtime.coverage import build_report

# ---------------------------------------------------------------------------
# Source programs — predictable line structure
# ---------------------------------------------------------------------------

# A minimal single-function program.  Every statement is on its own line:
#   Line 1: fn main() -> u8 {
#   Line 2:     return 42;
#   Line 3: }
SIMPLE_SRC = "fn main() -> u8 {\n    return 42;\n}"

# Two-function program (add + main):
#   Line 1: fn add(a: u8, b: u8) -> u8 { return a + b; }
#   Line 2: fn main() -> u8 { return add(10, 20); }
TWO_FN_SRC = (
    "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
    "fn main() -> u8 { return add(10, 20); }"
)

# Single-line program (used to verify empty-report edge cases are avoided).
SINGLE_LINE_SRC = "fn main() -> u8 { return 1; }"

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

SOURCE_PATH = "test.tetrad"


def _run(source: str) -> LineCoverageReport:
    rt = TetradRuntime()
    return rt.run_with_coverage(source, SOURCE_PATH)


# ---------------------------------------------------------------------------
# Return-type and structure tests
# ---------------------------------------------------------------------------

class TestReturnType:
    """run_with_coverage returns a proper LineCoverageReport."""

    def test_returns_line_coverage_report(self) -> None:
        report = _run(SIMPLE_SRC)
        assert isinstance(report, LineCoverageReport)

    def test_covered_lines_is_list(self) -> None:
        report = _run(SIMPLE_SRC)
        assert isinstance(report.covered_lines, list)

    def test_covered_line_items_are_covered_line_instances(self) -> None:
        report = _run(SIMPLE_SRC)
        for item in report.covered_lines:
            assert isinstance(item, CoveredLine)

    def test_source_path_matches(self) -> None:
        """Every CoveredLine.file must equal the source_path passed in."""
        report = _run(SIMPLE_SRC)
        for cl in report.covered_lines:
            assert cl.file == SOURCE_PATH


# ---------------------------------------------------------------------------
# Basic coverage — simple one-function program
# ---------------------------------------------------------------------------

class TestBasicCoverage:
    """At least some lines are covered for any complete program."""

    def test_at_least_one_line_covered(self) -> None:
        report = _run(SIMPLE_SRC)
        assert report.total_lines_covered() > 0

    def test_lines_for_file_returns_list(self) -> None:
        report = _run(SIMPLE_SRC)
        lines = report.lines_for_file(SOURCE_PATH)
        assert isinstance(lines, list)

    def test_lines_for_file_sorted(self) -> None:
        report = _run(TWO_FN_SRC)
        lines = report.lines_for_file(SOURCE_PATH)
        assert lines == sorted(lines)

    def test_lines_for_unknown_file_returns_empty(self) -> None:
        report = _run(SIMPLE_SRC)
        assert report.lines_for_file("nonexistent.tetrad") == []

    def test_iir_hit_count_positive(self) -> None:
        """Every CoveredLine must have a positive iir_hit_count."""
        report = _run(SIMPLE_SRC)
        for cl in report.covered_lines:
            assert cl.iir_hit_count > 0


# ---------------------------------------------------------------------------
# Two-function program — both functions covered
# ---------------------------------------------------------------------------

class TestTwoFunctionCoverage:
    """When main calls add, lines from both functions appear in the report."""

    def test_lines_present_for_both_functions(self) -> None:
        """Line 1 (add) and line 2 (main) should both be covered."""
        report = _run(TWO_FN_SRC)
        lines = report.lines_for_file(SOURCE_PATH)
        # Both the add function (line 1) and main (line 2) were executed.
        assert 1 in lines
        assert 2 in lines

    def test_total_covers_multiple_lines(self) -> None:
        report = _run(TWO_FN_SRC)
        assert report.total_lines_covered() >= 2

    def test_files_helper_returns_source_path(self) -> None:
        report = _run(TWO_FN_SRC)
        assert SOURCE_PATH in report.files()

    def test_files_helper_returns_sorted_unique_list(self) -> None:
        report = _run(TWO_FN_SRC)
        files = report.files()
        assert files == sorted(set(files))


# ---------------------------------------------------------------------------
# build_report directly — unit tests against the projection function
# ---------------------------------------------------------------------------

class TestBuildReport:
    """build_report composes raw IIR coverage with a sidecar correctly."""

    def _get_sidecar(self, source: str) -> bytes:
        rt = TetradRuntime()
        _module, sidecar = rt.compile_with_debug(source, SOURCE_PATH)
        return sidecar

    def test_empty_iir_coverage_gives_empty_report(self) -> None:
        sidecar = self._get_sidecar(SIMPLE_SRC)
        report = build_report({}, sidecar)
        assert report.total_lines_covered() == 0
        assert report.covered_lines == []

    def test_full_iir_coverage_gives_nonempty_report(self) -> None:
        rt = TetradRuntime()
        module, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        from vm_core import VMCore
        from tetrad_runtime.iir_translator import TETRAD_OPCODE_EXTENSIONS
        vm = VMCore(u8_wrap=True, opcodes=TETRAD_OPCODE_EXTENSIONS)
        vm.enable_coverage()
        vm.execute(module, fn=module.entry_point or "main")
        iir_cov = vm.coverage_data()
        report = build_report(iir_cov, sidecar)
        assert report.total_lines_covered() > 0

    def test_duplicate_ips_for_same_line_merge_correctly(self) -> None:
        """Multiple IIR instructions at the same source line produce one CoveredLine."""
        rt = TetradRuntime()
        module, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        from vm_core import VMCore
        from tetrad_runtime.iir_translator import TETRAD_OPCODE_EXTENSIONS
        vm = VMCore(u8_wrap=True, opcodes=TETRAD_OPCODE_EXTENSIONS)
        vm.enable_coverage()
        vm.execute(module, fn=module.entry_point or "main")
        iir_cov = vm.coverage_data()
        report = build_report(iir_cov, sidecar)
        # No duplicate (file, line) pairs.
        seen = [(cl.file, cl.line) for cl in report.covered_lines]
        assert len(seen) == len(set(seen))


# ---------------------------------------------------------------------------
# Total lines covered
# ---------------------------------------------------------------------------

class TestTotalLinesCovered:

    def test_total_matches_covered_lines_length(self) -> None:
        report = _run(TWO_FN_SRC)
        assert report.total_lines_covered() == len(report.covered_lines)

    def test_single_line_program_covers_one_line(self) -> None:
        report = _run(SINGLE_LINE_SRC)
        # A single-line program should cover at least line 1.
        assert report.total_lines_covered() >= 1
        assert 1 in report.lines_for_file(SOURCE_PATH)

    def test_empty_after_empty_iir_coverage(self) -> None:
        rt = TetradRuntime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        report = build_report({}, sidecar)
        assert report.total_lines_covered() == 0
