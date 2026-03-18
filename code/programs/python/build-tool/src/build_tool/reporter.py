"""
reporter.py -- Build Report Formatting
=======================================

This module formats and prints a summary table of build results. The output
is human-readable and designed for terminal display.

Output format
-------------

::

    Build Report
    ============
    Package                    Status     Duration
    python/logic-gates         SKIPPED    -
    python/arithmetic          BUILT      2.3s
    python/arm-simulator       FAILED     0.5s
    python/riscv-simulator     DEP-SKIP   - (dep failed)

    Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped
"""

from __future__ import annotations

import sys
from io import StringIO

from build_tool.executor import BuildResult


# Status display names and their sort priority (for consistent ordering)
STATUS_DISPLAY: dict[str, str] = {
    "built": "BUILT",
    "failed": "FAILED",
    "skipped": "SKIPPED",
    "dep-skipped": "DEP-SKIP",
    "would-build": "WOULD-BUILD",
}


def _format_duration(seconds: float) -> str:
    """Format a duration for display.

    Returns "-" for zero/negligible durations, otherwise "X.Ys".
    """
    if seconds < 0.01:
        return "-"
    return f"{seconds:.1f}s"


def format_report(results: dict[str, BuildResult]) -> str:
    """Format a build report as a string.

    Args:
        results: Mapping from package name to BuildResult.

    Returns:
        The formatted report string.
    """
    buf = StringIO()

    buf.write("\nBuild Report\n")
    buf.write("============\n")

    if not results:
        buf.write("No packages processed.\n")
        return buf.getvalue()

    # Calculate column widths
    max_name_len = max(len(name) for name in results)
    max_name_len = max(max_name_len, len("Package"))

    # Header
    buf.write(
        f"{'Package':<{max_name_len}}   {'Status':<12} {'Duration'}\n"
    )

    # Sort results by name for consistent output
    for name in sorted(results):
        result = results[name]
        status = STATUS_DISPLAY.get(result.status, result.status.upper())
        duration = _format_duration(result.duration)

        if result.status == "dep-skipped":
            duration = "- (dep failed)"

        buf.write(f"{name:<{max_name_len}}   {status:<12} {duration}\n")

    # Summary line
    total = len(results)
    built = sum(1 for r in results.values() if r.status == "built")
    skipped = sum(1 for r in results.values() if r.status == "skipped")
    failed = sum(1 for r in results.values() if r.status == "failed")
    dep_skipped = sum(1 for r in results.values() if r.status == "dep-skipped")
    would_build = sum(1 for r in results.values() if r.status == "would-build")

    buf.write(f"\nTotal: {total} packages")
    if built:
        buf.write(f" | {built} built")
    if skipped:
        buf.write(f" | {skipped} skipped")
    if failed:
        buf.write(f" | {failed} failed")
    if dep_skipped:
        buf.write(f" | {dep_skipped} dep-skipped")
    if would_build:
        buf.write(f" | {would_build} would-build")
    buf.write("\n")

    return buf.getvalue()


def print_report(
    results: dict[str, BuildResult],
    file: object = None,
) -> None:
    """Print a summary table of build results.

    Args:
        results: Mapping from package name to BuildResult.
        file: Output stream (defaults to sys.stdout).
    """
    output = file if file is not None else sys.stdout
    report = format_report(results)
    print(report, file=output, end="")
