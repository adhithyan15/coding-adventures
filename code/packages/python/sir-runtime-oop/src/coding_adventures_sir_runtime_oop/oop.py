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

import math
import re
from collections.abc import Callable
from typing import Any

# Block-taking catalog methods (each/map/select/…) invoke a Ruby block.  A block
# reaches us as a trailing ``Closure`` from ``sir-runtime-core``; ``apply`` calls
# it with proc-lenient arity, and ``truthy`` applies SIR truthiness (only
# ``False``/``nil`` are falsy) to predicate results.  ``intern`` mints the
# :class:`Symbol` that ``String#to_sym`` / ``Symbol#upcase`` return.
from coding_adventures_sir_runtime_core import Closure, Symbol, apply, eq, intern, truthy

# Typed-error entry point (T1).  A faulting Ruby method — ``arr.fetch`` out of
# bounds, ``hash.fetch`` on a missing key, an unknown method — must raise the
# *typed* :class:`SirError` the rescue matcher names (``IndexError`` / ``KeyError``
# / ``NoMethodError``), not the ``nil`` floor and not a native Python error (which
# the matcher only sees as an over-broad ``StandardError``).  ``raise_error`` is
# the same explicit-string raise the frontend already emits for ``raise Klass`` —
# no reflection on the source-derived method name (the C3 RCE lesson).
from coding_adventures_sir_runtime_exceptions import raise_error

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


# ── User class/instance method tables (O1) ───────────────────────────────────
#
# The Ruby→SIR frontend HOISTS every method to a detached top-level function,
# so nothing in the IR records that ``speak`` belongs to ``Dog``.  We recover
# that association at *runtime* with two explicit tables, populated by emitted
# ``__def_method__`` / ``__def_class_method__`` registrations:
#
#     _instance_methods[(class_name, method_name)] -> Closure   # def m
#     _class_methods[(class_name, method_name)]    -> Closure   # def self.m
#
# **Model.**  A method value is a ``sir-runtime-core`` :class:`Closure` (the
# hoisted top-level function captured by ``MakeClosure``); it is invoked with
# :func:`apply` — never ``getattr``/``eval``/reflection on the source-derived
# name (the C3 RCE lesson).  Dispatch is *always* an explicit dict lookup on the
# ``(class, method)`` key, walking the registered ancestry chain.
#
# **Self / receiver.**  Instance-method dispatch and ``call_new`` push the
# receiver onto the process-global self-stack (:func:`push_self`) before
# invoking the body and pop after, so ``@ivar`` access inside the body reads the
# right object with no explicit ``self`` parameter.  ``call_super`` runs in the
# *same* receiver (no push/pop) — ``super`` is a re-dispatch on the current self.
# This is the single-threaded v0 model documented at the top of this module;
# true per-object/per-thread binding is out of scope for v0.
#
# The ``(class, method)`` pair is stored as a 2-tuple key here (Python dicts key
# on tuples by value); the TypeScript mirror uses a ``"class\x00method"`` string
# key because JS ``Map`` keys arrays by identity, not value.  Both are pure
# value lookups with no reflection.

_instance_methods: dict[tuple[str, str], Closure] = {}
_class_methods: dict[tuple[str, str], Closure] = {}


# ── Mixins: included-modules table + Ruby MRO (MX2) ──────────────────────────
#
# Ruby's ``module M; def foo; …; end; end`` registers ``foo`` in the SAME method
# table as a class def — the *owner* key is simply the module name ``"M"`` (the
# frontend emits ``__def_method__("M", "foo", ⟨foo⟩)`` for a module body, exactly
# as it does for a class body).  A ``class C; include M; end`` then makes ``M``'s
# methods reachable from a ``C`` instance.  We record that mixing-in with one
# explicit table — NEVER reflection on a source-derived name (the C3 RCE lesson):
#
#     _included_modules[owner] = [M1, M2, …]   # append in *include order*
#
# Ruby searches the most-recently-included module first, so the resolution walk
# iterates this list in **reverse**.  Method resolution follows Ruby's MRO:
#
#     receiver class C
#       → C's own methods
#       → C's included modules, most-recent-first (each module's own methods,
#         and *its* included modules, depth-first)
#       → C's superclass
#       → the superclass's modules
#       → … → Object
#
# A diamond (a module reachable by two paths) is de-duplicated: the FIRST time an
# owner is visited fixes its position; later re-encounters are skipped.  The walk
# is cycle-guarded by the same ``seen``-set — a module that (transitively)
# includes itself terminates rather than looping.  ``extend`` reuses this table
# indirectly: :func:`extend_module` copies a module's instance methods into the
# owner's *class-method* table so they answer as ``Owner.method`` / singleton.

_included_modules: dict[str, list[str]] = {}


def def_method(class_name: str, method_name: str, fn: Closure) -> None:
    """Register instance method ``method_name`` for ``class_name`` (``def m``).

    Emitted by the frontend as ``__def_method__``; ``fn`` is the hoisted
    top-level function as a :class:`Closure`.  The ``class_name`` owner key may
    be a *module* name — a module body's ``def`` registers identically, which is
    what :func:`include_module` / :func:`extend_module` then draw from.
    """
    _instance_methods[(class_name, method_name)] = fn


def def_class_method(class_name: str, method_name: str, fn: Closure) -> None:
    """Register class method ``method_name`` for ``class_name`` (``def self.m``).

    Emitted by the frontend as ``__def_class_method__``.
    """
    _class_methods[(class_name, method_name)] = fn


def include_module(owner: str, module_name: str) -> None:
    """Mix ``module_name``'s instance methods into ``owner`` (``include M``).

    Emitted by the frontend as ``__include__("Owner", "M")``.  Appends ``M`` to
    ``owner``'s included-modules list **in include order**; the MRO walk
    (:func:`_owner_mro`) searches this list in reverse, so a later ``include``
    shadows an earlier one — matching Ruby, where the most-recently-included
    module wins.  Re-including a module appends it again (harmless: the MRO
    de-dups by first occurrence, so the earliest position stands).
    """
    _included_modules.setdefault(owner, []).append(module_name)


def extend_module(owner: str, module_name: str) -> None:
    """Mix ``module_name``'s instance methods in as ``owner``'s CLASS methods
    (``extend M``).

    Emitted by the frontend as ``__extend__("Owner", "M")``.  Ruby's ``extend``
    adds a module's *instance* methods to the receiver's singleton — for a class
    receiver that means they become callable as ``Owner.method``.  We realise
    that by copying every ``(module_name, m)`` entry from the instance-method
    table into the class-method table under ``owner``, so :func:`call_class_method`
    (a plain ``(class, method)`` dict lookup — no reflection) resolves them.  A
    class method the owner defines itself is registered *after* class-body defs
    run, so an explicit ``def self.m`` still wins if it was registered later; in
    practice the frontend emits ``extend`` directives after the owner's own defs,
    matching Ruby's "own class method shadows the extended module" order.
    """
    for (mod, method_name), fn in list(_instance_methods.items()):
        if mod == module_name:
            _class_methods.setdefault((owner, method_name), fn)


def _owner_mro(class_name: str) -> list[str]:
    """The Ruby method-resolution order for a receiver of class ``class_name``.

    Produces the linearised, de-duplicated owner list: for each class in the
    superclass chain we emit the class itself, then its included modules
    depth-first and most-recent-first (a module's own included modules follow
    it).  A given owner appears at most once — its FIRST occurrence fixes its
    position, so a diamond include resolves to a single slot.  Cycle-guarded by
    the shared ``seen`` set (a self-including module or a superclass cycle
    terminates).
    """
    order: list[str] = []
    seen: set[str] = set()

    def visit_module(mod: str) -> None:
        if mod in seen:
            return
        seen.add(mod)
        order.append(mod)
        # A module's own included modules, most-recent-first (reverse), depth
        # first — same discipline as a class's modules.
        for sub in reversed(_included_modules.get(mod, [])):
            visit_module(sub)

    cur: str | None = class_name
    while cur is not None and cur not in seen:
        seen.add(cur)
        order.append(cur)
        for mod in reversed(_included_modules.get(cur, [])):
            visit_module(mod)
        cur = superclass_of(cur)
    return order


def _resolve_instance_method(class_name: str, method_name: str) -> Closure | None:
    """Find ``method_name`` on ``class_name`` or any MRO ancestor.

    Walks the Ruby method-resolution order (:func:`_owner_mro` — class → its
    included modules reverse → superclass → its modules → … Object) and returns
    the first matching :class:`Closure`, or ``None`` if unresolved.  The MRO is
    cycle- and diamond-safe.
    """
    for owner in _owner_mro(class_name):
        fn = _instance_methods.get((owner, method_name))
        if fn is not None:
            return fn
    return None


