"""Exception runtime primitives for Semantic-IR-emitted Python.

Most SIR constructs translate to *native* Python (a sequence is a ``list``, a
loop is a ``for``).  Exception handling translates *mostly* natively —
``begin/rescue/ensure`` becomes a native ``try/except/finally`` — but two pieces
have no faithful native equivalent and live here:

    1. **A SIR exception object.**  Ruby's ``raise StandardError, "boom"`` names
       a *class* and carries a message.  Python's ``raise`` takes an exception
       *instance*, and a plain ``Exception`` carries no Ruby class tag.
       :class:`SirError` is the raised object: a real ``Exception`` (so
       tracebacks work) that also records the SIR class name in ``sir_class``.

    2. **Rescue-clause type matching.**  A native ``except`` clause matches by
       Python class; Ruby's ``rescue TypeError, ArgumentError => e`` matches a
       *set* of Ruby class *names* (and their subclasses) and falls through to
       the next clause otherwise.  :func:`rescue_matches` answers "does this
       caught value match this clause's class list?" so the emitted ``except``
       body can dispatch to the right clause (or re-``raise`` if none match).

**Keyed to SIR, not Ruby.**  These helpers implement the SIR exception model, so
a future JavaScript->SIR->Python path reuses them unchanged.  See
``code/specs/sir-runtime.md``.

**User-class ancestry (E2).**  The built-in table below is fixed, but SIR *does*
carry ``class MyErr < StandardError`` edges in ``Stmt::ClassDef``.  The backend
threads them here with :func:`register_ancestry` at program init, so a
``rescue StandardError`` catches a raised ``MyErr`` even though ``MyErr`` is not
in the built-in table.  We keep this an **explicit string→string map** — no
``eval``/reflection, no walking real Python classes — because the SIR class
names are just tags, not live Python types.  User edges are *additive*: they
extend the chain up to a built-in root (``StandardError → Exception``) and never
mutate the built-in entries, so built-in matching is unchanged.  A user class
with no registered superclass still matches only by exact name (or via
``Exception`` / a bare ``rescue``), exactly as before.
"""

from __future__ import annotations

from typing import Any, NoReturn

# The SIR universal value type at this package's boundary.
Val = Any

# ── Built-in Ruby exception ancestry ─────────────────────────────────────────

# Maps a subclass name to its immediate superclass name.  Walked by
# :func:`_is_ancestor_or_self` so a ``rescue StandardError`` also catches the
# everyday subclasses a program raises.  This is intentionally a small, curated
# slice of Ruby's tree (the classes the frontend is likely to name), not the
# whole standard library.  Every entry ultimately chains up to
# ``StandardError -> Exception``::
#
#     Exception
#     └─ StandardError
#        ├─ RuntimeError ├─ ArgumentError ├─ TypeError
#        ├─ NameError ─ NoMethodError      ├─ RangeError
#        ├─ IndexError ─ KeyError          ├─ ZeroDivisionError
#        ├─ IOError    ├─ StopIteration    └─ NotImplementedError
_BUILTIN_ANCESTRY: dict[str, str] = {
    "RuntimeError": "StandardError",
    "ArgumentError": "StandardError",
    "TypeError": "StandardError",
    "NameError": "StandardError",
    "NoMethodError": "NameError",
    "IndexError": "StandardError",
    "KeyError": "IndexError",
    "RangeError": "StandardError",
    "ZeroDivisionError": "StandardError",
    "IOError": "StandardError",
    "StopIteration": "StandardError",
    "NotImplementedError": "StandardError",
    "StandardError": "Exception",
}

# The *live* ancestry the matcher walks.  Seeded from the built-in table and
# then extended in place by :func:`register_ancestry` with user ``child ->
# superclass`` edges.  We start from a copy so a caller can never mutate the
# frozen built-in reference, and so tests can restore a pristine state by
# re-seeding.  A *user* edge that names a built-in child (e.g. redefining
# ``RuntimeError``) is honoured — last writer wins — but the emitter never does
# that; it only registers genuinely new class names.
_ANCESTRY: dict[str, str] = dict(_BUILTIN_ANCESTRY)


