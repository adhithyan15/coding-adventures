"""``match`` patterns on a ``Result`` (VER04r Appendix A.1, the ``ir.tuple`` level).

A ``match`` is ``["match", scrutinee, [[pattern, arm], ..]]``.  At ``ir.tuple``
the scrutinee is a ``Result<X,E>`` and a pattern is one of::

    ["ok", p]            an Ok whose payload matches p
    ["err", "CASE"]      an Err of exactly that case of E
    ["err_any"]          an Err of any case
    ["tuple", [p1..pn]]  a tuple of n components, each matching its pattern
    ["bind", "name"]     anything; the arm may use ``name`` for the value
    ["wild"]             anything

(``some none ctor bool`` are the *data* patterns of ``ir.data``; a lane that
lacks that level refuses a ``match`` that holds one as ``level-missing``,
section 16.18 R76.)

Three jobs live here, and all three are about the *patterns*, never about the
arm bodies:

1. **typing**: a pattern is read against the static type of the value it tests,
   so ``["tuple", [a, b]]`` on a ``U8`` payload is ``ill-typed``, as is an
   ``["err", "CASE"]`` that the error type does not declare;
2. **running**: a pattern tests a value and, on success, records what each
   ``bind`` captured;
3. **usefulness**: Appendix A.1 refuses a ``match`` that can fall through (not
   exhaustive) and an arm that no value can reach (every value it matches is
   matched earlier).  Both are decided with the textbook *usefulness* check
   (Maranget): is there a value the arm accepts that no earlier arm accepts?
   Is there a value the whole column rejects?

The usefulness check sees a ``Result`` as a sum with one constructor ``ok``
(one field, the payload) and one constructor per case of ``E`` (no field), a
tuple as a product with one constructor, and every other type as having
constructors that a pattern cannot name (only ``bind`` and ``wild`` match it,
so for them the check just drops the column).  An ``["err_any"]`` is every
``err`` constructor at once.
"""

from __future__ import annotations

import re
from collections.abc import Mapping
from dataclasses import dataclass

from .errors import Refusal
from .wtypes import UNKNOWN, Ty

DATA_PATTERNS = frozenset({"some", "none", "ctor", "bool"})
"""The tags of the ``ir.data`` patterns (Appendix A.2); refused without it."""
PATTERN_TAGS = frozenset({"ok", "err", "err_any", "tuple", "bind", "wild"})

_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z")
MAX_ARMS = 1024
"""A lane's own bound on the arms of one ``match`` (the spec states none)."""


def _ill_typed(why: str) -> Refusal:
    return Refusal("ill-typed", why)


@dataclass(frozen=True)
class Pattern:
    """One typed pattern.

    ``kind`` is ``ok``, ``err``, ``err_any``, ``tuple``, ``bind`` or ``wild``;
    ``subs`` the sub-patterns (one for ``ok``, ``n`` for a tuple); ``name`` the
    captured name of a ``bind`` or the case of an ``err``.
    """

    kind: str
    subs: tuple[Pattern, ...] = ()
    name: str = ""

    @property
    def is_wild(self) -> bool:
        return self.kind in ("wild", "bind")

    def test(self, value: object, binds: dict) -> bool:
        """Does ``value`` match?  On success ``binds`` holds what each bind caught."""
        kind = self.kind
        if kind == "wild":
            return True
        if kind == "bind":
            binds[self.name] = value
            return True
        if kind == "ok":
            return value[0] == "ok" and self.subs[0].test(value[1], binds)  # type: ignore[index]
        if kind == "err":
            return value[0] == "err" and value[1] == self.name  # type: ignore[index]
        if kind == "err_any":
            return value[0] == "err"  # type: ignore[index]
        return all(
            sub.test(item, binds)
            for sub, item in zip(self.subs, value, strict=True)  # type: ignore[arg-type, call-overload]
        )


WILD = Pattern("wild")


