"""coding-adventures-sir-runtime-core — core runtime for SIR-emitted Python.

Semantic-IR backends translate most constructs to **native** Python (a
sequence is a ``list``, a loop is a ``for``, a class is a ``class``).  The
handful of SIR semantics with **no faithful native equivalent** live here
and are imported by the emitted module:

    import coding_adventures_sir_runtime_core as _sir
    _sir.truthy(x)        # SIR truthiness: only False / nil are falsy
    _sir.cons(a, b)       # cons pairs
    _sir.sir_write("stdout", "once", False, v)  # SIR28 console output

This library implements **SIR** semantics (not any one source language's),
so a Ruby frontend today and a JavaScript or Python frontend tomorrow all
reuse it.  See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from .arithmetic import (
    add,
    div,
    ge,
    gt,
    le,
    lt,
    mul,
    shift_left,
    sub,
    true_div,
    trunc_div,
    utrunc_div,
)
from .pairs import Pair, car, cdr, cons, is_pair
from .pairs import set_display as _set_pairs_display
from .runtime import (
    Closure,
    LocalJumpError,
    apply,
    as_lambda,
    builtin_closure,
    call_builtin,
    global_get,
    global_get_static,
    global_set,
    make_closure,
    sir_write,
)
from .symbols import Symbol, intern
from .values import (
    eq,
    is_null,
    is_number,
    is_symbol,
    ne,
    set_display_convention,
    to_display,
    truthy,
)

# Inject core's richer ``to_display`` into the (dependency-free) pairs package
# so a ``Pair`` renders as a Lisp list (``(1 2 3)``, ``#t``/``nil``/symbols)
# rather than via plain ``str``.  Done once at import time; see ``pairs`` shim.
_set_pairs_display(to_display)

__all__ = [
    # values
    "truthy",
    "eq",
    "ne",
    "to_display",
    "set_display_convention",
    "is_null",
    "is_number",
    "is_symbol",
    # symbols
    "Symbol",
    "intern",
    # pairs
    "Pair",
    "cons",
    "car",
    "cdr",
    "is_pair",
    # arithmetic
    "add",
    "shift_left",
    "sub",
    "mul",
    "div",
    "trunc_div",
    "utrunc_div",
    "true_div",
    "lt",
    "gt",
    "le",
    "ge",
    # closures / globals / dispatch
    "Closure",
    "LocalJumpError",
    "apply",
    "make_closure",
    "as_lambda",
    "global_set",
    "global_get",
    "global_get_static",
    "sir_write",
    "call_builtin",
    "builtin_closure",
]
