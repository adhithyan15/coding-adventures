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

import inspect
import sys
from collections.abc import Callable
from typing import Any

from . import arithmetic, pairs, values
from .symbols import Symbol

# --- Closures --------------------------------------------------------------


class LocalJumpError(Exception):
    """Raised when a closure-shaped call has no closure to invoke.

    SIR's explicit-block-param ABI threads a method's block as an ordinary
    trailing parameter and lowers ``yield`` to an ``IndirectCall`` through
    it.  When the caller passed **no** block, that parameter is ``nil``
    (Python ``None``), so the ``IndirectCall`` reaches :func:`apply` with a
    ``None`` target.  Ruby raises ``LocalJumpError`` ("no block given
    (yield)") in exactly this situation; we mirror that with a dedicated
    exception so the failure is recognisable rather than a generic
    ``TypeError`` about an internal "non-closure".  The exact Ruby class
    identity is not modelled — this is the SIR analogue, keyed to the
    *shape* of the error, not Ruby's class hierarchy.
    """


class Closure:
    """A callable handle wrapping a Python function.

    Carries two extra facets used for **proc-vs-lambda arity** (Q10g):

    - ``arity`` — the number of *fixed positional* parameters the closure's
      block declares (after the captured values are bound), or ``None`` when
      the block is variadic (a ``*rest`` parameter) or its arity could not be
      introspected.  Used by :func:`apply` to adjust call arguments for
      proc/block leniency.
    - ``is_lambda`` — ``True`` for a Ruby lambda (``->(){}`` / ``lambda{}``),
      which enforces **strict** arity (Ruby raises ``ArgumentError`` on a
      mismatch); ``False`` for a block / proc, which **adjusts** arity (extra
      args dropped, missing args become ``nil``).  Set via :func:`as_lambda`.
    """

    __slots__ = ("fn", "arity", "is_lambda")

    def __init__(
        self,
        fn: Callable[..., Any],
        arity: int | None = None,
        is_lambda: bool = False,
    ) -> None:
        self.fn = fn
        self.arity = arity
        self.is_lambda = is_lambda


def apply(c: Any, args: list[Any]) -> Any:
    """Invoke a closure handle with ``args``.

    A ``None`` target is the no-block-given case (see
    :class:`LocalJumpError`): a ``yield`` reached through a nil block
    parameter.  It is reported distinctly from other non-closures (a
    genuine type error) so the two failures don't read alike.

    **Proc-vs-lambda arity (Q10g).**  A *block / proc* adjusts its arguments
    to its declared arity the way Ruby does — extra positional arguments are
    dropped and missing ones become ``nil`` (``None``) — so e.g. a one-param
    block yielded two values (``each_with_index``-style) binds the first and
    ignores the rest instead of raising.  A *lambda* (``is_lambda``) is left
    **strict**: arguments pass through unadjusted, so a genuine mismatch
    surfaces as Python's native ``TypeError`` (the analogue of Ruby's
    ``ArgumentError``).  A variadic block (``arity is None``) is also passed
    through unadjusted.
    """
    if c is None:
        raise LocalJumpError("no block given (yield)")
    if not isinstance(c, Closure):
        raise TypeError("apply on non-closure")
    if c.is_lambda or c.arity is None:
        return c.fn(*args)
    # Block / proc leniency: reshape args to exactly the declared arity.
    n = c.arity
    if len(args) > n:
        adjusted: list[Any] = list(args[:n])
    elif len(args) < n:
        adjusted = list(args) + [None] * (n - len(args))
    else:
        adjusted = list(args)
    return c.fn(*adjusted)


def _positional_arity(fn: Callable[..., Any]) -> int | None:
    """Count ``fn``'s fixed positional parameters, or ``None`` if variadic.

    Returns ``None`` when ``fn`` accepts ``*args`` (a variadic block — Ruby
    never trims arguments for ``|*rest|``) or when the signature cannot be
    introspected, signalling :func:`apply` to leave arguments untouched.
    """
    try:
        params = inspect.signature(fn).parameters.values()
    except (TypeError, ValueError):
        return None
    count = 0
    for p in params:
        if p.kind in (
            inspect.Parameter.POSITIONAL_ONLY,
            inspect.Parameter.POSITIONAL_OR_KEYWORD,
        ):
            count += 1
        elif p.kind == inspect.Parameter.VAR_POSITIONAL:
            return None
    return count