def register_ancestry(mapping: dict[str, str]) -> None:
    """Merge user ``{childClassName: superclassName}`` edges into the ancestry.

    Called once at program init with the module's ``class Child < Parent``
    pairs, *before* any ``rescue`` runs.  After this, :func:`rescue_matches`
    walks a user child up through its registered superclass and on into the
    built-in table — so ``rescue StandardError`` catches a raised
    ``MyErr < StandardError``.

    The mapping is an **explicit string→string map**: keys and values are SIR
    class-name tags, not Python classes.  We deliberately do no reflection and
    trust no live type — the frontend already knows the static superclass edge,
    so threading it as data keeps the runtime free of ``eval``/import magic.

    Idempotent and additive: re-registering the same edge is a no-op, and user
    edges layer on top of the built-in table without replacing it (a chain like
    ``Grandchild -> Child -> StandardError -> Exception`` resolves by walking
    both layers).  :func:`_is_ancestor_or_self` already guards against cycles,
    so a malformed self-referential edge cannot loop forever.
    """
    _ANCESTRY.update(mapping)


# ── The SIR exception object ─────────────────────────────────────────────────


class SirError(Exception):
    """A SIR exception: a native ``Exception`` tagged with its Ruby class name.

    ``sir_class`` is what :func:`rescue_matches` dispatches on; ``args[0]`` (the
    standard ``Exception`` message) is the human string Ruby's ``raise Klass,
    "msg"`` carries.  When no message is given the class name itself is used
    (matching Ruby's default ``exception.message``).
    """

    __slots__ = ("sir_class",)

    def __init__(self, sir_class: str, message: Val = None) -> None:
        text = sir_class if message is None else str(message)
        super().__init__(text)
        self.sir_class: str = sir_class


def raise_error(class_name: str = "RuntimeError", message: Val = None) -> NoReturn:
    """Raise a :class:`SirError` of ``class_name`` with an optional ``message``.

    Emitted for SIR ``BuiltinCall("raise", …)``:
        - ``raise Foo, "msg"`` -> ``raise_error("Foo", "msg")``
        - ``raise Foo``        -> ``raise_error("Foo")``
        - bare ``raise``       -> ``raise_error()`` -> re-raises as a generic
          ``RuntimeError`` (SIR v0 does not thread the in-flight exception into
          a bare re-raise; documented limitation).

    Declared ``NoReturn`` so type checkers know code after a ``raise`` is dead.
    """
    raise SirError(class_name, message)


def class_of_thrown(exc: object) -> str:
    """The SIR class name of a caught value.

    A :class:`SirError` reports its tag; any other exception (a native Python
    error) is treated as a ``StandardError`` so ``rescue StandardError`` /
    ``rescue => e`` catches Python runtime errors too.
    """
    if isinstance(exc, SirError):
        return exc.sir_class
    return "StandardError"


def ancestry_chain(class_name: str) -> list[str]:
    """``class_name`` followed by each of its registered ancestors, in order.

    ``ancestry_chain("ArgumentError")`` is
    ``["ArgumentError", "StandardError", "Exception"]``.

    The ancestry table is private to this module, but a *caller* sometimes has
    to visit each link rather than ask a yes/no question about the whole chain:
    the OOP runtime's ``is_a?`` must check every ancestor for an ``include``d
    module, which :func:`rescue_matches` (a pure name walk) cannot answer.
    Exposing the chain keeps :data:`_ANCESTRY` itself private and read-only to
    the outside.

    Cycle-safe: a malformed registration (``A → B → A``) terminates rather
    than looping forever, and each class appears at most once.
    """
    chain: list[str] = []
    cur: str | None = class_name
    seen: set[str] = set()
    while cur is not None and cur not in seen:
        seen.add(cur)
        chain.append(cur)
        cur = _ANCESTRY.get(cur)
    return chain


def _is_ancestor_or_self(actual: str, target: str) -> bool:
    """``True`` if ``actual`` is ``target`` or any of its registered ancestors."""
    cur: str | None = actual
    seen: set[str] = set()
    while cur is not None and cur not in seen:
        if cur == target:
            return True
        seen.add(cur)
        cur = _ANCESTRY.get(cur)
    return False


def rescue_matches(exc: object, class_names: list[str]) -> bool:
    """Does a caught value match a ``rescue`` clause that names ``class_names``?

    - An **empty** ``class_names`` is a bare ``rescue`` (catch-all) -> always
      ``True``.
    - ``Exception`` is Ruby's universal exception root -> matches anything.
    - Otherwise the value matches if its class equals, or descends from, any
      named class (per the built-in :data:`_ANCESTRY`; user classes match by
      exact name).

    The emitted ``except`` block calls this once per rescue clause, in source
    order, running the first matching clause's body and re-``raise``-ing if none
    match.
    """
    if not class_names:
        return True
    actual = class_of_thrown(exc)
    return any(
        name == "Exception" or _is_ancestor_or_self(actual, name) for name in class_names
    )