def _resolve_class_method(class_name: str, method_name: str) -> Closure | None:
    """Find class method ``method_name`` on ``class_name`` or an ancestor
    (cycle-guarded ancestry walk); ``None`` if unresolved."""
    cur: str | None = class_name
    seen: set[str] = set()
    while cur is not None and cur not in seen:
        fn = _class_methods.get((cur, method_name))
        if fn is not None:
            return fn
        seen.add(cur)
        cur = superclass_of(cur)
    return None


def call_new(class_name: str, *args: Val) -> SirInstance:
    """Allocate a ``class_name`` instance and run its ``initialize`` (``Foo.new``).

    Allocates via :func:`new_instance`, pushes the new object as the current
    self, and — if an ``initialize`` is registered for ``class_name`` or any
    ancestor — invokes it with ``args`` (so ``@ivar`` assignments in the
    constructor land on the new object).  Always pops self and returns the
    object, even when there is no ``initialize`` (a plain allocation).
    """
    obj = new_instance(class_name)
    push_self(obj)
    try:
        initializer = _resolve_instance_method(class_name, "initialize")
        if initializer is not None:
            apply(initializer, list(args))
    finally:
        pop_self()
    return obj


def call_super(method_name: str, class_name: str, *args: Val) -> Val:
    """Dispatch ``super`` — re-run ``method_name`` from ``class_name``'s parent.

    Walks from ``superclass_of(class_name)`` upward and invokes the first
    ancestor implementation of ``method_name`` with ``args``, keeping the
    *current* self bound (``super`` runs in the same receiver, so no push/pop).
    If no ancestor defines the method, returns ``None`` (Ruby ``nil``) — the
    runtime's honest floor, consistent with :func:`call_method`.
    """
    parent = superclass_of(class_name)
    if parent is None:
        return None
    fn = _resolve_instance_method(parent, method_name)
    if fn is None:
        return None
    return apply(fn, list(args))


def call_class_method(class_name: str, method_name: str, *args: Val) -> Val:
    """Dispatch a class method (``Foo.bar`` for ``def self.bar``).

    Looks up ``method_name`` in the class-method table walking ``class_name``'s
    ancestry and applies it; returns ``None`` if unresolved.  (``Foo.new`` is
    the implicit class method but routes to :func:`call_new`, not here.)
    """
    fn = _resolve_class_method(class_name, method_name)
    if fn is None:
        return None
    return apply(fn, list(args))


def current_self() -> Val:
    """The current receiver (top of the self-stack), or ``None`` if empty.

    Backs the ``__self__`` builtin — a bare ``self`` in a method body.  Named
    ``current_self`` (not ``self``) to avoid the Python keyword clash.  Returns
    ``None`` (Ruby ``nil``) at top level where no receiver is bound rather than
    the internal default-self sentinel.
    """
    return _self_stack[-1] if _self_stack else None


# ── Built-in method catalog (SIR method-dispatch spec, M1) ───────────────────
#
# ``recv.meth(args…)`` reaches the backend as ``BuiltinCall("__method__",
# [recv, "meth", …])`` and is dispatched here.  Before this catalog every method
# outside the reflective four (``is_a?`` etc.) + the ``define_method`` table
# returned ``nil`` — so ``[1,2,3].reverse`` evaluated to nil instead of running.
# This catalog gives the everyday Ruby built-ins their faithful native behaviour,
# dispatched by the receiver's runtime type.  See
# ``code/specs/sir-method-dispatch.md``.
#
# **This file (M1a) covers the *non-block* Array surface plus the universal
# Object methods.**  Block-taking methods (``each``/``map``/``select``/…) and the
# Hash/String/Numeric/Symbol catalogs arrive in follow-up PRs that take the
# ``sir-runtime-core`` ``apply`` dependency for proc-lenient block invocation.
#
# Resolution order (see :func:`call_method`): reflective built-ins → user
# ``define_method`` table → this catalog → ``nil`` floor.  ``respond_to?`` reports
# catalog membership honestly, so an out-of-catalog method is both ``nil`` *and*
# ``respond_to? == False``.

# Sentinel meaning "this name is not in the catalog for this receiver" — distinct
# from a catalog method that legitimately returns ``None`` (Ruby ``nil``).
_MISS = object()

# Universal methods available on *every* receiver (Ruby's ``Object``/``Kernel``).
# ``to_s``/``inspect`` render Ruby display forms (see :func:`_ruby_to_s` /
# :func:`_ruby_inspect`); they live here so ``nil``/``true``/``false`` need no
# catalog of their own (``nil.to_s == ""``, ``true.to_s == "true"``,
# ``nil.inspect == "nil"``), and ``nil.to_a == []`` is handled below.
_OBJECT_METHODS = frozenset(
    {
        "nil?",
        "==",
        "!=",
        "equal?",
        "respond_to?",
        "freeze",
        "frozen?",
        "dup",
        "clone",
        "itself",
        "to_a",
        "to_s",
        "inspect",
        # Kernel flow-control methods (M6).  ``send``/``__send__`` re-enter
        # dispatch with a dynamic method name; ``tap`` and ``then``/``yield_self``
        # are the block-taking pair (handled in :func:`_object_block_method`),
        # but they are listed here so ``respond_to?`` reports them on *every*
        # receiver — block-less and block-bearing calls alike resolve.
        "send",
        "__send__",
        "public_send",
        "tap",
        "then",
        "yield_self",
    }
)

# Block-taking universal methods (M6): ``tap`` yields the receiver and returns
# it; ``then``/``yield_self`` yield the receiver and return the block's result.
# Dispatched in :func:`_object_block_method` only when a trailing ``Closure`` is
# present (block-less ``tap``/``then`` fall through to the receiver-identity
# floor in :func:`_object_method`).
_OBJECT_BLOCK_METHODS = frozenset({"tap", "then", "yield_self"})

# ``Symbol``-routing methods (M6): the receiver's *first* argument names the
# method to dispatch.  Listed for ``respond_to?`` honesty and split out in
# :func:`call_method` because they recurse through dispatch with a dynamic name.
_SEND_METHODS = frozenset({"send", "__send__", "public_send"})

# ``TrueClass``/``FalseClass`` boolean logic (M6).  Ruby's ``&`` and ``|`` on a
# boolean are *non-short-circuiting* logical operators (``true & nil == false``,
# ``false | 1 == true``), distinct from the lazy ``&&``/``||`` keywords.  ``^`` is
# logical XOR.  These resolve on a ``bool`` receiver *before* the universal
# ``Object`` table so ``true & false`` runs rather than bottoming out at ``nil``.
_BOOL_METHODS = frozenset({"&", "|", "^"})

# Non-block ``Array`` methods (M1a).  Block methods (``each``/``map``/…) land in
# a later PR; they are deliberately absent here so ``respond_to?`` stays honest.
_ARRAY_METHODS = frozenset(
    {
        "length",
        "size",
        "count",
        "first",
        "last",
        "include?",
        "index",
        "push",
        "append",
        "<<",
        "pop",
        "shift",
        "unshift",
        "prepend",
        "reverse",
        "sort",
        "min",
        "max",
        "sum",
        "uniq",
        "flatten",
        "compact",
        "empty?",
        "to_a",
        "join",
        "fetch",
        "take",
        "drop",
        "values_at",
        "rotate",
        "zip",
    }
)

# Block-taking ``Array`` / ``Enumerable`` methods (M1b).  Each invokes a trailing
# ``Closure`` block via :func:`apply`.  Listed in :func:`_responds_to` so
# ``respond_to?`` reports them, and dispatched in :func:`_array_block_method`.
_ARRAY_BLOCK_METHODS = frozenset(
    {
        "each",
        "each_with_index",
        "map",
        "collect",
        "select",
        "filter",
        "reject",
        "reduce",
        "inject",
        "find",
        "detect",
        "flat_map",
        "collect_concat",
        "any?",
        "all?",
        "none?",
        "sort_by",
        "min_by",
        "max_by",
        "group_by",
        "partition",
        "take_while",
        "drop_while",
        "count",
        "each_with_object",
    }
)

# Non-block ``Hash`` methods (M1c).  Hash is a Python ``dict``.
_HASH_METHODS = frozenset(
    {
        "keys",
        "values",
        "has_key?",
        "key?",
        "include?",
        "member?",
        "has_value?",
        "value?",
        "fetch",
        "size",
        "length",
        "empty?",
        "to_a",
        "dig",
        "store",
        "[]=",
        "merge",
        "delete",
        "clear",
        "invert",
    }
)

