"""vm-type-suggestions — parameter type suggestions from the VM profiler.

After running a program under vm-core, this package reads the profiler
observations on each function's parameter-loading instructions and asks:

    "Based on what the VM actually observed at runtime, what type
     annotations should the developer add?"

The answer is simple and direct:

    add — called 1,000,000 times
      'a' (arg 0): always u8  → declare 'a: u8'
      'b' (arg 1): always u8  → declare 'b: u8'

No JIT required.  No guard analysis.  Just: "here's what came in —
you should say so in the source code."

Public API
----------
``suggest(fn_list, program_name)``
    The main entry point.  Returns a ``SuggestionReport``.

``SuggestionReport``
    Top-level result: ``actionable()`` (CERTAIN suggestions only),
    ``by_function()`` (grouped view), ``format_text()``, ``format_json()``.

``ParamSuggestion``
    One parameter: name, observed type, confidence, suggestion string.

``Confidence``
    ``CERTAIN`` / ``MIXED`` / ``NO_DATA``.

Quick start::

    from vm_type_suggestions import suggest

    report = suggest(module.functions, program_name="fibonacci")
    print(report.format_text())

    for s in report.actionable():
        print(f"  {s.function}.{s.param_name}: {s.suggestion}")
"""

from __future__ import annotations

from vm_type_suggestions.suggest import suggest
from vm_type_suggestions.types import Confidence, ParamSuggestion, SuggestionReport

__all__ = [
    "suggest",
    "Confidence",
    "ParamSuggestion",
    "SuggestionReport",
]
