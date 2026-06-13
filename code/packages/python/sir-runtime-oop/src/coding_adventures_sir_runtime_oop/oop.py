"""OOP runtime primitives for Semantic-IR-emitted Python.

Most SIR constructs translate to *native* Python (a sequence is a ``list``,
a loop is a ``for``).  Ruby-style object orientation does not survive that
translation cleanly, for one structural reason:

    **The Ruby->SIR frontend HOISTS every method to a detached top-level
    function with no receiver (no ``self``).**

So inside an emitted method there is no ``self`` to hang an instance variable
on, and a class-variable assignment carries no enclosing-class context.  Native
attribute access (``self.x``) is therefore impossible.  This module supplies the
missing object model as an explicit, in-process runtime:

    - a **class registry** (``define_class``) + ancestry-aware ``is_a``,
    - an **instance-variable store** addressed through a *current-self* stack
      (``push_self`` / ``pop_self`` / ``ivar_get`` / ``ivar_set``),
    - a **class-variable store** (``cvar_get`` / ``cvar_set``),
    - **method dispatch** (``call_method``) covering the reflective built-ins
      the frontend emits (``is_a?``, ``kind_of?``, ``instance_of?``, ``class``)
      plus a ``define_method`` table for singleton-method attachment.

**Honest v0 limitation.** Because the frontend does not thread receivers, the
*current self* is a process-global stack rather than a true per-call binding,
and class variables share a single namespace keyed by bare name.  This models
single-instance / single-class programs faithfully and never raises on the OO
surface, but full multi-object Ruby semantics await a frontend that carries
receivers into method bodies (out of scope for the backend).  See
``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

# The SIR universal value type at this package's boundary.
Val = Any

# ── Class registry ──────────────────────────────────────────────────────────

# Maps a class name to its (optional) superclass name.
_classes: dict[str, str | None] = {}


def define_class(name: str, super_name: str | None = None) -> None:
    """Register a class and (optionally) its superclass.

    Emitted from a SIR ``ClassDef`` so later ``is_a`` queries can walk the
    ancestry chain.  Re-defining a class replaces its prior registration
    (matching Ruby's open classes).
    """
    _classes[name] = super_name


def superclass_of(name: str) -> str | None:
    """The registered superclass name of ``name``, or ``None`` if none/unknown."""
    return _classes.get(name)


# ── Instances ─────────────────────────────────────────────────────────────────


class SirInstance:
    """A SIR object instance: a class tag plus a bag of instance variables.

    Created by :func:`new_instance`; instance variables are read/written through
    the current-self stack rather than direct attribute access (see module docs).
    """

    __slots__ = ("sir_class", "ivars")

    def __init__(self, sir_class: str) -> None:
        self.sir_class: str = sir_class
        self.ivars: dict[str, Val] = {}


def new_instance(class_name: str) -> SirInstance:
    """Allocate a fresh instance tagged with ``class_name``."""
    return SirInstance(class_name)


# ── Current-self stack + instance-variable store ─────────────────────────────

_self_stack: list[SirInstance] = []

# A program that never pushes a self (the common detached-method case) still
# needs somewhere to put instance variables; this default object provides it so
# ``@x`` reads/writes never raise.
_default_self = SirInstance("Object")


def _current_self() -> SirInstance:
    return _self_stack[-1] if _self_stack else _default_self


def push_self(obj: SirInstance) -> None:
    """Make ``obj`` the receiver for subsequent ``ivar_get``/``ivar_set`` calls."""
    _self_stack.append(obj)


def pop_self() -> None:
    """Pop the most recently pushed receiver."""
    if _self_stack:
        _self_stack.pop()


def ivar_get(name: str) -> Val:
    """Read instance variable ``name`` (incl. the leading ``@``) on the current
    self.  Unset instance variables read as ``None`` (Ruby's ``nil``).
    """
    return _current_self().ivars.get(name)


def ivar_set(name: str, value: Val) -> Val:
    """Set instance variable ``name`` on the current self; returns the value."""
    _current_self().ivars[name] = value
    return value


# ── Class-variable store ─────────────────────────────────────────────────────

_cvars: dict[str, Val] = {}


def cvar_get(name: str) -> Val:
    """Read class variable ``name`` (incl. ``@@``); unset reads as ``None``."""
    return _cvars.get(name)


def cvar_set(name: str, value: Val) -> Val:
    """Set class variable ``name``; returns the value."""
    _cvars[name] = value
    return value


# ── Class identity / ancestry ────────────────────────────────────────────────


def class_of(value: Val) -> str:
    """The Ruby class name of a value.

    Registered :class:`SirInstance` tag for objects, or the conventional
    built-in name for primitives (``Integer``, ``Float``, ``String``, ``Array``,
    ``Hash``, ``NilClass``, ``TrueClass``, ``FalseClass``, else ``Object``).
    """
    if isinstance(value, SirInstance):
        return value.sir_class
    if value is None:
        return "NilClass"
    if isinstance(value, bool):
        return "TrueClass" if value else "FalseClass"
    if isinstance(value, int):
        return "Integer"
    if isinstance(value, float):
        return "Float"
    if isinstance(value, str):
        return "String"
    if isinstance(value, list):
        return "Array"
    if isinstance(value, dict):
        return "Hash"
    return "Object"


def is_a(value: Val, class_name: str) -> bool:
    """``True`` if ``value`` is an instance of ``class_name`` or any registered
    ancestor.

    Primitive built-in names match structurally; ``Numeric`` matches both
    ``Integer`` and ``Float``; ``Object``/``BasicObject`` match everything (the
    universal Ruby roots).  The ancestry walk is cycle-safe.
    """
    if class_name in ("Object", "BasicObject"):
        return True
    actual = class_of(value)
    if class_name == "Numeric":
        return actual in ("Integer", "Float")
    cur: str | None = actual
    seen: set[str] = set()
    while cur is not None and cur not in seen:
        if cur == class_name:
            return True
        seen.add(cur)
        cur = superclass_of(cur)
    return False


# ── Method dispatch ──────────────────────────────────────────────────────────

_methods: dict[str, Callable[[Val, list[Val]], Val]] = {}


def define_method(name: str, fn: Callable[[Val, list[Val]], Val]) -> None:
    """Attach a (singleton/instance) method implementation under ``name``.

    Models ``def obj.m`` / ``class << self`` once a frontend supplies bodies;
    today it backs the :func:`call_method` fallback.
    """
    _methods[name] = fn


def _class_name_arg(arg: Val) -> str:
    return arg if isinstance(arg, str) else class_of(arg)


def call_method(recv: Val, name: str, *args: Val) -> Val:
    """Dispatch reflective method ``name`` on ``recv``.

    Handles the built-ins the SIR frontend emits as ``__method__`` calls —
    ``is_a?``/``kind_of?``/``instance_of?`` (predicate against a class) and
    ``class`` (the class name) — then falls back to a :func:`define_method`
    table, returning ``None`` (nil) for an unknown method rather than raising.

    The class argument to a predicate may arrive as a class-name **string** or as
    a value whose class is taken; ``instance_of?`` requires an exact
    (non-ancestor) match.
    """
    if name in ("is_a?", "kind_of?"):
        return is_a(recv, _class_name_arg(args[0]))
    if name == "instance_of?":
        return class_of(recv) == _class_name_arg(args[0])
    if name == "class":
        return class_of(recv)
    fn = _methods.get(name)
    return fn(recv, list(args)) if fn is not None else None


def reset_oop() -> None:
    """Reset all OOP runtime state — class registry, self stack, instance/class
    variable stores, and the method table.  Primarily for test isolation.
    """
    _classes.clear()
    _self_stack.clear()
    _default_self.ivars.clear()
    _cvars.clear()
    _methods.clear()