# Block-taking ``Hash`` methods (M1c); the block receives ``[key, value]``.
_HASH_BLOCK_METHODS = frozenset(
    {
        "each",
        "each_pair",
        "map",
        "select",
        "filter",
        "reject",
        "each_key",
        "each_value",
    }
)

# Non-block ``String`` methods (M1c).  A Ruby ``String`` is a Python ``str``,
# which is **immutable** — so every method here is non-mutating and returns a
# fresh value (Ruby's bang-free methods do the same; the in-place ``upcase!``
# family is out of v0 scope).  ``sub``/``gsub`` here are the *literal* forms:
# the pattern is matched as a plain substring, never a regex, and the replacement
# is inserted verbatim (no ``\1``/``&`` back-reference expansion).
_STRING_METHODS = frozenset(
    {
        "length",
        "size",
        "upcase",
        "downcase",
        "capitalize",
        "reverse",
        "strip",
        "lstrip",
        "rstrip",
        "chomp",
        "chars",
        "bytes",
        "split",
        "include?",
        "start_with?",
        "end_with?",
        "index",
        "replace",
        "sub",
        "gsub",
        "to_i",
        "to_f",
        "to_sym",
        "empty?",
        "*",
        "+",
        "ljust",
        "rjust",
        "center",
        "swapcase",
        "tr",
        "count",
        "delete",
        "squeeze",
    }
)

# Block-taking ``String`` methods (M1c); ``each_char`` yields one character.
_STRING_BLOCK_METHODS = frozenset({"each_char"})

# Non-block ``Integer`` / ``Float`` methods (M1c).  A Ruby ``Integer`` is a
# Python ``int`` and ``Float`` a ``float`` — but ``bool`` is a subclass of
# ``int`` in Python, so :func:`call_method` routes ``True``/``False`` to the
# universal ``Object`` methods (``true.to_s == "true"``) *before* this catalog.
_NUMERIC_METHODS = frozenset(
    {
        "abs",
        "to_i",
        "to_f",
        "even?",
        "odd?",
        "zero?",
        "positive?",
        "negative?",
        "succ",
        "next",
        "pred",
        "floor",
        "ceil",
        "round",
        "divmod",
        "fdiv",
        "clamp",
        "between?",
        "gcd",
        "pow",
        "**",
        "digits",
    }
)

# Block-taking ``Integer`` methods (M1c): each invokes the block N times.
_NUMERIC_BLOCK_METHODS = frozenset({"times", "upto", "downto", "step"})

# ``Symbol`` methods (M1c).  A Ruby ``Symbol`` is a ``sir-runtime-core``
# :class:`Symbol`; ``upcase``/``downcase`` return a *new* interned symbol.
_SYMBOL_METHODS = frozenset(
    {
        "to_s",
        "to_sym",
        "length",
        "size",
        "upcase",
        "downcase",
        "inspect",
        "empty?",
    }
)


def _method_name(arg: Val) -> str:
    """Coerce a ``respond_to?`` argument (a :class:`Symbol`, ``":m"``-ish string,
    or bare name) to the plain method name used as the catalog key."""
    name = getattr(arg, "name", None)
    return name if isinstance(name, str) else str(arg)


def _responds_to(recv: Val, name: str) -> bool:
    """Whether dispatch on ``recv`` resolves ``name`` — across the reflective
    built-ins, the ``define_method`` table, and the type-specific catalog."""
    if name in ("is_a?", "kind_of?", "instance_of?", "class"):
        return True
    if name in _methods:
        return True
    if name in _OBJECT_METHODS:
        return True
    # ``str`` is checked before ``list``/``dict`` (a str is neither).  ``bool`` is
    # a subclass of ``int`` so it is excluded from the numeric check — bools only
    # resolve the universal ``Object`` methods (handled above).
    if isinstance(recv, str):
        return name in _STRING_METHODS or name in _STRING_BLOCK_METHODS
    if isinstance(recv, Symbol):
        return name in _SYMBOL_METHODS
    if isinstance(recv, bool):
        return name in _BOOL_METHODS
    if isinstance(recv, (int, float)):
        return name in _NUMERIC_METHODS or name in _NUMERIC_BLOCK_METHODS
    if isinstance(recv, list):
        return name in _ARRAY_METHODS or name in _ARRAY_BLOCK_METHODS
    if isinstance(recv, dict):
        return name in _HASH_METHODS or name in _HASH_BLOCK_METHODS
    return False


def _flatten(seq: Val) -> list[Val]:
    """Recursively flatten nested lists (Ruby ``Array#flatten``)."""
    out: list[Val] = []
    for item in seq:
        if isinstance(item, list):
            out.extend(_flatten(item))
        else:
            out.append(item)
    return out


def _uniq(seq: list[Val]) -> list[Val]:
    """Order-preserving de-duplication (Ruby ``Array#uniq``)."""
    out: list[Val] = []
    for item in seq:
        if item not in out:
            out.append(item)
    return out


def _object_method(recv: Val, name: str, args: list[Val]) -> Val:
    """Universal ``Object`` methods.  Returns :data:`_MISS` if ``name`` is not a
    universal method."""
    if name == "nil?":
        return recv is None
    if name == "==":
        return recv == args[0]
    if name == "!=":
        return recv != args[0]
    if name == "equal?":
        return recv is args[0]
    if name == "respond_to?":
        return _responds_to(recv, _method_name(args[0]))
    if name == "itself":
        return recv
    if name in ("freeze",):
        # No true immutability in v0 — identity-returning, matching Ruby's API
        # shape (``freeze`` returns the receiver).
        return recv
    if name == "frozen?":
        # v0: nothing is frozen except the always-frozen immutable primitives.
        return recv is None or isinstance(recv, (bool, int, float))
    if name in ("dup", "clone"):
        if isinstance(recv, list):
            return list(recv)
        if isinstance(recv, dict):
            return dict(recv)
        return recv
    if name == "to_a":
        # Ruby: nil.to_a == [], Array#to_a == self; other receivers fall through.
        if recv is None:
            return []
        if isinstance(recv, list):
            return recv
        return _MISS
    if name == "to_s":
        return _ruby_to_s(recv)
    if name == "inspect":
        return _ruby_inspect(recv)
    if name == "tap":
        # Block-less ``tap`` (no ``Closure`` reached :func:`_object_block_method`)
        # still returns the receiver — Ruby returns an Enumerator-less self in v0.
        return recv
    if name in ("then", "yield_self"):
        # Block-less ``then``/``yield_self`` returns the receiver (Ruby returns an
        # Enumerator; v0 floor — see the spec's "Out of scope" note).
        return recv
    return _MISS


def _object_block_method(recv: Val, name: str, block: Closure) -> Val:
    """Block-taking universal methods (Kernel).  ``block`` is applied via
    :func:`apply` with the receiver as its single argument.  Returns
    :data:`_MISS` if ``name`` is not a universal block method.

    | method                | yields | returns        |
    |-----------------------|--------|----------------|
    | ``tap``               | recv   | **recv**       |
    | ``then``/``yield_self`` | recv | **block result** |

    ``tap`` is the "inspect-in-a-pipeline" method (run a side effect, keep the
    value); ``then``/``yield_self`` is functional "pipe into a block" (replace
    the value with the block's result).
    """
    if name == "tap":
        apply(block, [recv])
        return recv
    if name in ("then", "yield_self"):
        return apply(block, [recv])
    return _MISS


def _bool_method(recv: bool, name: str, args: list[Val]) -> Val:
    """``TrueClass``/``FalseClass`` logical operators (``&``, ``|``, ``^``).

    These are Ruby's *eager* boolean operators (every operand is evaluated — no
    short-circuit), and they coerce the argument by Ruby truthiness: ``nil`` and
    ``false`` are falsy, everything else (``0``, ``""``, …) is truthy.  So
    ``true & nil == false`` and ``false | 0 == true``.  Returns :data:`_MISS` if
    ``name`` is not a boolean operator.
    """
    if name not in _BOOL_METHODS or not args:
        # Not an operator (or called with no operand, e.g. ``true.to_s``) — defer
        # to the universal ``Object`` table rather than indexing an empty ``args``.
        return _MISS
    other = truthy(args[0])
    if name == "&":
        return recv and other
    if name == "|":
        return recv or other
    # name == "^"
    return recv != other