def make_closure(fn: Callable[..., Any], captures: list[Any]) -> Closure:
    """Build a closure that prepends the captured values to each call's
    arguments.

    Records the block's own arity (``fn``'s fixed positional parameters minus
    the captured values) so :func:`apply` can apply proc/block leniency.  The
    closure is **not** a lambda by default; the ``lambda`` builtin marks its
    result strict via :func:`as_lambda`.
    """
    arity = _positional_arity(fn)
    if arity is not None:
        arity = max(0, arity - len(captures))
    return Closure(lambda *args: fn(*captures, *args), arity=arity)


def as_lambda(c: Any) -> Any:
    """Mark a closure as a Ruby **lambda** (strict arity); return it.

    The ``lambda`` / ``->(){}`` builtin lowers to a `MakeClosure`, which by
    default yields a proc-lenient closure.  Wrapping that result here flips it
    to strict so :func:`apply` does not silently drop/pad its arguments — Ruby
    lambdas raise on an arity mismatch.  A non-closure passes through unchanged
    (defensive; the emitter only wraps a `MakeClosure`).
    """
    if isinstance(c, Closure):
        c.is_lambda = True
    return c


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


def _write_puts_one(v: Any, out: Any, seen: set[int]) -> None:
    """Emit a single ``per_value`` argument under ``__sys_write__`` (SIR28 §2.1),
    honouring Ruby ``puts``'s per-value rules.

    Ruby ``puts`` is deceptively subtle.  For one argument it behaves as:

    - **Array** → recurse over the *elements*, one per line.  Nesting is
      flattened: ``puts [1, [2, 3]]`` prints ``1\\n2\\n3\\n``.  An **empty**
      array prints nothing here (the caller's top-level ``puts []`` still
      emits a single newline — see :func:`sir_write` — because ``puts`` with
      an empty argument list of its own writes one newline).
    - **anything else** → its display string, then a newline.

    Truth table (single arg → stdout):

    ============  ================
    argument      bytes written
    ============  ================
    ``"x"``       ``x\\n``
    ``[]``        (nothing)
    ``[1, 2]``    ``1\\n2\\n``
    ``[1, [2]]``  ``1\\n2\\n``
    ============  ================

    **Cycle safety.**  A list is a shared, mutable reference, so a program can
    build a *cyclic* array (``a = []; a << a``).  The element-per-line flatten
    recurses through nested arrays, so it MUST be cycle-guarded or a self-
    referential array raises ``RecursionError`` (a DoS: CWE-674, uncontrolled
    recursion).  ``seen`` holds the ``id()`` of the lists currently on the
    active flatten path.  A list ALREADY on the path is a cycle: rather than
    recurse forever we write the ``[...]`` placeholder then a newline, matching
    real Ruby (``puts a`` on a self-referential array prints ``[...]`` and
    terminates).  (We emit the literal placeholder rather than ``str(v)``:
    Python's cycle-safe ``repr`` would render the *containing* level too, e.g.
    ``[[...]]`` for ``a = [a]``, whereas Ruby prints a bare ``[...]``.)  A list
    removed from ``seen`` on exit still flattens in full via a sibling path —
    only a true self-cycle is short-circuited, so non-cyclic output is
    unchanged (``puts [1, [2, 3]]`` still prints ``1\\n2\\n3\\n``).
    """
    if isinstance(v, list):
        vid = id(v)
        if vid in seen:
            print("[...]", file=out)
            return None
        seen.add(vid)
        for item in v:
            _write_puts_one(item, out, seen)
        seen.discard(vid)
        return None
    print(values.to_display(v), file=out)
    return None