def holds_data_pattern(raw: object) -> bool:
    """Is any pattern at any depth of ``raw`` a data pattern?  (Iterative.)"""
    pending = [raw]
    while pending:
        item = pending.pop()
        if type(item) is not list or not item or type(item[0]) is not str:
            continue
        if item[0] in DATA_PATTERNS:
            return True
        if item[0] == "ok" and len(item) == 2:
            pending.append(item[1])
        elif item[0] == "tuple" and len(item) == 2 and type(item[1]) is list:
            pending.extend(item[1])
    return False


def parse(
    raw: object,
    ty: Ty,
    errors: Mapping[str, tuple[str, ...]],
    taken: set[str],
    bound: dict[str, Ty],
) -> Pattern:
    """Read ``raw`` as a pattern for a value of type ``ty``, or refuse ``ill-typed``.

    ``taken`` is every name already in scope (binders, parameters, ``result``,
    an outer ``bind``): a ``bind`` may not reuse one, because the IR has no
    shadowing.  ``bound`` collects the names this pattern binds, with their
    types; the same name twice in one pattern is refused too.
    """
    if type(raw) is not list or not raw or type(raw[0]) is not str:
        raise _ill_typed("a pattern is a list that starts with its tag")
    tag = raw[0]
    if tag not in PATTERN_TAGS:
        raise _ill_typed("not a pattern of this level")
    if tag == "wild":
        if len(raw) != 1:
            raise _ill_typed("wild takes no operand")
        return WILD
    if tag == "bind":
        if len(raw) != 2 or type(raw[1]) is not str or not _NAME.match(raw[1]):
            raise _ill_typed("bind takes one name")
        name = raw[1]
        if name in taken or name in bound:
            raise _ill_typed("a bind may not repeat or shadow a name")
        if _undetermined(ty):
            raise _ill_typed("a bind of a value of an undetermined type")
        bound[name] = ty
        return Pattern("bind", name=name)
    if tag == "tuple":
        if len(raw) != 2 or type(raw[1]) is not list:
            raise _ill_typed("tuple takes a list of patterns")
        if ty.head != "()" or len(raw[1]) != len(ty.args):
            raise _ill_typed("a tuple pattern needs a tuple of the same arity")
        subs = tuple(
            parse(item, part, errors, taken, bound)
            for item, part in zip(raw[1], ty.args, strict=True)
        )
        return Pattern("tuple", subs)
    # ok, err, err_any: patterns of a Result
    if ty.head != "Result":
        raise _ill_typed(f"{tag} is a pattern of a Result")
    if tag == "err_any":
        if len(raw) != 1:
            raise _ill_typed("err_any takes no operand")
        return Pattern("err_any")
    if tag == "err":
        cases = errors.get(ty.args[1].head)
        if len(raw) != 2 or type(raw[1]) is not str or cases is None:
            raise _ill_typed("err takes the name of a case of a known error type")
        if raw[1] not in cases:
            raise _ill_typed("the error type declares no such case")
        return Pattern("err", name=raw[1])
    if len(raw) != 2:
        raise _ill_typed("ok takes one pattern")
    if _undetermined(ty.args[0]):
        raise _ill_typed("an ok pattern on a value of an undetermined type")
    return Pattern("ok", (parse(raw[1], ty.args[0], errors, taken, bound),))


def _undetermined(ty: Ty) -> bool:
    return ty.head == UNKNOWN or any(
        isinstance(arg, Ty) and _undetermined(arg) for arg in ty.args
    )


# ---------------------------------------------------------------------------
# Usefulness
# ---------------------------------------------------------------------------
Ctor = tuple  # ("ok",) | ("err", case) | ("tuple", arity)
Row = list  # of Pattern


def _signature(ty: Ty, errors: Mapping[str, tuple[str, ...]]) -> list[Ctor] | None:
    """Every constructor of ``ty`` a pattern can name, or ``None`` if it has none."""
    if ty.head == "Result":
        cases = errors.get(ty.args[1].head, ())
        return [("ok",), *(("err", case) for case in cases)]
    if ty.head == "()":
        return [("tuple", len(ty.args))]
    return None