def _array_method(recv: list[Val], name: str, args: list[Val]) -> Val:
    """Non-block ``Array`` methods.  Returns :data:`_MISS` if ``name`` is not a
    catalogued array method."""
    if name in ("length", "size"):
        return len(recv)
    if name == "count":
        return recv.count(args[0]) if args else len(recv)
    if name == "first":
        if args:
            return recv[: args[0]]
        return recv[0] if recv else None
    if name == "last":
        if args:
            return recv[-args[0] :] if args[0] else []
        return recv[-1] if recv else None
    if name == "include?":
        return args[0] in recv
    if name == "index":
        return recv.index(args[0]) if args[0] in recv else None
    if name in ("push", "append"):
        recv.extend(args)
        return recv
    if name == "<<":
        recv.append(args[0])
        return recv
    if name == "pop":
        return recv.pop() if recv else None
    if name == "shift":
        return recv.pop(0) if recv else None
    if name in ("unshift", "prepend"):
        recv[:0] = args
        return recv
    if name == "reverse":
        return list(reversed(recv))
    if name == "sort":
        return sorted(recv)
    if name == "min":
        return min(recv) if recv else None
    if name == "max":
        return max(recv) if recv else None
    if name == "sum":
        total: Val = args[0] if args else 0
        for item in recv:
            total = total + item
        return total
    if name == "uniq":
        return _uniq(recv)
    if name == "flatten":
        return _flatten(recv)
    if name == "compact":
        return [item for item in recv if item is not None]
    if name == "empty?":
        return len(recv) == 0
    if name == "to_a":
        return recv
    if name == "join":
        # Ruby ``Array#join``: elements rendered with ``to_s`` (default sep "").
        sep = args[0] if args else ""
        return sep.join(_ruby_to_s(item) for item in recv)
    if name == "fetch":
        # Ruby ``Array#fetch(i)``: like ``arr[i]`` for an *in-range* index
        # (negative indices count from the end), but an **out-of-bounds** index
        # with no default raises ``IndexError`` (T1) — unlike ``arr[i]``, which
        # returns nil.  A second argument supplies a default returned instead of
        # raising.  (The block form ``fetch(i) { … }`` is out of v0 scope.)
        index = args[0]
        length = len(recv)
        if isinstance(index, int) and -length <= index < length:
            return recv[index]
        if len(args) > 1:
            return args[1]
        raise_error(
            "IndexError",
            f"index {index} outside of array bounds: {-length}...{length}",
        )
    if name in ("take", "drop"):
        # Ruby ``Array#take(n)`` / ``#drop(n)``: the first ``n`` elements, or all
        # elements *after* the first ``n``.  ``n`` is clamped to ``[0, len]`` — Ruby
        # raises ``ArgumentError`` on a negative ``n``, but the never-raise floor
        # (mirroring the Go/Rust/JS runtimes) folds a negative count to 0, and
        # Python slicing already saturates ``n > len``.  A non-numeric argument
        # degrades to 0 rather than raising.
        n = int(args[0]) if args and isinstance(args[0], (int, float)) else 0
        if n < 0:
            n = 0
        return recv[:n] if name == "take" else recv[n:]
    if name == "values_at":
        # Ruby ``Array#values_at(*idxs)``: one element per index, with a negative
        # index folded from the end **once**.  An out-of-range index yields ``nil``
        # (``None``) rather than raising — matching the sibling backends.
        length = len(recv)
        out: list[Val] = []
        for arg in args:
            idx = int(arg) if isinstance(arg, (int, float)) else 0
            if idx < 0:
                idx += length
            out.append(recv[idx] if 0 <= idx < length else None)
        return out
    if name == "rotate":
        # Ruby ``Array#rotate(n=1)``: rotate left by ``n`` (a negative ``n`` rotates
        # right).  The modulo wraps so any ``n`` terminates; an empty array is ``[]``.
        # No arg defaults to 1; a non-numeric arg degrades to 0 (never raises),
        # matching the Go/Rust runtimes.
        length = len(recv)
        if length == 0:
            return []
        if not args:
            n = 1
        elif isinstance(args[0], (int, float)):
            n = int(args[0])
        else:
            n = 0
        shift = n % length  # Python ``%`` folds negatives into ``[0, length)``
        return recv[shift:] + recv[:shift]
    if name == "zip":
        # Ruby ``Array#zip(*others)``: an Array of tuples ``[self[i], others..[i]]``
        # of length ``len(self)``.  A shorter operand pads with ``nil`` (``None``);
        # a non-array operand is treated as empty (pad-only), never raising.
        others = [o if isinstance(o, list) else [] for o in args]
        zipped: list[Val] = []
        for i, x in enumerate(recv):
            row: list[Val] = [x]
            for o in others:
                row.append(o[i] if i < len(o) else None)
            zipped.append(row)
        return zipped
    return _MISS


def _array_block_method(recv: list[Val], name: str, args: list[Val], block: Closure) -> Val:
    """Block-taking ``Array``/``Enumerable`` methods.  ``block`` is applied via
    :func:`apply` (proc-lenient); predicate results route through SIR
    :func:`truthy`.  Returns :data:`_MISS` if ``name`` is not a block method."""
    if name == "each":
        for item in recv:
            apply(block, [item])
        return recv
    if name == "each_with_index":
        for index, item in enumerate(recv):
            apply(block, [item, index])
        return recv
    if name in ("map", "collect"):
        return [apply(block, [item]) for item in recv]
    if name in ("select", "filter"):
        return [item for item in recv if truthy(apply(block, [item]))]
    if name == "reject":
        return [item for item in recv if not truthy(apply(block, [item]))]
    if name in ("reduce", "inject"):
        if args:
            acc: Val = args[0]
            rest = recv
        elif recv:
            acc = recv[0]
            rest = recv[1:]
        else:
            return None
        for item in rest:
            acc = apply(block, [acc, item])
        return acc
    if name in ("find", "detect"):
        for item in recv:
            if truthy(apply(block, [item])):
                return item
        return None
    if name in ("flat_map", "collect_concat"):
        out: list[Val] = []
        for item in recv:
            mapped = apply(block, [item])
            if isinstance(mapped, list):
                out.extend(mapped)
            else:
                out.append(mapped)
        return out
    if name == "any?":
        return any(truthy(apply(block, [item])) for item in recv)
    if name == "all?":
        return all(truthy(apply(block, [item])) for item in recv)
    if name == "none?":
        return not any(truthy(apply(block, [item])) for item in recv)
    if name == "sort_by":
        # Sort by the block-computed key (Ruby ``sort_by``).  Python's sort is
        # stable, matching Ruby.  A key that is not mutually comparable raises
        # ``TypeError`` — identical to the plain ``sort`` arm above.
        return sorted(recv, key=lambda item: apply(block, [item]))
    if name in ("min_by", "max_by"):
        if not recv:
            return None
        chooser = min if name == "min_by" else max
        return chooser(recv, key=lambda item: apply(block, [item]))
    if name == "group_by":
        # A Hash (Python ``dict``) of block key -> list of elements, in
        # first-seen key order.  Keys must be hashable, consistent with the
        # backend's dict-based Hash model.
        groups: dict[Val, list[Val]] = {}
        for item in recv:
            groups.setdefault(apply(block, [item]), []).append(item)
        return groups
    if name == "partition":
        yes: list[Val] = []
        no: list[Val] = []
        for item in recv:
            (yes if truthy(apply(block, [item])) else no).append(item)
        return [yes, no]
    if name == "take_while":
        out2: list[Val] = []
        for item in recv:
            if truthy(apply(block, [item])):
                out2.append(item)
            else:
                break
        return out2
    if name == "drop_while":
        out3: list[Val] = []
        dropping = True
        for item in recv:
            if dropping and truthy(apply(block, [item])):
                continue
            dropping = False
            out3.append(item)
        return out3
    if name == "count":
        # ``count { |x| pred }`` — number of truthy results.  (The argument and
        # bare forms are handled by the non-block ``_array_method``.)
        return sum(1 for item in recv if truthy(apply(block, [item])))
    if name == "each_with_object":
        # ``each_with_object(memo) { |x, memo| … }`` — yields each element with
        # the memo and returns the (mutated) memo.
        if not args:
            return recv
        memo = args[0]
        for item in recv:
            apply(block, [item, memo])
        return memo
    return _MISS


