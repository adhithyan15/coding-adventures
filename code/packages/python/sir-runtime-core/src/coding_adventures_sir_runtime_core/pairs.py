"""Cons pairs — re-exported from ``coding-adventures-sir-runtime-pairs``.

The SIR ``Pair`` value type (``cons`` / ``car`` / ``cdr``) used to live here,
but it is a self-contained per-concern quirk, so it has moved to its own
publishable package.  This module is now a thin **re-export shim** kept for
back-compatibility: every existing intra-core import (``from .pairs import
Pair``) and external consumer keeps working unchanged, and a value built by
``core.cons`` is the *same* class as one built by the dedicated package (no
two-``Pair``-classes hazard).

The pairs package deliberately depends on **nothing** — its Lisp-list display
calls an injectable hook.  Core wires its richer :func:`to_display` into that
hook in :mod:`coding_adventures_sir_runtime_core` (the package ``__init__``) via
:func:`set_display`, so a pair still renders as ``(1 2 3)`` once core is
imported.  See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from coding_adventures_sir_runtime_pairs import (
    Pair,
    car,
    cdr,
    cons,
    is_pair,
    set_display,
)

__all__ = ["Pair", "car", "cdr", "cons", "is_pair", "set_display"]
