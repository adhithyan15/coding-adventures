"""coding-adventures-sir-runtime-exceptions — exception runtime for SIR Python.

SIR backends translate most constructs to **native** Python.  Exception
handling translates *mostly* natively — ``begin/rescue/ensure`` becomes a native
``try/except/finally`` — but two pieces have no faithful native equivalent and
ship here, imported by the emitted module::

    from coding_adventures_sir_runtime_exceptions import raise_error as _sir_exc_raise_error
    _sir_exc_raise_error("ArgumentError", "bad")   # raise ArgumentError, "bad"

1. **A SIR exception object** — :class:`SirError`, a real ``Exception`` tagged
   with the Ruby/SIR class name in ``sir_class``.
2. **Rescue-clause type matching** — :func:`rescue_matches`, so the emitted
   ``except`` body can dispatch an ordered list of typed ``rescue`` clauses.

It implements **SIR** semantics (not any one source language's), so a future
JavaScript -> SIR -> Python path reuses it.  See ``code/specs/sir-runtime.md``.

**User ancestry (E2):** the backend threads ``class Child < Parent`` edges here
via :func:`register_ancestry` at program init, so ``rescue StandardError`` also
catches a raised user ``MyErr < StandardError``.  User edges are additive over
the built-in table; a user class with no registered superclass still matches by
exact name (or via ``Exception`` / a bare ``rescue``).  A bare re-raise still
becomes a generic ``RuntimeError`` (awaits frontend threading of the in-flight
exception).
"""

from __future__ import annotations

from .exceptions import (
    SirError,
    Val,
    ancestry_chain,
    class_of_thrown,
    raise_error,
    register_ancestry,
    rescue_matches,
)

__all__ = [
    "SirError",
    "Val",
    "ancestry_chain",
    "class_of_thrown",
    "raise_error",
    "register_ancestry",
    "rescue_matches",
]
