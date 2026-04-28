"""jit-profiling-insights — developer feedback from the JIT compiler.

This package surfaces the JIT compiler's internal decisions to the developer
as actionable advice.  Instead of "this function took 40ms" (cProfile), it
says "this expression causes 1 type guard per call because `n` is declared
untyped — annotating it as `Int` eliminates the branch and saves ~8% runtime".

Public API
----------
``analyze(fn_list, program_name)``
    The main entry point.  Accepts a list of ``IIRFunction`` objects whose
    instructions carry profiler annotations (``observed_type``,
    ``observation_count``) and returns a structured ``ProfilingReport``.

``ProfilingReport``
    The top-level result: ``total_instructions_executed``, ranked
    ``TypeSite`` list, and ``format_text()`` / ``format_json()`` renderers.

``TypeSite``
    One instruction-level hotspot: which register is untyped, what the
    profiler observed, how expensive the dispatch is, and what to do about it.

``DispatchCost``
    Four-level enum: ``NONE`` / ``GUARD`` / ``GENERIC_CALL`` / ``DEOPT``.
    Drives the impact formula (``call_count × cost_weight``).

Quick start::

    from jit_profiling_insights import analyze, ProfilingReport, TypeSite

    # After running a program under jit-core:
    report: ProfilingReport = analyze(fn_list, program_name="fibonacci")

    # Human-readable terminal output:
    print(report.format_text())

    # JSON for tooling (LSP, CI, REPL):
    import json
    data = json.loads(report.format_json())

    # Programmatic access:
    for site in report.top_n(5):
        print(f"{site.function}.{site.instruction_op}: "
              f"{site.call_count:,} calls, cost={site.dispatch_cost}")
"""

from __future__ import annotations

from jit_profiling_insights.analyze import analyze
from jit_profiling_insights.types import DispatchCost, ProfilingReport, TypeSite

__all__ = [
    "analyze",
    "DispatchCost",
    "ProfilingReport",
    "TypeSite",
]
