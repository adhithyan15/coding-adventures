"""Patterns of ``match`` on a ``Result`` (``ir.tuple``, VER04r Appendix A.1 and section 16.10 [D35]).

The six patterns
----------------
============================  ======================================================
``["ok", pat]``               an ``Ok`` whose payload matches ``pat``
``["err", "CASE"]``           an ``Err`` of exactly that error case
``["err_any"]``               an ``Err`` of any case
``["tuple", [pat, ..]]``      a tuple of the same arity, component by component
``["bind", "name"]``          anything; binds ``name`` to the matched value
``["wild"]``                  anything
============================  ======================================================

The arms are tried in order and **exactly one body is evaluated**: the arm of the first pattern
that matches (section 4.2 rule 2).  Values are the evaluator's own: a ``Result`` is
``("ok", payload)`` or ``("err", case)`` and a tuple is a Python tuple.

Three author refinements (section 16.10 R2, Appendix A.1) are corpus errors (oracle code
``corpus``, generator code ``ir``) and are decided here, from the patterns and the scrutinee's
static type alone:

* **fall-through**: the arms are not exhaustive over ``ok``/``err`` cases;
* **an arm no value can reach**: every value it matches is matched by an earlier arm;
* **a bind name twice in one pattern**.

(The fourth, a ``bind`` that shadows a binder, a parameter, ``result`` or an outer ``bind``,
needs the scope and is checked by the two callers, the lowerer and the typed IR compiler.)

Exhaustiveness and reachability: the usual usefulness check
-----------------------------------------------------------
The classic algorithm (Maranget, "Warnings for pattern matching"): a pattern vector ``q`` is
**useful** with respect to a matrix of rows ``P`` when some value matches ``q`` and no row of ``P``.
Then

* an arm is **unreachable** exactly when its pattern is not useful with respect to the arms before
  it, and
* the match **falls through** exactly when the all-wild vector is useful with respect to all arms.

Constructors are those of the *type*: ``Result<X,E>`` has ``ok`` (one field of type ``X``) and
``err`` (one field of the error enum ``E``, whose constructors are its cases, with no fields); a
tuple has one constructor with one field per component; every other type has none (only a wild or
bind can match it).  A pattern is a *wildcard* (``None`` below) or a constructor with sub-patterns.
When the type's error enum is unknown (the type of a bare ``ok``/``err`` node, ``Result<_,?>``) its
cases are an open set, so a case pattern never completes it.

The check is exponential in the worst case, so it carries a work budget and refuses (``limit``)
beyond it, like the Rust checker's.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Mapping

from . import ast as A
from .errors import OracleError

#: ``MAX_PATTERN_DEPTH`` of the parser (pattern nesting).
MAX_PATTERN_DEPTH = 8
MAX_ARMS = 256
MAX_TUPLE_ARITY = 8
#: Steps the usefulness check may take before it gives up with a ``limit`` error.
WORK_BUDGET = 200_000
_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]{0,63}$")
#: The pseudo type of the field of ``err``: the error enum of the ``Result``.
_ERR = "$err"
_UNKNOWN = "?"


def _fail(code: str, message: str) -> OracleError:
    return OracleError(code, message)


# -- typing an IR pattern ----------------------------------------------------------------------


def type_pattern(
    pattern: object,
    ty: A.Ty,
    errors: Mapping[str, tuple],
    binds: list,
    *,
    code: str = "ir",
    depth: int = 1,
) -> None:
    """Check the IR ``pattern`` against the static type ``ty``; append ``(name, type)`` to ``binds``.

    ``errors`` maps an error type name to its cases (the error type of ``ty`` is looked up in it;
    an error type that is not there, or ``?``, is an open set).  A malformed pattern, a pattern of
    the wrong shape for the type and an unknown error case are errors of ``code``.
    """
    if depth > MAX_PATTERN_DEPTH:
        raise _fail("limit", "pattern nesting exceeds 8 levels")
    if not (isinstance(pattern, list) and pattern and isinstance(pattern[0], str)):
        raise _fail(code, "a malformed pattern")
    tag = pattern[0]
    if tag == "wild" and len(pattern) == 1:
        return
    if tag == "err_any" and len(pattern) == 1:
        _need_result(ty, code)
        return
    if tag == "bind" and len(pattern) == 2:
        name = pattern[1]
        if not isinstance(name, str) or name == "_" or not _NAME.match(name):
            raise _fail(code, "a malformed bind name")
        binds.append((name, ty))
        return
    if tag == "ok" and len(pattern) == 2:
        _need_result(ty, code)
        type_pattern(pattern[1], ty.args[0], errors, binds, code=code, depth=depth + 1)
        return
    if tag == "err" and len(pattern) == 2:
        _need_result(ty, code)
        case = pattern[1]
        if not isinstance(case, str) or not case:
            raise _fail(code, "a malformed error case")
        cases = errors.get(ty.args[1].head)
        if cases is not None and case not in cases:
            raise _fail(code, "an error case the Result's error type does not have")
        return
    if tag == "tuple" and len(pattern) == 2 and isinstance(pattern[1], list):
        parts = pattern[1]
        if not MAX_TUPLE_ARITY >= len(parts) >= 2:
            raise _fail(code, "a tuple pattern has 2 to 8 components")
        if ty.head != "()" or len(ty.args) != len(parts):
            raise _fail(code, "a tuple pattern does not match the type of its value")
        for part, part_ty in zip(parts, ty.args):
            type_pattern(part, part_ty, errors, binds, code=code, depth=depth + 1)
        return
    raise _fail(code, "an unknown pattern")


def _need_result(ty: A.Ty, code: str) -> None:
    if ty.head != "Result":
        raise _fail(code, "an ok/err pattern needs a Result")


# -- exhaustiveness and reachability -----------------------------------------------------------


def _normalise(pattern: list):
    """The abstract pattern: ``None`` (wildcard) or ``(constructor, [sub-patterns])``."""
    tag = pattern[0]
    if tag in ("wild", "bind"):
        return None
    if tag == "ok":
        return ("ok", [_normalise(pattern[1])])
    if tag == "err":
        return ("err", [(pattern[1], [])])
    if tag == "err_any":
        return ("err", [None])
    return ("tuple", [_normalise(p) for p in pattern[1]])


def _signature(ty: A.Ty, errors: Mapping[str, tuple]) -> dict | None:
    """The constructors of ``ty`` with their field types, or ``None`` when the set is open."""
    if ty.head == "Result":
        return {"ok": [ty.args[0]], "err": [A.Ty(_ERR, (ty.args[1],))]}
    if ty.head == "()":
        return {"tuple": list(ty.args)}
    if ty.head == _ERR:
        cases = errors.get(ty.args[0].head)
        return None if cases is None else {c: [] for c in cases}
    return None


class _Budget:
    def __init__(self) -> None:
        self.left = WORK_BUDGET

    def spend(self, n: int) -> None:
        self.left -= n
        if self.left < 0:
            raise _fail("limit", "the exhaustiveness check exceeds its work budget")


def _specialise(rows: list, ctor: object, arity: int) -> list:
    out = []
    for row in rows:
        head = row[0]
        if head is None:
            out.append([None] * arity + row[1:])
        elif head[0] == ctor:
            out.append(list(head[1]) + row[1:])
    return out


def _useful(rows: list, vec: list, tys: list, errors: Mapping[str, tuple], budget: _Budget) -> bool:
    """Is the pattern vector ``vec`` useful with respect to ``rows`` (all of types ``tys``)?"""
    budget.spend(1 + len(rows))
    if not vec:
        return not rows
    head = vec[0]
    if head is not None:
        ctor, args = head
        sig = _signature(tys[0], errors)
        fields = sig[ctor] if sig is not None and ctor in sig else [A.Ty(_UNKNOWN)] * len(args)
        return _useful(
            _specialise(rows, ctor, len(args)), list(args) + vec[1:], fields + tys[1:], errors, budget
        )
    sig = _signature(tys[0], errors)
    present = {r[0][0] for r in rows if r[0] is not None}
    if sig is not None and set(sig) <= present:
        return any(
            _useful(
                _specialise(rows, ctor, len(fields)),
                [None] * len(fields) + vec[1:],
                fields + tys[1:],
                errors,
                budget,
            )
            for ctor, fields in sig.items()
        )
    default = [r[1:] for r in rows if r[0] is None]
    return _useful(default, vec[1:], tys[1:], errors, budget)


def check_arms(
    patterns: list, ty: A.Ty, errors: Mapping[str, tuple], *, code: str = "corpus"
) -> None:
    """Refuse a match that can fall through or has an arm no value can reach (errors of ``code``).

    ``patterns`` are the IR patterns of the arms in order, already typed against ``ty``.  Unreachable
    arms are found first, in arm order, then fall-through.
    """
    if not 1 <= len(patterns) <= MAX_ARMS:
        raise _fail("limit", "a match has 1 to 256 arms")
    budget = _Budget()
    rows: list = []
    for pattern in patterns:
        abstract = _normalise(pattern)
        if not _useful(rows, [abstract], [ty], errors, budget):
            raise _fail(code, "a match arm that no value can reach")
        rows.append([abstract])
    if _useful(rows, [None], [ty], errors, budget):
        raise _fail(code, "a match that can fall through")


# -- matching at run time ------------------------------------------------------------------------


def compile_pattern(pattern: list) -> Callable[[object, dict], bool]:
    """A closure ``match(value, binds) -> bool`` for a typed IR pattern; it fills ``binds``."""
    tag = pattern[0]
    if tag == "wild":
        return lambda value, binds: True
    if tag == "bind":
        name = pattern[1]

        def bind(value: object, binds: dict) -> bool:
            binds[name] = value
            return True

        return bind
    if tag == "ok":
        inner = compile_pattern(pattern[1])
        return lambda value, binds: value[0] == "ok" and inner(value[1], binds)
    if tag == "err_any":
        return lambda value, binds: value[0] == "err"
    if tag == "err":
        case = pattern[1]
        return lambda value, binds: value[0] == "err" and value[1] == case
    parts = [compile_pattern(p) for p in pattern[1]]

    def match_tuple(value: object, binds: dict) -> bool:
        return all(part(item, binds) for part, item in zip(parts, value))

    return match_tuple