def _field_types(ctor: Ty | Ctor, ty: Ty) -> list[Ty]:
    """The types of the fields of constructor ``ctor`` of ``ty``."""
    if ctor[0] == "ok":  # type: ignore[index]
        return [ty.args[0]]
    if ctor[0] == "tuple":  # type: ignore[index]
        return list(ty.args)
    return []


def _constructor(pattern: Pattern) -> Ctor | None:
    if pattern.kind == "ok":
        return ("ok",)
    if pattern.kind == "err":
        return ("err", pattern.name)
    if pattern.kind == "tuple":
        return ("tuple", len(pattern.subs))
    return None


def _specialize(rows: list[Row], ctor: Ctor, arity: int) -> list[Row]:
    """The rows that can match a value built with ``ctor``, its fields opened."""
    out: list[Row] = []
    for row in rows:
        head, rest = row[0], row[1:]
        if head.is_wild:
            out.append([WILD] * arity + rest)
        elif head.kind == "err_any":
            if ctor[0] == "err":
                out.append(rest)
        elif _constructor(head) == ctor:
            out.append(list(head.subs) + rest)
    return out


def _used(rows: list[Row], cases: list[str]) -> set[Ctor]:
    """The constructors the first column of ``rows`` names (``err_any``: every case)."""
    used: set[Ctor] = set()
    for row in rows:
        head = row[0]
        if head.kind == "err_any":
            used.update(("err", case) for case in cases)
        else:
            ctor = _constructor(head)
            if ctor is not None:
                used.add(ctor)
    return used


def useful(
    rows: list[Row],
    query: Row,
    tys: list[Ty],
    errors: Mapping[str, tuple[str, ...]],
) -> bool:
    """Does some value match ``query`` and no row of ``rows``?

    With ``query`` all wildcards this asks whether ``rows`` leave a value
    uncovered (the ``match`` is not exhaustive); with ``query`` one arm's
    pattern and ``rows`` the arms above it, it asks whether that arm can be
    reached.  The recursion is over the *types* (at most 16 deep) and the
    columns, so it is bounded by the size of the patterns.
    """
    if not tys:
        return not rows
    head, ty = query[0], tys[0]
    signature = _signature(ty, errors)
    cases = list(errors.get(ty.args[1].head, ())) if ty.head == "Result" else []
    if head.kind == "err_any":
        return any(
            useful(_specialize(rows, ("err", case), 0), query[1:], tys[1:], errors)
            for case in cases
        )
    if not head.is_wild:
        ctor = _constructor(head)
        assert ctor is not None
        fields = _field_types(ctor, ty)
        return useful(
            _specialize(rows, ctor, len(fields)),
            list(head.subs) + query[1:],
            fields + tys[1:],
            errors,
        )
    if signature is not None and set(signature) <= _used(rows, cases):
        for ctor in signature:
            fields = _field_types(ctor, ty)
            if useful(
                _specialize(rows, ctor, len(fields)),
                [WILD] * len(fields) + query[1:],
                fields + tys[1:],
                errors,
            ):
                return True
        return False
    defaults = [row[1:] for row in rows if row[0].is_wild]
    return useful(defaults, query[1:], tys[1:], errors)


def check_arms(
    arms: list[Pattern], scrutinee: Ty, errors: Mapping[str, tuple[str, ...]]
) -> None:
    """Refuse an arm no value reaches, then a ``match`` that can fall through.

    Appendix A.1 names both as corpus errors (section 16.10 R2, R12: the
    unreachable arm is reported first).  The kit's closed code set for a ``v2-ir``
    tree has no ``corpus``, so this lane reports them as ``ill-typed``.
    """
    for place, arm in enumerate(arms):
        above = [[earlier] for earlier in arms[:place]]
        if not useful(above, [arm], [scrutinee], errors):
            raise _ill_typed("a match arm that no value can reach")
    if useful([[arm] for arm in arms], [WILD], [scrutinee], errors):
        raise _ill_typed("a match that can fall through")