def _hash_method(recv: dict[Val, Val], name: str, args: list[Val]) -> Val:
    """Non-block ``Hash`` methods (Hash is a ``dict``).  Returns :data:`_MISS` if
    ``name`` is not a catalogued hash method."""
    if name == "keys":
        return list(recv.keys())
    if name == "values":
        return list(recv.values())
    if name in ("has_key?", "key?", "include?", "member?"):
        return args[0] in recv
    if name in ("has_value?", "value?"):
        return args[0] in recv.values()
    if name == "fetch":
        # Ruby ``Hash#fetch(k)``: returns the value for ``k`` if present; a
        # **missing** key with no default raises ``KeyError`` (T1) — unlike
        # ``hash[k]``, which returns nil.  A second argument supplies a default
        # returned instead of raising.  (The block form is out of v0 scope.)
        if args[0] in recv:
            return recv[args[0]]
        if len(args) > 1:
            return args[1]
        raise_error("KeyError", f"key not found: {_ruby_inspect(args[0])}")
    if name in ("size", "length"):
        return len(recv)
    if name == "empty?":
        return len(recv) == 0
    if name == "to_a":
        return [[key, value] for key, value in recv.items()]
    if name == "dig":
        # v0: single-level dig; nested dig is a documented follow-up.
        return recv.get(args[0])
    if name in ("store", "[]="):
        recv[args[0]] = args[1]
        return args[1]
    if name == "merge":
        return {**recv, **args[0]}
    if name == "delete":
        return recv.pop(args[0], None)
    if name == "clear":
        recv.clear()
        return recv
    if name == "invert":
        return {value: key for key, value in recv.items()}
    return _MISS


def _hash_block_method(recv: dict[Val, Val], name: str, block: Closure) -> Val:
    """Block-taking ``Hash`` methods; the block receives ``[key, value]`` (or a
    single key/value for ``each_key``/``each_value``).  Returns :data:`_MISS` if
    ``name`` is not a hash block method."""
    if name in ("each", "each_pair"):
        for key, value in list(recv.items()):
            apply(block, [key, value])
        return recv
    if name == "each_key":
        for key in list(recv.keys()):
            apply(block, [key])
        return recv
    if name == "each_value":
        for value in list(recv.values()):
            apply(block, [value])
        return recv
    if name == "map":
        return [apply(block, [key, value]) for key, value in recv.items()]
    if name in ("select", "filter"):
        return {k: v for k, v in recv.items() if truthy(apply(block, [k, v]))}
    if name == "reject":
        return {k: v for k, v in recv.items() if not truthy(apply(block, [k, v]))}
    return _MISS


# Leading-numeric extractors for ``String#to_i`` / ``String#to_f``.  Ruby parses
# an optional sign and the longest leading numeric run, ignoring surrounding
# whitespace, and yields ``0`` / ``0.0`` when nothing numeric leads — never an
# error (unlike Python's ``int()``/``float()``, which raise).
_INT_PREFIX = re.compile(r"[+-]?\d+")
_FLOAT_PREFIX = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?")


def _str_to_i(s: str) -> int:
    """Ruby ``String#to_i``: leading integer, else ``0``."""
    match = _INT_PREFIX.match(s.strip())
    return int(match.group()) if match else 0


def _str_to_f(s: str) -> float:
    """Ruby ``String#to_f``: leading float, else ``0.0``."""
    match = _FLOAT_PREFIX.match(s.strip())
    return float(match.group()) if match else 0.0


# Upper bound on the character count ``String#*`` will produce.  A repeat count
# can come from untrusted input (e.g. ``gets.to_i``); without a cap a hostile
# count would attempt a multi-gigabyte allocation (``MemoryError``).  Past the
# cap we yield the empty string rather than raise — honouring the runtime's
# "never raise on the OO surface" invariant.
_MAX_REPEAT_LEN = 100_000_000


def _str_repeat(s: str, count: Val) -> str:
    """Ruby ``String#*``: ``s`` repeated ``count`` times.

    Non-positive counts yield ``""`` (Ruby raises ``ArgumentError`` on a negative
    count, but the runtime floor is to never raise), and an over-large product is
    clamped to :data:`_MAX_REPEAT_LEN` characters to bound a DoS from a hostile
    count.
    """
    n = int(count) if isinstance(count, (int, float)) else 0
    if n <= 0 or not s:
        return ""
    if len(s) * n > _MAX_REPEAT_LEN:
        n = _MAX_REPEAT_LEN // len(s)
    return s * n


def _chomp(s: str, sep: Val) -> str:
    """Ruby ``String#chomp``: drop a trailing record separator.

    With an explicit ``sep`` argument, drop exactly that trailing suffix; with no
    argument, drop one trailing ``\\r\\n``, ``\\n``, or ``\\r`` (Ruby's default
    line ending handling).
    """
    if sep is not None:
        return s[: -len(sep)] if sep and s.endswith(sep) else s
    if s.endswith("\r\n"):
        return s[:-2]
    if s.endswith(("\n", "\r")):
        return s[:-1]
    return s


def _string_method(recv: str, name: str, args: list[Val]) -> Val:
    """Non-block ``String`` methods.  Returns :data:`_MISS` if ``name`` is not a
    catalogued string method.  Every result is a fresh value — ``str`` is
    immutable, so nothing mutates ``recv`` in place."""
    if name in ("length", "size"):
        return len(recv)
    if name == "upcase":
        return recv.upper()
    if name == "downcase":
        return recv.lower()
    if name == "capitalize":
        # Ruby: first char upcased, the rest downcased — exactly ``str.capitalize``.
        return recv.capitalize()
    if name == "reverse":
        return recv[::-1]
    if name == "strip":
        return recv.strip()
    if name == "lstrip":
        return recv.lstrip()
    if name == "rstrip":
        return recv.rstrip()
    if name == "chomp":
        return _chomp(recv, args[0] if args else None)
    if name == "chars":
        return list(recv)
    if name == "bytes":
        return list(recv.encode("utf-8"))
    if name == "split":
        # No argument ⇒ split on runs of whitespace (Ruby's awk-style default);
        # with a separator ⇒ split on that literal substring.
        return recv.split(args[0]) if args else recv.split()
    if name == "include?":
        return args[0] in recv
    if name == "start_with?":
        return recv.startswith(args[0])
    if name == "end_with?":
        return recv.endswith(args[0])
    if name == "index":
        pos = recv.find(args[0])
        return pos if pos >= 0 else None
    if name == "replace":
        # Ruby ``String#replace`` overwrites the whole content; for an immutable
        # ``str`` that is just the replacement value.
        return args[0]
    if name == "sub":
        # Literal first-occurrence replacement; ``str.replace`` is verbatim (no
        # back-reference expansion), so there is no ``$&`` foot-gun here.
        return recv.replace(args[0], args[1], 1)
    if name == "gsub":
        return recv.replace(args[0], args[1])
    if name == "to_i":
        return _str_to_i(recv)
    if name == "to_f":
        return _str_to_f(recv)
    if name == "to_sym":
        return intern(recv)
    if name == "empty?":
        return len(recv) == 0
    if name == "*":
        return _str_repeat(recv, args[0])
    if name == "+":
        return recv + args[0]
    if name in ("ljust", "rjust", "center"):
        # Ruby ``String#ljust``/``#rjust``/``#center(width, pad=" ")``: pad to
        # ``width`` CHARACTERS using ``pad`` cyclically.  ``width <= len(recv)``
        # returns the string unchanged; ``center`` puts any odd extra pad char on
        # the RIGHT (Ruby's rule — the opposite of Python's ``str.center``, which
        # also only accepts a single-char fill).  An empty pad degrades to a
        # single space rather than raising, holding the never-raise floor.
        width = int(args[0]) if args and isinstance(args[0], (int, float)) else 0
        pad = args[1] if len(args) > 1 and isinstance(args[1], str) and args[1] else " "
        # Clamp the padding to `_MAX_REPEAT_LEN` — the same DoS bound `_str_repeat`
        # uses — so a hostile width (e.g. ``"".ljust(10**12)``) cannot OOM the host.
        deficit = min(width - len(recv), _MAX_REPEAT_LEN)
        if deficit <= 0:
            return recv
        if name == "ljust":
            return recv + _str_pad(pad, deficit)
        if name == "rjust":
            return _str_pad(pad, deficit) + recv
        left = deficit // 2
        return _str_pad(pad, left) + recv + _str_pad(pad, deficit - left)
    if name == "swapcase":
        # Ruby ``String#swapcase``: flip the case of each ASCII letter (leaving
        # non-letters and non-ASCII characters untouched), matching the Go/JS
        # runtimes byte-for-byte.
        out = []
        for ch in recv:
            code = ord(ch)
            if 65 <= code <= 90:
                out.append(chr(code + 32))
            elif 97 <= code <= 122:
                out.append(chr(code - 32))
            else:
                out.append(ch)
        return "".join(out)
    if name == "tr":
        # Ruby ``String#tr(from, to)``: translate each char that appears in
        # ``from`` to the char at the same position in ``to``.  A shorter ``to``
        # repeats its LAST char; an empty ``to`` deletes matching chars; when
        # ``from`` repeats a char the last mapping wins.
        # NOTE: the char-RANGE (``"a-z"``) and NEGATION (``"^abc"``) forms are a
        # follow-up, matching the literal-only ``sub``/``gsub`` precedent here.
        if len(args) < 2 or not isinstance(args[0], str) or not isinstance(args[1], str):
            return recv
        frm, to = args[0], args[1]
        table: dict[str, str] = {}
        for i, ch in enumerate(frm):
            table[ch] = (to[i] if i < len(to) else to[-1]) if to else ""
        return "".join(table.get(ch, ch) for ch in recv)
    if name in ("count", "delete", "squeeze"):
        # Char-set methods.  Each ``set`` argument is treated LITERALLY — the set
        # of characters it contains (ranges/negation are a follow-up).  ``count``
        # returns how many chars of ``recv`` lie in the set; ``delete`` removes
        # them; ``squeeze`` collapses consecutive runs (of set chars, or of ALL
        # chars when no set is given).  Multiple set args intersect (Ruby's rule).
        str_sets = [set(a) for a in args if isinstance(a, str)]
        if name == "squeeze" and not str_sets:
            squeezed: list[str] = []
            for ch in recv:
                if not squeezed or squeezed[-1] != ch:
                    squeezed.append(ch)
            return "".join(squeezed)

        def in_all(ch: str) -> bool:
            return bool(str_sets) and all(ch in s for s in str_sets)

        if name == "count":
            return sum(1 for ch in recv if in_all(ch))
        if name == "delete":
            return "".join(ch for ch in recv if not in_all(ch))
        out = []
        for ch in recv:
            if out and out[-1] == ch and in_all(ch):
                continue
            out.append(ch)
        return "".join(out)
    return _MISS


