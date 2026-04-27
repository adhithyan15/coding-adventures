"""Impact ranking for the JIT insight pass.

The ranking formula converts raw profiler numbers into a single integer
that lets the report sort sites from "fix this first" to "fix this last":

    impact = call_count × cost_weight

where ``cost_weight`` is the ``DispatchCost.weight`` property (0 / 1 / 10 / 100).

Why this formula?
-----------------
We do not have cycle-accurate timings per instruction.  ``call_count ×
cost_weight`` is a conservative *proxy* that preserves the relative ordering
of dispatch strategies:

* A DEOPT site with 10 calls scores 1 000 — worse than a GUARD site with
  100 calls (score 100), because deoptimisations are ~100× more expensive.
* A GENERIC_CALL site with 50 000 calls scores 500 000 — worse than a GUARD
  site with 200 000 calls (score 200 000), even though the GUARD fires more
  often, because each generic call is ~10× more expensive.

The developer sees the worst offenders first.  The ``estimated speedup``
field in ``ProfilingReport.format_text()`` translates the score back into
an approximate percentage for human consumption.

Functions
---------
``rank_sites(sites)``
    Sort a list of ``TypeSite`` objects in descending impact order (in-place
    for efficiency; also returns the sorted list for chaining).

``total_instructions(fn_list)``
    Sum ``observation_count`` across all instructions in all functions to
    produce the ``total_instructions_executed`` field for ``ProfilingReport``.
"""

from __future__ import annotations

from interpreter_ir.function import IIRFunction

from jit_profiling_insights.types import TypeSite


def rank_sites(sites: list[TypeSite]) -> list[TypeSite]:
    """Sort *sites* in-place by descending impact score.

    Sites with equal impact are further sorted by ``DispatchCost.weight``
    (descending) so that DEOPT ties beat GENERIC_CALL ties beat GUARD ties.
    This ensures the most severe dispatch strategy is always shown first
    when the raw impact scores happen to be equal.

    Parameters
    ----------
    sites:
        The list of ``TypeSite`` objects to sort.  Sorted in-place.

    Returns
    -------
    list[TypeSite]
        The same list, now sorted, for call-chaining.

    Example::

        ranked = rank_sites(sites)
        # ranked[0] is the worst offender.
    """
    sites.sort(
        key=lambda s: (s.impact, s.dispatch_cost.weight),
        reverse=True,
    )
    return sites


def total_instructions(fn_list: list[IIRFunction]) -> int:
    """Sum observation counts across all functions to get total exec'd instructions.

    Only instructions that the profiler actually sampled (``observation_count
    > 0``) are counted.  Instructions that were never reached have a count of
    zero and contribute nothing.

    Parameters
    ----------
    fn_list:
        List of ``IIRFunction`` objects from a post-JIT module.

    Returns
    -------
    int
        Total number of instruction executions observed by the profiler.
        Zero if no profiling data was recorded.

    Example::

        total = total_instructions(module.functions)
        # Use as ProfilingReport.total_instructions_executed.
    """
    return sum(
        instr.observation_count
        for fn in fn_list
        for instr in fn.instructions
    )
