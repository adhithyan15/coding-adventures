"""Core data types for jit-profiling-insights.

Three layers of abstraction:

1. ``DispatchCost`` — a four-level enum ranking the cost of each dynamic
   dispatch strategy the JIT may choose.  The ordering (NONE < GUARD <
   GENERIC_CALL < DEOPT) is intentional and drives the impact formula.

2. ``TypeSite`` — one instruction in one function that the insight pass
   identified as a candidate for improvement.  It bundles the raw profiler
   data (call_count, observed_type) with a human-readable diagnosis
   (savings_description).

3. ``ProfilingReport`` — the top-level result of calling ``analyze()``.
   Contains the program name, total instruction count, and a ranked list of
   ``TypeSite`` entries.  Consumers can call ``top_n()``, ``format_text()``,
   or ``format_json()`` to get the data in their preferred shape.

Example usage::

    from jit_profiling_insights.types import DispatchCost, ProfilingReport, TypeSite

    site = TypeSite(
        function="fibonacci",
        instruction_op="add",
        source_register="%r0",
        observed_type="int",
        type_hint="any",
        dispatch_cost=DispatchCost.GUARD,
        call_count=1_048_576,
        deopt_count=0,
        savings_description="would eliminate 1 type_assert per call",
    )

    report = ProfilingReport(
        program_name="fibonacci",
        total_instructions_executed=8_388_608,
        sites=[site],
    )
    print(report.format_text())
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from enum import Enum


class DispatchCost(str, Enum):
    """How expensive is the dynamic dispatch path the JIT chose?

    The numeric weight of each variant is used in the impact formula
    (``call_count × cost_weight``).  Choosing ``str`` as the mixin lets the
    enum serialise directly to JSON without a custom encoder.

    Ordered from cheapest to most expensive:

    NONE
        The instruction's destination is statically typed or the type was
        successfully inferred.  The JIT emits a direct typed operation —
        zero runtime overhead.

    GUARD
        The JIT inferred a concrete type but the source variable is declared
        ``"any"``.  It inserts one ``type_assert`` instruction on every call
        to verify the assumption at runtime.  Cost: one branch per call.

    GENERIC_CALL
        Neither the static annotation nor the profiler found a concrete type.
        The JIT falls back to a full runtime dispatch through the generic
        call-table.  Cost: roughly 10× slower than a typed operation.

    DEOPT
        A guard was emitted but the runtime type violated it.  The function
        fell back to the interpreter (deoptimisation).  Cost: roughly 100×
        slower than a typed operation because the entire native frame must be
        unwound and rebuilt as an interpreter frame.
    """

    NONE = "none"
    GUARD = "guard"
    GENERIC_CALL = "generic"
    DEOPT = "deopt"

    # ------------------------------------------------------------------
    # Numeric weight — used by rank.py's impact formula
    # ------------------------------------------------------------------

    @property
    def weight(self) -> int:
        """Return the cost multiplier for the impact formula.

        These weights reflect the approximate relative cost of each dispatch
        strategy on a modern out-of-order CPU:

        * NONE → 0   (no overhead; statically resolved)
        * GUARD → 1  (one conditional branch per call)
        * GENERIC_CALL → 10  (virtual dispatch + tag check ≈ 10 branches)
        * DEOPT → 100  (interpreter fallback ≈ 100× slower)
        """
        _weights = {
            DispatchCost.NONE: 0,
            DispatchCost.GUARD: 1,
            DispatchCost.GENERIC_CALL: 10,
            DispatchCost.DEOPT: 100,
        }
        return _weights[self]


@dataclass
class TypeSite:
    """One instruction-level hotspot identified by the insight pass.

    A ``TypeSite`` answers three questions:

    * *What happened?*  — ``instruction_op``, ``call_count``, ``observed_type``
    * *Why is it expensive?*  — ``dispatch_cost``, ``deopt_count``
    * *What should you do?*  — ``savings_description`` (human-readable advice)

    Parameters
    ----------
    function:
        Name of the ``IIRFunction`` containing this instruction.
    instruction_op:
        The mnemonic of the hot instruction (e.g. ``"add"``, ``"cmp_lt"``).
    source_register:
        The SSA register whose ``type_hint == "any"`` is causing the overhead.
        The insight pass traces the data-flow chain back to find the root
        register, which is often a function parameter.
    observed_type:
        The actual runtime type the profiler saw on this register, e.g.
        ``"int"``.  ``"polymorphic"`` if multiple types were seen.
    type_hint:
        The declared type from the source program.  Almost always ``"any"``
        for instructions the insight pass flags.
    dispatch_cost:
        The classified dispatch strategy chosen by the JIT.
    call_count:
        How many times this instruction executed (from ``observation_count``).
    deopt_count:
        How many times a guard on this register failed and triggered an
        interpreter fallback.  Zero unless ``dispatch_cost == DEOPT``.
    savings_description:
        A one-sentence human-readable explanation of what adding a type
        annotation would eliminate.  Written by ``classify.py``.
    """

    function: str
    instruction_op: str
    source_register: str
    observed_type: str
    type_hint: str
    dispatch_cost: DispatchCost
    call_count: int
    deopt_count: int
    savings_description: str

    @property
    def impact(self) -> int:
        """Impact score = call_count × cost_weight.

        Higher is worse.  Used to sort sites from most to least urgent.
        A GUARD with 1 M calls scores 1 M; a GENERIC_CALL with 100 calls
        scores 1 000; a DEOPT with 10 calls scores 1 000 — but they
        mean very different things, so the caller may still want to filter
        by cost tier separately.
        """
        return self.call_count * self.dispatch_cost.weight

    def to_dict(self) -> dict:
        """Return a JSON-serialisable dict representation."""
        d = asdict(self)
        d["dispatch_cost"] = self.dispatch_cost.value
        d["impact"] = self.impact
        return d


@dataclass
class ProfilingReport:
    """Top-level output of the insight pass.

    Produced by ``analyze()`` and consumed by the CLI, LSP, REPL, or CI
    tooling.  All fields are populated at construction time; ``sites`` is
    expected to be pre-sorted by impact (highest first).

    Parameters
    ----------
    program_name:
        Friendly label for the output — typically the IIRModule name or the
        source file name.
    total_instructions_executed:
        Sum of ``observation_count`` across all instructions in all functions.
        Used to compute the percentage of overhead that each site represents.
    sites:
        Ranked list of ``TypeSite`` entries.  The list is sorted by impact
        (highest first) by ``analyze()`` before being stored here.
    """

    program_name: str
    total_instructions_executed: int
    sites: list[TypeSite] = field(default_factory=list)

    # ------------------------------------------------------------------
    # Programmatic access
    # ------------------------------------------------------------------

    def top_n(self, n: int = 10) -> list[TypeSite]:
        """Return the top *n* sites by impact score.

        The list is already sorted; this is just a slice.  Useful for
        displaying a summary in a REPL or CI log without the full list.
        """
        return self.sites[:n]

    def functions_with_issues(self) -> list[str]:
        """Return a deduplicated, order-preserving list of function names
        that have at least one non-NONE dispatch site.

        Functions appear in the order of their highest-impact site.
        """
        seen: set[str] = set()
        result: list[str] = []
        for site in self.sites:
            if site.dispatch_cost != DispatchCost.NONE and site.function not in seen:
                seen.add(site.function)
                result.append(site.function)
        return result

    def has_deopts(self) -> bool:
        """Return True if any site is classified as DEOPT."""
        return any(s.dispatch_cost == DispatchCost.DEOPT for s in self.sites)

    # ------------------------------------------------------------------
    # Formatted output
    # ------------------------------------------------------------------

    def format_text(self) -> str:
        """Render the report as a human-readable text string.

        The format is designed to be readable in a terminal, a REPL session,
        or a CI log.  Each site gets a coloured header line (using emoji
        instead of ANSI codes, so it renders in Markdown-capable viewers too)
        followed by the diagnosis and suggestion.

        Example output::

            JIT Profiling Report — fibonacci (8,388,608 total instructions)
            ═══════════════════════════════════════════════════════════════

            🔴 HIGH IMPACT  fibonacci::add
              Source: %r0 (type_hint="any")
              Observed: int (100% of 1,048,576 calls)
              Cost: GUARD — 1 type_assert per call = 1,048,576 branches
              Fix: would eliminate 1 type_assert per call
              Estimated speedup: ~8%

            ✅ No deoptimisations occurred.

            Summary: 1 annotation site would eliminate ~8% of total overhead.
        """
        lines: list[str] = []
        title = (
            f"JIT Profiling Report — {self.program_name} "
            f"({self.total_instructions_executed:,} total instructions)"
        )
        lines.append(title)
        lines.append("═" * len(title))

        active_sites = [s for s in self.sites if s.dispatch_cost != DispatchCost.NONE]

        if not active_sites:
            lines.append("")
            lines.append("✅ No dispatch overhead detected — all hot paths are typed.")
            return "\n".join(lines)

        for site in active_sites:
            lines.append("")
            tier_label, icon = _tier_label(site)
            lines.append(f"{icon} {tier_label}  {site.function}::{site.instruction_op}")
            lines.append(f"  Source: {site.source_register} (type_hint={site.type_hint!r})")

            pct = ""
            if self.total_instructions_executed > 0:
                p = 100 * site.call_count / self.total_instructions_executed
                pct = f" ({p:.0f}% of total)"
            lines.append(
                f"  Observed: {site.observed_type} on {site.call_count:,} calls{pct}"
            )

            cost_name = site.dispatch_cost.value.upper()
            lines.append(f"  Cost: {cost_name} — {site.savings_description}")

            if site.deopt_count > 0:
                lines.append(
                    f"  Deoptimisations: {site.deopt_count:,} guard failures"
                )

            speedup = _estimate_speedup(site, self.total_instructions_executed)
            if speedup:
                lines.append(f"  Estimated speedup: ~{speedup}%")

        lines.append("")
        if self.has_deopts():
            deopt_count = sum(1 for s in self.sites if s.dispatch_cost == DispatchCost.DEOPT)
            lines.append(f"⚠️  {deopt_count} deoptimisation(s) detected — highest priority fixes.")
        else:
            lines.append("✅ No deoptimisations occurred.")

        n = len(active_sites)
        noun = "site" if n == 1 else "sites"
        total_savings = sum(
            _estimate_speedup(s, self.total_instructions_executed) or 0
            for s in active_sites
        )
        lines.append(
            f"\nSummary: {n} annotation {noun} would eliminate "
            f"~{total_savings}% of total overhead."
        )

        return "\n".join(lines)

    def format_json(self) -> str:
        """Render the report as a JSON string.

        The JSON is pretty-printed with 2-space indentation.  Each site is
        represented by its ``to_dict()`` output plus a top-level
        ``total_instructions_executed`` and ``program_name``.

        Example::

            {
              "program_name": "fibonacci",
              "total_instructions_executed": 8388608,
              "sites": [
                {
                  "function": "fibonacci",
                  "instruction_op": "add",
                  ...
                }
              ]
            }
        """
        payload = {
            "program_name": self.program_name,
            "total_instructions_executed": self.total_instructions_executed,
            "sites": [s.to_dict() for s in self.sites],
        }
        return json.dumps(payload, indent=2)


# ---------------------------------------------------------------------------
# Private helpers used by format_text()
# ---------------------------------------------------------------------------

def _tier_label(site: TypeSite) -> tuple[str, str]:
    """Return (label_string, emoji) for a site's impact tier."""
    if site.dispatch_cost == DispatchCost.DEOPT:
        return "CRITICAL", "🚨"
    impact = site.impact
    if impact >= 100_000:
        return "HIGH IMPACT", "🔴"
    if impact >= 1_000:
        return "MEDIUM IMPACT", "🟡"
    return "LOW IMPACT", "🟢"


def _estimate_speedup(site: TypeSite, total: int) -> int | None:
    """Estimate the speedup percentage from annotating this site.

    The estimate is conservative: it assumes that eliminating the dispatch
    overhead for ``site.call_count`` calls saves exactly
    ``site.dispatch_cost.weight`` "work units" per call, and divides that
    by the total instruction count.

    Returns ``None`` if the savings would round to 0% or if ``total == 0``.
    """
    if total == 0 or site.dispatch_cost == DispatchCost.NONE:
        return None
    savings = site.call_count * site.dispatch_cost.weight
    pct = round(100 * savings / total)
    return pct if pct > 0 else None