def _str_pad(pad: str, n: int) -> str:
    """Build a padding string of exactly ``n`` characters by repeating ``pad``
    cyclically (truncating the final repeat).  ``n <= 0`` or an empty ``pad``
    yields ``""`` — callers guarantee a non-empty pad, so the guard is defensive."""
    if n <= 0 or not pad:
        return ""
    repeats = (n // len(pad)) + 1
    return (pad * repeats)[:n]


def _string_block_method(recv: str, name: str, block: Closure) -> Val:
    """Block-taking ``String`` methods.  Returns :data:`_MISS` if ``name`` is not
    a string block method."""
    if name == "each_char":
        for char in recv:
            apply(block, [char])
        return recv
    return _MISS


# ── Ruby display forms (to_s / inspect) ──────────────────────────────────────
#
# ``sir-runtime-core``'s ``to_display`` renders *Lisp* forms (``nil``, ``#t``,
# ``#f``), so it is wrong for Ruby's ``to_s``/``inspect``.  These two helpers
# implement Ruby's surface: ``nil.to_s == ""`` but ``nil.inspect == "nil"``;
# booleans print ``true``/``false``; a symbol's ``to_s`` is its bare name and
# ``inspect`` prefixes ``:``; an ``Array``'s ``to_s`` equals its ``inspect``
# (``"[1, 2]"``); a ``Hash`` renders ``{:k=>v}``.  String escaping in ``inspect``
# is the v0 naive form (wrap in quotes; no backslash escaping yet).


def _ruby_to_s(v: Val) -> str:
    """Ruby ``to_s`` display form of ``v``."""
    if v is None:
        return ""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, Symbol):
        return v.name
    if isinstance(v, str):
        return v
    if isinstance(v, (list, dict)):
        return _ruby_inspect(v)
    return str(v)


def _ruby_inspect(v: Val, seen: set[int] | None = None, depth: int = 0) -> str:
    """Ruby ``inspect`` display form of ``v``.

    ``seen`` (a set of container ``id``s) and ``depth`` make this safe on
    self-referential or deeply-nested structures: a cycle renders ``[...]`` /
    ``{...}`` (matching Ruby) instead of recursing forever, and depth is capped
    at :data:`_MAX_DISPLAY_DEPTH` so a deep acyclic structure cannot blow the
    stack.  Both keep the never-raise invariant."""
    if seen is None:
        seen = set()
    if v is None:
        return "nil"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, Symbol):
        return ":" + v.name
    if isinstance(v, str):
        return '"' + v + '"'
    if isinstance(v, list):
        if id(v) in seen or depth > _MAX_DISPLAY_DEPTH:
            return "[...]"
        seen.add(id(v))
        body = ", ".join(_ruby_inspect(item, seen, depth + 1) for item in v)
        seen.discard(id(v))
        return "[" + body + "]"
    if isinstance(v, dict):
        if id(v) in seen or depth > _MAX_DISPLAY_DEPTH:
            return "{...}"
        seen.add(id(v))
        body = ", ".join(
            f"{_ruby_inspect(k, seen, depth + 1)}=>{_ruby_inspect(val, seen, depth + 1)}"
            for k, val in v.items()
        )
        seen.discard(id(v))
        return "{" + body + "}"
    return str(v)


# ── Numeric (Integer / Float) catalog ────────────────────────────────────────


# Bit-length budget bounding the size of values ``**``/``pow`` will materialise
# and ``digits`` will walk.  Exponents come from untrusted input (e.g.
# ``gets.to_i``), and Python ints are arbitrary precision, so ``2 ** (10 ** 9)``
# would allocate ~125 MB and block the interpreter.  ~1M bits (~128 KB / ~315k
# decimal digits) is far above any legitimate use yet bounds a hostile operand.
# Mirrors the ``String#*`` ``_MAX_REPEAT_LEN`` precedent.
_MAX_POW_BITS = 1 << 20

# Recursion bound for ``to_s``/``inspect``/``join`` on nested containers; a
# deeply-nested (or — caught separately by identity tracking — cyclic) structure
# would otherwise blow the Python stack (``RecursionError`` reaches the OO
# surface, violating the never-raise invariant).
_MAX_DISPLAY_DEPTH = 100


def _ruby_round(x: float) -> int:
    """Ruby ``Float#round`` (no digits): round half **away from zero** — unlike
    Python's banker's rounding, ``2.5.round == 3`` and ``-2.5.round == -3``."""
    return math.floor(x + 0.5) if x >= 0 else math.ceil(x - 0.5)


def _safe_pow(base: Val, exp: Val) -> Val:
    """``base ** exp`` with a bignum guard.  An integer result whose approximate
    bit-length exceeds :data:`_MAX_POW_BITS` is refused (returns ``0``) rather
    than allocating gigabytes; a float overflow returns ``inf`` instead of
    raising — both honour the never-raise-on-the-OO-surface invariant."""
    if isinstance(base, int) and isinstance(exp, int):
        if exp > 0 and base not in (0, 1, -1) and exp * base.bit_length() > _MAX_POW_BITS:
            return 0
        return base ** exp
    try:
        return base ** exp
    except OverflowError:
        return math.inf


def _digits(n: int) -> list[int]:
    """Ruby ``Integer#digits``: base-10 digits, least-significant first.  A
    hostile bignum past :data:`_MAX_POW_BITS` is refused (``[0]``) so it cannot
    build a multi-hundred-megabyte list."""
    n = abs(n)
    if n == 0 or n.bit_length() > _MAX_POW_BITS:
        return [0]
    out: list[int] = []
    while n > 0:
        out.append(n % 10)
        n //= 10
    return out


