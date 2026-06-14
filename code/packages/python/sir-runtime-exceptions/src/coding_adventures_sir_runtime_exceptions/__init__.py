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

**v0 limitation:** user-defined exception-class ancestry is unknown (no SIR
exception-class symbol table), so ``rescue StandardError`` matches only the
built-in subclasses and user classes match by exact name; a bare re-raise
becomes a generic ``RuntimeError``.  Both await frontend threading of the
exception model.
"""

from __future__ import annotations

from .exceptions import (
    SirError,
    Val,
    class_of_thrown,
    raise_error,
    rescue_matches,
)

__all__ = [
    "SirError",
    "Val",
    "class_of_thrown",
    "raise_error",
    "rescue_matches",
]
