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

import re
from collections.abc import Callable
from typing import Any

# Block-taking catalog methods (each/map/select/…) invoke a Ruby block.  A block
# reaches us as a trailing ``Closure`` from ``sir-runtime-core``; ``apply`` calls
# it with proc-lenient arity, and ``truthy`` applies SIR truthiness (only
# ``False``/``nil`` are falsy) to predicate results.  ``intern`` mints the
# :class:`Symbol` that ``String#to_sym`` returns.
from coding_adventures_sir_runtime_core import Closure, apply, intern, truthy

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
    }
)

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
        "any?",
        "all?",
        "none?",
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
    }
)

# Block-taking ``String`` methods (M1c); ``each_char`` yields one character.
_STRING_BLOCK_METHODS = frozenset({"each_char"})


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
    # ``str`` is checked before ``list``/``dict`` would be irrelevant (a str is
    # neither), but note bools are ints — order vs Array/Hash does not collide.
    if isinstance(recv, str):
        return name in _STRING_METHODS or name in _STRING_BLOCK_METHODS
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
    return _MISS


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
    if name == "flat_map":
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
        if args[0] in recv:
            return recv[args[0]]
        return args[1] if len(args) > 1 else None
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
    return _MISS


def _string_block_method(recv: str, name: str, block: Closure) -> Val:
    """Block-taking ``String`` methods.  Returns :data:`_MISS` if ``name`` is not
    a string block method."""
    if name == "each_char":
        for char in recv:
            apply(block, [char])
        return recv
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
    """
    if name in ("is_a?", "kind_of?"):
        return is_a(recv, _class_name_arg(args[0]))
    if name == "instance_of?":
        return class_of(recv) == _class_name_arg(args[0])
    if name == "class":
        return class_of(recv)

    fn = _methods.get(name)
    if fn is not None:
        return fn(recv, list(args))

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
    result = _object_method(recv, name, arg_list)
    if result is not _MISS:
        return result

    return None


def reset_oop() -> None:
    """Reset all OOP runtime state — class registry, self stack, instance/class
    variable stores, and the method table.  Primarily for test isolation.
    """
    _classes.clear()
    _self_stack.clear()
    _default_self.ivars.clear()
    _cvars.clear()
    _methods.clear()