def _numeric_method(recv: Val, name: str, args: list[Val]) -> Val:
    """Non-block ``Integer``/``Float`` methods.  Returns :data:`_MISS` if ``name``
    is not a catalogued numeric method."""
    # ``int()`` raises ``OverflowError``/``ValueError`` on ``inf``/``nan``; the
    # int-coercing methods below degrade to a safe value there rather than
    # raising (never-raise-on-the-OO-surface invariant).
    if isinstance(recv, float) and not math.isfinite(recv):
        if name == "to_i":
            return 0
        if name in ("even?", "odd?"):
            return False
        if name == "gcd":
            return 0
    if name == "abs":
        return abs(recv)
    if name == "to_i":
        return int(recv)
    if name == "to_f":
        return float(recv)
    if name == "even?":
        return int(recv) % 2 == 0
    if name == "odd?":
        return int(recv) % 2 != 0
    if name == "zero?":
        return recv == 0
    if name == "positive?":
        return recv > 0
    if name == "negative?":
        return recv < 0
    if name in ("succ", "next"):
        return recv + 1
    if name == "pred":
        return recv - 1
    # ``floor``/``ceil``/``round`` raise on a non-finite float in Python; return
    # the receiver unchanged there (never-raise floor for ``inf``/``nan``).
    if name == "floor":
        return recv if isinstance(recv, float) and not math.isfinite(recv) else math.floor(recv)
    if name == "ceil":
        return recv if isinstance(recv, float) and not math.isfinite(recv) else math.ceil(recv)
    if name == "round":
        # Ruby ``round`` / ``round(ndigits)``.  With no argument (or ``ndigits <=
        # 0`` on an Integer) the result is an Integer rounded half-away-from-zero;
        # a positive ``ndigits`` on a Float rounds to that many decimal places
        # (still half-away-from-zero, unlike Python's banker's rounding).  A
        # non-finite Float is returned unchanged (never-raise floor).
        #
        # DoS guard: ``ndigits`` is caller-controlled, so ``10 ** (-ndigits)``
        # could build a multi-gigabyte bignum for a hostile magnitude.  Rounding
        # to a place value that dwarfs the receiver is exactly ``0`` in Ruby
        # (``1234.round(-10) == 0``), so we short-circuit to ``0`` once the place
        # count clearly exceeds the receiver's decimal width instead of
        # allocating the factor — and cap positive ``ndigits`` past a Float's
        # precision (the value is already at full precision) to dodge the
        # ``10.0 ** ndigits`` ``OverflowError``.
        ndigits = int(args[0]) if args and isinstance(args[0], (int, float)) else 0
        if isinstance(recv, float) and not math.isfinite(recv):
            return recv
        # Decimal width of the integer magnitude — cheap and bounded.
        int_width = len(str(abs(int(recv)))) if math.isfinite(recv) else 0
        if isinstance(recv, int):
            if ndigits >= 0:
                return recv
            if -ndigits > int_width + 1:
                return 0  # rounding place dwarfs the value ⇒ 0 (Ruby parity)
            factor = 10 ** (-ndigits)
            return int(_ruby_round(recv / factor)) * factor
        if ndigits <= 0:
            if -ndigits > int_width + 1:
                return 0
            factor = 10 ** (-ndigits)
            return int(_ruby_round(recv / factor) * factor)
        # A binary64 Float carries ~15–17 significant digits; rounding to more
        # decimals than that returns the value unchanged (and avoids overflow).
        if ndigits > 17:
            return recv
        factor = 10.0**ndigits
        return _ruby_round(recv * factor) / factor
    if name == "divmod":
        # Ruby ``Integer#divmod`` / ``Float#divmod``: ``[quotient, remainder]``
        # where the quotient is floored and the remainder takes the divisor's
        # sign (Python's ``divmod`` matches this).  Division by zero raises a
        # typed ``ZeroDivisionError`` so a translated ``rescue`` catches it.  A
        # non-numeric divisor degrades to ``0`` → the same typed error (rather
        # than an untyped ``TypeError`` from Python's ``divmod``).
        divisor = args[0] if args and isinstance(args[0], (int, float)) else 0
        if divisor == 0:
            raise_error("ZeroDivisionError", "divided by 0")
        quotient, remainder = divmod(recv, divisor)
        return [quotient, remainder]
    if name == "fdiv":
        # Ruby ``fdiv``: floating-point division.  Unlike ``/``, dividing by zero
        # yields ``Infinity``/``NaN`` rather than raising (Ruby never raises on
        # ``Float`` division), honouring the never-raise floor.  A non-numeric
        # argument degrades to a ``0`` divisor (→ ``Infinity``/``NaN``) rather
        # than raising an untyped ``ValueError``/``TypeError`` from ``float()``.
        divisor = float(args[0]) if args and isinstance(args[0], (int, float)) else 0.0
        numer = float(recv)
        if divisor == 0.0:
            if numer == 0.0:
                return math.nan
            return math.inf if numer > 0 else -math.inf
        return numer / divisor
    if name == "clamp":
        # Ruby ``Comparable#clamp(min, max)``: return ``min`` if ``recv < min``,
        # ``max`` if ``recv > max``, else ``recv``.  (The Range form is deferred.)
        low, high = args[0], args[1]
        if recv < low:
            return low
        if recv > high:
            return high
        return recv
    if name == "between?":
        # Ruby ``Comparable#between?(min, max)``: ``min <= recv <= max``.
        return args[0] <= recv <= args[1]
    if name == "gcd":
        return math.gcd(int(recv), int(args[0]))
    if name in ("pow", "**"):
        return _safe_pow(recv, args[0])
    if name == "digits":
        if isinstance(recv, float) and not math.isfinite(recv):
            return [0]
        return _digits(int(recv))
    return _MISS


def _numeric_block_method(recv: Val, name: str, args: list[Val], block: Closure) -> Val:
    """Block-taking ``Integer`` methods (``times``/``upto``/``downto``/``step``);
    each returns the receiver.  Returns :data:`_MISS` otherwise."""
    if name == "times":
        for i in range(int(recv)):
            apply(block, [i])
        return recv
    if name == "upto":
        for i in range(int(recv), int(args[0]) + 1):
            apply(block, [i])
        return recv
    if name == "downto":
        for i in range(int(recv), int(args[0]) - 1, -1):
            apply(block, [i])
        return recv
    if name == "step":
        limit = args[0]
        stride = args[1] if len(args) > 1 else 1
        value = recv
        if stride > 0:
            while value <= limit:
                apply(block, [value])
                value += stride
        elif stride < 0:
            while value >= limit:
                apply(block, [value])
                value += stride
        return recv
    return _MISS


# ── Symbol catalog ───────────────────────────────────────────────────────────


def _symbol_method(recv: Symbol, name: str, args: list[Val]) -> Val:
    """``Symbol`` methods.  Returns :data:`_MISS` if ``name`` is not catalogued.
    ``upcase``/``downcase`` return a *new* interned symbol (Ruby semantics)."""
    if name == "to_s":
        return recv.name
    if name == "to_sym":
        return recv
    if name in ("length", "size"):
        return len(recv.name)
    if name == "upcase":
        return intern(recv.name.upper())
    if name == "downcase":
        return intern(recv.name.lower())
    if name == "inspect":
        return ":" + recv.name
    if name == "empty?":
        return len(recv.name) == 0
    return _MISS