def sir_write(stream: str, terminator: str, unpack_arrays: bool, *values_: Any) -> None:
    """SIR28 §2.1: ``__sys_write__``, the general console-output primitive
    every frontend lowers ``print``/``puts``/``console.log``/etc. to.

    It generalizes what used to be several backend-hardcoded newline
    policies into ONE operation parameterized by policy flags carried as
    DATA -- the root cause SIR28 exists to fix: real Ruby's ``print``
    never newline-terminates, Python's own ``print()`` always does, but
    before SIR28 both lowered to the identical ``BuiltinCall("print", ...)``
    this backend had no way to tell apart.

    ``stream``: ``"stdout"`` | ``"stderr"``. ``terminator``: ``"none"``
    (write each value back to back, no newline -- matches Ruby's ``print``)
    | ``"per_value"`` (one newline per value, honouring ``unpack_arrays``
    -- matches Ruby's ``puts``) | ``"once"`` (Python's native
    ``print(a, b)`` -- space-join every value, one trailing newline).
    Deliberately does NOT replicate Ruby ``puts``'s trailing-newline-
    suppression nuance (``puts "x\\n"`` prints ``x\\n``, not ``x\\n\\n``)
    -- that's a pre-existing, orthogonal divergence between backends' own
    historical ``puts`` implementations that SIR28 does not fix or
    replicate; ``per_value`` here always appends exactly one newline per
    value, matching SIR28 §2.1's table and every other backend's
    ``__sys_write__`` faithfully. (Parameter named ``values_`` with a
    trailing underscore to avoid shadowing the ``values`` module imported
    at the top of this file.)
    """
    out = sys.stderr if stream == "stderr" else sys.stdout
    if terminator == "per_value":
        if not values_:
            print(file=out)
            return None
        seen: set[int] = set()
        for v in values_:
            if unpack_arrays:
                _write_puts_one(v, out, seen)
            else:
                print(values.to_display(v), file=out)
        return None
    if terminator == "once":
        print(" ".join(values.to_display(v) for v in values_), file=out)
        return None
    # "none"
    for v in values_:
        print(values.to_display(v), end="", file=out)
    return None


# --- Builtin dispatch ------------------------------------------------------

_builtins: dict[str, Callable[..., Any]] = {
    "+": arithmetic.add,
    "<<": arithmetic.shift_left,
    "-": arithmetic.sub,
    "*": arithmetic.mul,
    "/": arithmetic.div,
    # SIR21 T3b-2: `div_floor` is a bare alias for `div` (Ruby's `/` already
    # floors ints / true-divides floats — see `arithmetic.div`'s own
    # docstring). `div_trunc`/`udiv_trunc`/`div_true` are genuinely new —
    # see `arithmetic.trunc_div`/`utrunc_div`/`true_div`.
    "div_floor": arithmetic.div,
    "div_trunc": arithmetic.trunc_div,
    "udiv_trunc": arithmetic.utrunc_div,
    "div_true": arithmetic.true_div,
    "=": values.eq,
    # `==` is a synonym for `=`; `!=`/`<=`/`>=` complete the comparison family
    # the Ruby frontend lowers a comparison chain to.  Present here (not only in
    # the emitter's direct-call map) so a first-class `:==` symbol reference
    # dispatches too, and so the "known builtins" error message lists them.
    "==": values.eq,
    "!=": values.ne,
    "<": arithmetic.lt,
    ">": arithmetic.gt,
    "<=": arithmetic.le,
    ">=": arithmetic.ge,
    "cons": pairs.cons,
    "car": pairs.car,
    "cdr": pairs.cdr,
    "null?": values.is_null,
    "pair?": pairs.is_pair,
    "number?": values.is_number,
    "symbol?": values.is_symbol,
}


def call_builtin(name: str, args: list[Any]) -> Any:
    """Invoke a builtin by SIR name with a list of arguments.

    The SIR backends translate most builtins to native code or route them to a
    dedicated per-concern runtime package, so this generic dispatch only fires
    for the small set the core registers (arithmetic / pairs / type
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
