"""Core data types for vm-type-suggestions.

Two main types:

``Confidence``
    Three-level enum describing how certain the suggestion is.
    Only ``CERTAIN`` suggestions are shown as actionable advice.
    ``MIXED`` and ``NO_DATA`` are reported so the developer understands
    why no suggestion was made.

``ParamSuggestion``
    One parameter in one function.  Bundles the profiler observation
    (call_count, observed_type) with the confidence level and a
    ready-to-use suggestion string like ``"declare 'n: u8'"``.

``SuggestionReport``
    The top-level result of ``suggest()``.  Provides ``actionable()``
    (only CERTAIN suggestions), ``by_function()`` (grouped view),
    ``format_text()`` (terminal), and ``format_json()`` (tooling).

Example::

    from vm_type_suggestions.types import Confidence, ParamSuggestion, SuggestionReport

    s = ParamSuggestion(
        function="add",
        param_name="a",
        param_index=0,
        observed_type="u8",
        call_count=1_000_000,
        confidence=Confidence.CERTAIN,
        suggestion="declare 'a: u8'",
    )
    report = SuggestionReport(program_name="add", total_calls=1_000_000, suggestions=[s])
    print(report.format_text())
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from enum import Enum


class Confidence(str, Enum):
    """How certain is the type suggestion?

    Uses ``str`` as a mixin so the value is directly JSON-serialisable.

    CERTAIN
        The profiler observed exactly one concrete type on every call.
        Safe to annotate.

    MIXED
        The profiler saw multiple different types (``"polymorphic"`` in
        IIR terminology).  A single annotation would be wrong — either
        the function genuinely handles multiple types (keep it untyped
        or add an overload) or there is a bug.

    NO_DATA
        The profiler never reached this parameter.  This happens when
        the function was never called during the profiling run, or when
        the parameter-loading instruction was skipped (e.g. unreachable
        code).  No advice can be given.
    """

    CERTAIN = "certain"
    MIXED = "mixed"
    NO_DATA = "no_data"


@dataclass
class ParamSuggestion:
    """A type suggestion for one function parameter.

    Parameters
    ----------
    function:
        Name of the ``IIRFunction`` containing this parameter.
    param_name:
        The parameter name as declared in ``IIRFunction.params``.
    param_index:
        0-based position of the parameter in the function signature.
    observed_type:
        The IIR type string the profiler observed (e.g. ``"u8"``),
        ``"polymorphic"`` for mixed types, or ``None`` for no data.
    call_count:
        How many times the parameter-loading instruction was profiled.
        Equals the number of times the function was called (for untyped
        functions, the profiler records every call).
    confidence:
        ``CERTAIN`` / ``MIXED`` / ``NO_DATA``.
    suggestion:
        Human-readable advice string (e.g. ``"declare 'n: u8'"``),
        or ``None`` when no safe suggestion can be made.
    """

    function: str
    param_name: str
    param_index: int
    observed_type: str | None
    call_count: int
    confidence: Confidence
    suggestion: str | None

    def to_dict(self) -> dict:
        """Return a JSON-serialisable dict."""
        d = asdict(self)
        d["confidence"] = self.confidence.value
        return d


@dataclass
class SuggestionReport:
    """Top-level output of ``suggest()``.

    Parameters
    ----------
    program_name:
        Friendly label for the output.
    total_calls:
        Sum of ``call_count`` across all ``CERTAIN`` suggestions —
        a rough measure of how much execution the suggestions cover.
    suggestions:
        All ``ParamSuggestion`` entries, including MIXED and NO_DATA,
        so consumers can understand the full picture.
    """

    program_name: str
    total_calls: int
    suggestions: list[ParamSuggestion] = field(default_factory=list)

    # ------------------------------------------------------------------
    # Programmatic access
    # ------------------------------------------------------------------

    def actionable(self) -> list[ParamSuggestion]:
        """Return only CERTAIN suggestions — the ones to act on.

        These are the parameters where one concrete type was observed on
        every call.  Adding the annotation is safe and eliminates all
        type guards on this parameter.
        """
        return [s for s in self.suggestions if s.confidence == Confidence.CERTAIN]

    def by_function(self) -> dict[str, list[ParamSuggestion]]:
        """Group suggestions by function name, preserving insertion order.

        Returns a dict mapping function name → list of ``ParamSuggestion``.
        Functions appear in the order their first suggestion was seen.
        """
        result: dict[str, list[ParamSuggestion]] = {}
        for s in self.suggestions:
            result.setdefault(s.function, []).append(s)
        return result

    # ------------------------------------------------------------------
    # Formatted output
    # ------------------------------------------------------------------

    def format_text(self) -> str:
        """Render the report as a human-readable string.

        Groups suggestions by function and uses simple ASCII / emoji
        markers so the output renders in terminals, CI logs, and
        Markdown viewers.

        Example::

            VM Type Suggestions — fibonacci (1,048,576 total calls)
            ════════════════════════════════════════════════════════

            ✅ fibonacci — 1,048,576 calls
              'n' (arg 0): always u8
              → declare 'n: u8'

            Summary: 1 of 1 untyped parameters can be annotated.
        """
        lines: list[str] = []
        title = (
            f"VM Type Suggestions — {self.program_name} "
            f"({self.total_calls:,} total calls)"
        )
        lines.append(title)
        lines.append("═" * len(title))

        grouped = self.by_function()

        if not grouped:
            lines.append("")
            lines.append("✅ No untyped parameters found — everything is already typed.")
            return "\n".join(lines)

        for fn_name, params in grouped.items():
            lines.append("")
            # Use the call_count from the first param with data, or 0.
            call_count = next(
                (p.call_count for p in params if p.call_count > 0), 0
            )
            lines.append(f"Function: {fn_name} — {call_count:,} calls")

            for p in params:
                if p.confidence == Confidence.CERTAIN:
                    lines.append(
                        f"  ✅ '{p.param_name}' (arg {p.param_index}): "
                        f"always {p.observed_type}"
                    )
                    lines.append(f"     → {p.suggestion}")
                elif p.confidence == Confidence.MIXED:
                    lines.append(
                        f"  ⚠️  '{p.param_name}' (arg {p.param_index}): "
                        f"mixed types observed (polymorphic)"
                    )
                    lines.append(
                        f"     → cannot suggest; consider typed overloads instead"
                    )
                else:  # NO_DATA
                    lines.append(
                        f"  ℹ️  '{p.param_name}' (arg {p.param_index}): "
                        f"no profiling data"
                    )

        lines.append("")
        n_actionable = len(self.actionable())
        n_total = len(self.suggestions)
        noun = "parameter" if n_total == 1 else "parameters"
        lines.append(
            f"Summary: {n_actionable} of {n_total} untyped {noun} can be annotated."
        )

        return "\n".join(lines)

    def format_json(self) -> str:
        """Render the report as a pretty-printed JSON string.

        Example::

            {
              "program_name": "fibonacci",
              "total_calls": 1048576,
              "suggestions": [
                {
                  "function": "fibonacci",
                  "param_name": "n",
                  "param_index": 0,
                  "observed_type": "u8",
                  "call_count": 1048576,
                  "confidence": "certain",
                  "suggestion": "declare 'n: u8'"
                }
              ]
            }
        """
        payload = {
            "program_name": self.program_name,
            "total_calls": self.total_calls,
            "suggestions": [s.to_dict() for s in self.suggestions],
        }
        return json.dumps(payload, indent=2)