def call_method(recv: Val, name: str, *args: Val) -> Val:
    """Dispatch method ``name`` on ``recv``.

    Resolution order:

    1. **Reflective built-ins** the SIR frontend emits as ``__method__`` calls —
       ``is_a?``/``kind_of?``/``instance_of?`` (predicate against a class) and
       ``class`` (the class name).
    2. The user :func:`define_method` table.
    3. The **built-in method catalog** (universal ``Object`` methods, and — when
       ``recv`` is a list — the non-block ``Array`` methods).
    4. ``None`` (Ruby ``nil``) for anything still unresolved — the honest floor.
       :func:`_responds_to` reports exactly which names resolve, so an
       out-of-catalog method is both ``nil`` and ``respond_to? == False``.

    The class argument to a predicate may arrive as a class-name **string** or as
    a value whose class is taken; ``instance_of?`` requires an exact
    (non-ancestor) match.

    **User objects (O1).**  When ``recv`` is a user :class:`SirInstance`, a
    method registered via ``def_method`` (walking the class's ancestry) is
    dispatched first: the receiver is pushed as the current self, the stored
    :class:`Closure` is applied with ``args``, and self is popped.  Only if no
    user method resolves does dispatch fall through to the reflective built-ins
    (``is_a?``/``class``/…) and the primitive catalog below — so ``obj.class``
    still works while ``obj.speak`` runs the user body.
    """
    if isinstance(recv, SirInstance):
        user_fn = _resolve_instance_method(recv.sir_class, name)
        if user_fn is not None:
            push_self(recv)
            try:
                return apply(user_fn, list(args))
            finally:
                pop_self()

    if name in ("is_a?", "kind_of?"):
        return is_a(recv, _class_name_arg(args[0]))
    if name == "instance_of?":
        return class_of(recv) == _class_name_arg(args[0])
    if name == "class":
        return class_of(recv)

    # ``send``/``__send__``/``public_send`` re-enter dispatch with a *dynamic*
    # method name taken from the first argument (a Symbol or string), forwarding
    # the rest unchanged — so ``x.send(:upcase)`` is exactly ``x.upcase`` and a
    # trailing block survives as a trailing arg.  An empty arg list (``send`` with
    # no method name) bottoms out at the ``nil`` floor rather than raising.  The
    # user :func:`define_method` table is consulted *first* (resolution order #2),
    # so a user-defined ``send`` override wins; routing recurses through
    # :func:`call_method`.
    fn = _methods.get(name)
    if fn is not None:
        return fn(recv, list(args))

    if name in _SEND_METHODS and args:
        return call_method(recv, _method_name(args[0]), *args[1:])

    arg_list = list(args)
    if isinstance(recv, str):
        # A block method (each_char) dispatches only with a trailing Closure.
        if name in _STRING_BLOCK_METHODS and arg_list and isinstance(arg_list[-1], Closure):
            result = _string_block_method(recv, name, arg_list[-1])
            if result is not _MISS:
                return result
        result = _string_method(recv, name, arg_list)
        if result is not _MISS:
            return result
    elif isinstance(recv, Symbol):
        result = _symbol_method(recv, name, arg_list)
        if result is not _MISS:
            return result
    elif isinstance(recv, bool):
        # bool is a subclass of int — skip the numeric catalog so True/False
        # resolve only the boolean operators (&/|/^) and the universal Object
        # methods (true.to_s == "true").
        result = _bool_method(recv, name, arg_list)
        if result is not _MISS:
            return result
    elif isinstance(recv, (int, float)):
        if name in _NUMERIC_BLOCK_METHODS and arg_list and isinstance(arg_list[-1], Closure):
            result = _numeric_block_method(recv, name, arg_list[:-1], arg_list[-1])
            if result is not _MISS:
                return result
        result = _numeric_method(recv, name, arg_list)
        if result is not _MISS:
            return result
    elif isinstance(recv, list):
        # A block method (each/map/…) is dispatched only when an actual trailing
        # Closure block is present; the block is split off the positional args.
        if name in _ARRAY_BLOCK_METHODS and arg_list and isinstance(arg_list[-1], Closure):
            result = _array_block_method(recv, name, arg_list[:-1], arg_list[-1])
            if result is not _MISS:
                return result
        result = _array_method(recv, name, arg_list)
        if result is not _MISS:
            return result
    elif isinstance(recv, dict):
        if name in _HASH_BLOCK_METHODS and arg_list and isinstance(arg_list[-1], Closure):
            result = _hash_block_method(recv, name, arg_list[-1])
            if result is not _MISS:
                return result
        result = _hash_method(recv, name, arg_list)
        if result is not _MISS:
            return result

    # Universal block-taking methods (``tap``/``then``/``yield_self``) apply to
    # *every* receiver, so they are dispatched here — after the type-specific
    # catalogs — only when an actual trailing ``Closure`` block is present.  A
    # block-less ``tap``/``then`` falls through to :func:`_object_method`, which
    # returns the receiver (the documented v0 Enumerator-less floor).
    if name in _OBJECT_BLOCK_METHODS and arg_list and isinstance(arg_list[-1], Closure):
        result = _object_block_method(recv, name, arg_list[-1])
        if result is not _MISS:
            return result

    result = _object_method(recv, name, arg_list)
    if result is not _MISS:
        return result

    # ── Floor (T1) ────────────────────────────────────────────────────────────
    # Nothing above returned a value for ``name`` on ``recv``.  Two distinct
    # cases hide here, and Ruby treats them differently:
    #
    #   1. **Known method, wrong call shape** — a *catalogued* method invoked in
    #      a form v0 doesn't implement, e.g. a block-taking ``map``/``times``
    #      called *without* a block.  Ruby returns an Enumerator; the honest v0
    #      floor is ``nil`` (documented).  This is NOT a missing method, so it
    #      must not raise ``NoMethodError``.
    #   2. **Genuinely unknown method** — ``obj.undefined``, ``nil.foo``,
    #      ``"s".scan`` (no catalog entry at all).  This is exactly Ruby's
    #      ``NoMethodError`` (T1), replacing the old blanket nil floor.
    #
    # :func:`_responds_to` is the precise discriminator: it reports catalog +
    # reflective + ``define_method`` membership, so a name it knows is case 1
    # (nil) and a name it doesn't is case 2 (raise).  The message mirrors Ruby's
    # shape (``undefined method 'x' for <receiver class>``); ``name`` is
    # interpolated as an opaque string, never used to reflect a Python attribute
    # (the C3 dynamic-dispatch RCE lesson).
    if _responds_to(recv, name):
        return None
    raise_error("NoMethodError", f"undefined method '{name}' for {class_of(recv)}")


# --- Symbol#to_proc (&:sym) ------------------------------------------------
#
# Ruby's ``&:sym`` block argument converts a ``Symbol`` into a block via
# ``Symbol#to_proc``: the resulting proc calls the named method on its first
# argument, forwarding any remaining arguments.  So ``[1, 2, 3].map(&:to_s)``
# is ``[1, 2, 3].map { |x| x.to_s }`` and ``[1, 2].inject(&:+)`` is
# ``inject { |acc, x| acc + x }``.
#
# The Ruby→SIR frontend lowers ``&:sym`` to ``block_pass(SymLit("sym"))``;
# the backend emits the surviving ``block_pass`` envelope as a call to this
# helper (``_sir_oop_sym_to_proc(intern("sym"))``), which yields a
# :class:`Closure` the block-taking catalog methods (``map``/``select``/…)
# drive through :func:`apply` exactly like a ``{ }`` block.
#
# The closure's ``arity`` is ``None`` (variadic): ``apply`` then passes a
# block method's arguments through unadjusted, so the *first* becomes the
# receiver and the rest are forwarded as method arguments.  That matches
# Ruby's ``&:sym`` arity (one required receiver plus a rest) and makes the
# one-arg (``map``) and two-arg (``inject``) shapes both correct.


def sym_to_proc(sym: Val) -> Closure:
    """Build a :class:`Closure` equivalent to Ruby's ``sym.to_proc``.

    ``sym`` is normally a ``sir-runtime-core`` :class:`Symbol` (the emitted
    ``intern("name")``); a bare string is accepted defensively.  Applying the
    returned closure to ``[recv, *rest]`` dispatches ``recv.name(*rest)``
    through :func:`call_method`, so an out-of-catalog method bottoms out at
    ``nil`` rather than raising — upholding the never-raise-on-the-OO-surface
    invariant for the proc body too.
    """
    method = sym.name if isinstance(sym, Symbol) else str(sym)

    def _invoke(recv: Val, *rest: Val) -> Val:
        return call_method(recv, method, *rest)

    return Closure(_invoke, arity=None)


def case_eq(pattern: Val, value: Val) -> bool:
    """Ruby case-equality (``pattern === value``), the test a ``when`` clause
    runs (M5).  Unlike ``==``, the operation is keyed to the *pattern*'s type:

    | pattern kind        | semantics                                  |
    |---------------------|--------------------------------------------|
    | ``Range``           | membership — ``value`` falls in the range  |
    | ``re.Pattern`` (Regexp) | the regex matches ``str(value)``       |
    | anything else       | value equality (``==``)                    |

    The class case (``when Integer``) is handled at the *frontend* — it lowers
    to ``value.is_a?(Const)`` via the ``__method__`` dispatch envelope — so it
    never reaches here.  The else-branch floor is ``eq`` (``==``), so a literal
    ``when 5`` keeps its plain-equality meaning.

    ``Range`` is detected structurally (by type name) rather than imported, so
    ``sir-runtime-oop`` gains no dependency on ``sir-runtime-range``; ``re``
    is already imported by this module.
    """
    if isinstance(pattern, re.Pattern):
        # Ruby `/re/ === x` is true when the regex matches x (a String).  A
        # non-string scrutinee never matches (Ruby returns false), mirrored by
        # `str(value)` only being meaningful for strings — guard explicitly.
        if not isinstance(value, str):
            return False
        return pattern.search(value) is not None
    # `Range` is our own type (from sir-runtime-range); match by name to avoid
    # a package dependency.  A real Range exposes `includes`.
    if type(pattern).__name__ == "Range" and hasattr(pattern, "includes"):
        # Ruby `(10..20) === "x"` is *false*, not an error — a `<`/`>` between
        # incomparable types raises `TypeError` in Python, so swallow it and
        # report no match (mirroring Ruby's case-equality on a mismatched type).
        try:
            return bool(pattern.includes(value))
        except TypeError:
            return False
    return eq(pattern, value)


def reset_oop() -> None:
    """Reset all OOP runtime state — class registry, self stack, instance/class
    variable stores, and the method table.  Primarily for test isolation.
    """
    _classes.clear()
    _self_stack.clear()
    _default_self.ivars.clear()
    _cvars.clear()
    _methods.clear()
    _instance_methods.clear()
    _class_methods.clear()
    _included_modules.clear()
