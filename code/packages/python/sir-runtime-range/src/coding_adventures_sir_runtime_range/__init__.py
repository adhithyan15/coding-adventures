"""coding-adventures-sir-runtime-range — the SIR ``Range`` value type.

A Ruby *range* (``1..5``, ``1...5``, ``1..``, ``..5``) is a first-class value,
not a loop.  Python's ``range`` is half-open and integer-only and cannot model
the inclusive or begin/endless forms, so the SIR ``Range`` ships here as a
per-concern runtime::

    from coding_adventures_sir_runtime_range import range as sir_range
    r = sir_range(1, 5, False)   # the inclusive range 1..5
    list(r)                      # [1, 2, 3, 4, 5]
    3 in r                       # True
    repr(r)                      # "1..5"

The backend emits ``_sir_range(start, stop, exclusive)`` for a Ruby range
literal, gated so pure modules never gain the dependency.  The package depends
on **nothing** (numeric ranges need no richer display).

See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from .range import Range, Val, includes, is_range, range_, to_list

# Public alias: callers (and the emitted-code import header) bind the
# constructor as ``range``; internally it is ``range_`` to avoid shadowing the
# builtin inside the module.
range = range_  # noqa: A001  (intentional re-export under the SIR name)

__all__ = [
    "Range",
    "Val",
    "includes",
    "is_range",
    "range",
    "range_",
    "to_list",
]
