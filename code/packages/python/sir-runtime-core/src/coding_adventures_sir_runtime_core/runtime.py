"""Closures, the global store, printing, and the builtin dispatch table.

These tie the value-level helpers together into the small runtime surface
that SIR-emitted Python calls into.

- **Closures** — SIR closures carry their captured values explicitly (the
  frontend computed them).  ``make_closure`` binds the captures ahead of
  the call-time arguments; ``apply`` invokes a closure handle.  A uniform
  ``Closure`` object lets an ``IndirectCall`` invoke *any* callable target
  the same way.
- **Globals** — a process-global name→value store backing SIR ``Globals``.
- **Dispatch** — when a builtin is used as a *first-class value* (passed
  around, not called directly), ``builtin_closure`` wraps it; ``call_builtin``
  looks it up by SIR name.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from . import arithmetic, pairs, values
from .symbols import Symbol

# --- Closures --------------------------------------------------------------


class Closure:
    """A callable handle wrapping a Python function."""

    __slots__ = ("fn",)

    def __init__(self, fn: Callable[..., Any]) -> None:
        self.fn = fn


def apply(c: Any, args: list[Any]) -> Any:
    """Invoke a closure handle with ``args``.  Errors on a non-closure."""
    if not isinstance(c, Closure):
        raise TypeError("apply on non-closure")
    return c.fn(*args)


def make_closure(fn: Callable[..., Any], captures: list[Any]) -> Closure:
    """Build a closure that prepends the captured values to each call's
    arguments."""
    return Closure(lambda *args: fn(*captures, *args))


# --- Global store ----------------------------------------------------------

_globals: dict[str, Any] = {}


def global_set(name: Any, value: Any) -> Any:
    """Store ``value`` under ``name`` (a string or :class:`Symbol`)."""
    key = name.name if isinstance(name, Symbol) else str(name)
    _globals[key] = value
    return value


def global_get(name: Any) -> Any:
    """Fetch a global by ``name`` (string or :class:`Symbol`).  Errors if
    undefined."""
    key = name.name if isinstance(name, Symbol) else str(name)
    if key not in _globals:
        raise NameError(f"undefined global: {key}")
    return _globals[key]


def global_get_static(name: str) -> Any:
    """Fetch a global by a statically-known string name."""
    if name not in _globals:
        raise NameError(f"undefined global: {name}")
    return _globals[name]


# --- Printing --------------------------------------------------------------


def sir_print(v: Any) -> None:
    """Print the SIR display form of ``v`` followed by a newline."""
    print(values.to_display(v))
    return None


# --- Builtin dispatch ------------------------------------------------------

_builtins: dict[str, Callable[..., Any]] = {
    "+": arithmetic.add,
    "-": arithmetic.sub,
    "*": arithmetic.mul,
    "/": arithmetic.div,
    "=": values.eq,
    "<": arithmetic.lt,
    ">": arithmetic.gt,
    "cons": pairs.cons,
    "car": pairs.car,
    "cdr": pairs.cdr,
    "null?": values.is_null,
    "pair?": pairs.is_pair,
    "number?": values.is_number,
    "symbol?": values.is_symbol,
    "print": sir_print,
}


def call_builtin(name: str, args: list[Any]) -> Any:
    """Invoke a builtin by SIR name with a list of arguments.

    The SIR backends translate most builtins to native code or route them to a
    dedicated per-concern runtime package, so this generic dispatch only fires
    for the small set the core registers (arithmetic / pairs / print / type
    predicates).  An unregistered name means the emitting backend produced a
    `call_builtin("<name>", …)` for a builtin it does not yet lower — a backend
    coverage gap, not a user error — so the message names the builtin and points
    there rather than reading like a missing Python name.
    """
    fn = _builtins.get(name)
    if fn is None:
        known = ", ".join(sorted(_builtins))
        raise NameError(
            f"SIR builtin {name!r} is not implemented in sir-runtime-core's "
            f"dispatch table (known: {known}). The backend emitted a "
            f"call_builtin for a builtin it does not lower natively or via a "
            f"per-concern runtime package; this is a backend coverage gap."
        )
    return fn(*args)


def builtin_closure(name: str) -> Closure:
    """Wrap a builtin as a first-class :class:`Closure`."""
    return Closure(lambda *args: call_builtin(name, list(args)))
