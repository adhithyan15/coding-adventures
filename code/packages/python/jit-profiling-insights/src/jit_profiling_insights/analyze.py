"""analyze() — the main entry point for jit-profiling-insights.

This module ties together the three sub-passes:

1. **Scan** — iterate over every instruction in every function.
2. **Classify** — call ``_classify_cost()`` and ``_find_root_register()``
   for each instruction to determine dispatch overhead and root cause.
3. **Rank** — sort the resulting ``TypeSite`` list by impact and wrap it in
   a ``ProfilingReport``.

Usage::

    from jit_profiling_insights import analyze

    # fn_list is a list[IIRFunction] from a post-JIT IIRModule.
    report = analyze(fn_list, program_name="fibonacci")
    print(report.format_text())

The function is intentionally stateless — it reads the IIR annotations
written by vm-core and jit-core but never modifies them.  The same
``fn_list`` can be passed to ``analyze()`` multiple times (e.g., before and
after adding annotations to see the improvement).

Filtering
---------
Instructions that the profiler never observed (``observation_count == 0``)
are skipped because we have no runtime data to reason about.  Only hot
instructions contribute to the report.

The ``min_call_count`` parameter (default 1) lets the caller raise the bar
so that rarely-executed instructions don't appear in the report.  For
interactive use (REPL, LSP) the default is fine.  For CI / performance
budget tests, a higher threshold (e.g. 1 000) avoids noise.
"""

from __future__ import annotations

from interpreter_ir.function import IIRFunction

from jit_profiling_insights.classify import (
    _classify_cost,
    _find_root_register,
    _savings_description,
)
from jit_profiling_insights.rank import rank_sites, total_instructions
from jit_profiling_insights.types import DispatchCost, ProfilingReport, TypeSite


def analyze(
    fn_list: list[IIRFunction],
    *,
    program_name: str = "program",
    min_call_count: int = 1,
) -> ProfilingReport:
    """Analyse a list of profiled IIR functions and produce a ProfilingReport.

    The algorithm runs in four steps:

    Step 1 — Compute total instruction count
        Sum ``observation_count`` across all instructions.  Used for
        percentage estimates in the text report.

    Step 2 — Scan every instruction
        For each instruction in each function that the profiler observed
        (``observation_count >= min_call_count``), classify its dispatch cost
        and, for non-NONE costs, find the root untyped register.

    Step 3 — Build TypeSite records
        For each non-NONE site, construct a ``TypeSite`` with the classified
        cost, root register, and a human-readable savings description.

    Step 4 — Rank and wrap
        Sort sites by descending impact and return a ``ProfilingReport``.

    Parameters
    ----------
    fn_list:
        ``IIRFunction`` objects whose instructions carry profiler annotations
        (``observed_type`` and ``observation_count``).  Typically the
        ``module.functions`` from a post-JIT ``IIRModule``.
    program_name:
        A friendly label for the report — the module name or source filename.
    min_call_count:
        Instructions with fewer than this many observations are skipped.
        Default 1 means "include all observed instructions".

    Returns
    -------
    ProfilingReport
        A ranked, structured report ready for ``format_text()`` or
        ``format_json()`` or programmatic inspection via ``top_n()``.

    Example::

        from interpreter_ir import IIRModule
        from jit_profiling_insights import analyze

        module = ...  # post-JIT IIRModule
        report = analyze(module.functions, program_name=module.name)
        for site in report.top_n(5):
            print(f"{site.function}::{site.instruction_op} — {site.dispatch_cost}")
    """
    # Step 1 — total instruction count for percentage calculations.
    total = total_instructions(fn_list)

    sites: list[TypeSite] = []

    # Step 2 & 3 — scan, classify, build TypeSite records.
    for fn in fn_list:
        for idx, instr in enumerate(fn.instructions):
            # Skip instructions the profiler never reached.
            if instr.observation_count < min_call_count:
                continue

            cost = _classify_cost(instr)

            # NONE means no overhead — skip it; keep the report focused.
            if cost == DispatchCost.NONE:
                continue

            # Trace the data-flow chain to find the root untyped register.
            root_reg = _find_root_register(instr, fn.instructions, idx)

            observed = instr.observed_type or "unknown"
            deopt_count: int = getattr(instr, "deopt_count", 0)

            savings = _savings_description(cost, instr.observation_count, instr.op)

            site = TypeSite(
                function=fn.name,
                instruction_op=instr.op,
                source_register=root_reg,
                observed_type=observed,
                type_hint=instr.type_hint,
                dispatch_cost=cost,
                call_count=instr.observation_count,
                deopt_count=deopt_count,
                savings_description=savings,
            )
            sites.append(site)

    # Step 4 — rank and wrap.
    rank_sites(sites)

    return ProfilingReport(
        program_name=program_name,
        total_instructions_executed=total,
        sites=sites,
    )
