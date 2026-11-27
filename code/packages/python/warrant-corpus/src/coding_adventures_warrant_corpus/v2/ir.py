"""The ``ir.core``, ``ir.fixed`` and ``ir.tuple`` evaluator (VER04r section 4, App. A).

Every claim body, ``admit`` refinement, model and fault expression in a v2
corpus is a small expression tree such as::

    ["eq", ["subject_op", "bump", [["var", "a"]]],
          ["model_call", "inc", [["var", "a"]]]]

meaning "the real export ``bump`` agrees with the pure model ``inc``".  This
module checks such a tree completely **before** the first case runs and then
compiles it into nested Python closures, so evaluating a case costs one call per
node instead of re-interpreting lists of lists.

What "checks" means (everything below is refused with a *kit code*, see
:class:`~.errors.Refusal`; nothing is skipped or guessed)::

    depth          an expression nested more than 64 deep, or more than 64 active
                   model_call frames                                (section 14)
    unknown-tag    a tag of no level this document defines           (section 4.2.7)
    level-missing  a tag, or a type, of a defined level the runner lacks
    subject-forbidden  a subject_op in a model body, or in an admit/fault expression
    model-order    a model_call to a model that is not *earlier* in model_order
    ill-typed      every other static type error
    impure-guard   a subject call under a guard that itself calls the subject
    ir-cost        a worst-path cost above max_ir_cost               (section 4.2.6)

and at evaluation time ``ir-overflow``.

The evaluation rules this file implements, in the order the spec states them:

1. Trees have no recursion, loop or mutable state; model ``m`` may call only
   models declared *before* it (the ``model_order`` array, not the key order).
2. Arguments are evaluated left to right.  ``and``, ``or``, ``implies`` and
   ``if`` **short-circuit**: an untaken operand is never evaluated, so it
   contributes no subject call.
3. ``subject_op`` dispatches to the real export (a *stand-in model* in the
   conformance kit), is forbidden inside every model body and every fault
   expression, and is **counted**: the number of calls is measured, not computed.
4. Arithmetic on a fixed-width type wraps (section 3); on ``Nat``/``Int`` it is
   exact inside a 128-bit window and overflowing it is ``ir-overflow``.
5. Equality is structural and type-directed: errors are nominal, ``Bool`` is not
   ``U8``, ``Nat`` widens to ``Int``.

Slice S10e adds the two levels that depend on ``ir.core`` alone (they are
siblings: neither needs the other):

* ``ir.fixed`` -- the 35 built-ins of Appendix A.1 (``bit_and`` .. ``unwrap_or``):
  bit operations on the ``w``-bit pattern, shifts and rotations whose exact-``Nat``
  count is compared with ``w`` before any host shift, ``widen``/``wrap``/``try``,
  checked arithmetic and the two kinds of division.  The value rules are in
  :mod:`.fixed`; the ``Option`` the nine producers yield is an *intermediate*
  that only ``unwrap_or`` consumes (anywhere else it is ``ir.data``).
* ``ir.tuple`` -- the tuple literal, ``project``, structural ``eq`` and ``match``
  on a ``Result`` (patterns in :mod:`.patterns`).  The level of a ``match`` is
  **type-directed** (R76): the same JSON on a bare tuple, record, variant,
  ``Option`` or ``Bool``, or with a data pattern, is ``ir.data``.

The cost bound is the whole "fuel": the IR has no loop and no recursion, so a
static worst-path count of node evaluations is a sound bound and no run-time
meter exists.  An ``if`` costs its condition plus the *larger* branch (an
evaluator that visits more nodes than that is nonconforming), a ``model_call``
costs its arguments plus its body's cost, and a short circuit is never credited.
"""

from __future__ import annotations

import operator
import sys
from collections.abc import Callable, Collection, Iterator
from dataclasses import dataclass, field

from . import fixed, patterns
from .arith import WindowExceeded, raw_binary
from .errors import IrOverflow, KitError, Refusal
from .semantics import STANDARD, Semantics
from .values import decode_value
from .wtypes import (
    BOOL,
    BYTES,
    INT,
    MAX_FIXED_WIDTH,
    NAT,
    UNKNOWN_TY,
    Ty,
    bind_widths,
    check_types_table,
    fault_unify,
    fixed_range,
    flows,
    forget_widths,
    parse_type,
    unify,
)

# ---------------------------------------------------------------------------
# Limits (section 14)
# ---------------------------------------------------------------------------
MAX_IR_COST = 2_000  # the default per-expression bound
MAX_IR_COST_CEILING = 65_536  # the most a corpus may declare
MAX_IR_VALUE = 65_536  # any Bytes an evaluator produces, checked BEFORE allocating
MAX_DEPTH = 64  # nodes on a path
MAX_FRAMES = 64  # active model_call frames


# ---------------------------------------------------------------------------
# The tags of every level this document defines (Appendix A, A.1, A.2)
# ---------------------------------------------------------------------------
# A runner knows the closed tag set of *every* defined level, implemented or not,
# because it must tell ``level-missing`` (a real tag of a level it lacks) from
# ``unknown-tag`` (a tag of no level) -- section 11, R24.
def _tags(text: str) -> frozenset[str]:
    """The set of whitespace-separated names in ``text``."""
    return frozenset(text.split())


LEVEL_TAGS: dict[str, frozenset[str]] = {
    "ir.core": _tags(
        "lit var subject_op model_call len ok err eq ne lt gt le ge add sub mul "
        "and or not implies if neg concat take drop is_err"
    ),
    "ir.fixed": _tags(
        "bit_and bit_or bit_xor bit_not shl shr sar rotl rotr popcount clz ctz bit "
        "widen wrap try to_int to_nat max_value min_value checked_add checked_sub "
        "checked_mul div rem div_trunc rem_trunc imin imax be_bytes le_bytes "
        "from_be from_le at unwrap_or"
    ),
    # `match` is `ir.tuple` when its scrutinee is a Result and `ir.data` otherwise;
    # a runner without either lacks it whichever, so it is listed once, here.
    "ir.tuple": _tags("tuple project match"),
    "ir.data": _tags("some none ctor field"),
    "ir.list": _tags("list"),
    "ir.seq": _tags("fold map all any range count"),
}
# The seven level names section 4.1 defines.  ``ir.list``, ``ir.seq`` and
# ``ir.inout.records`` are reserved (nothing of them is implemented) but still
# *defined*: a ``requires`` list naming them is a real list, and one naming
# anything else is a corpus error, not a level the runner happens to lack.
DEFINED_LEVELS = frozenset(LEVEL_TAGS) | {"ir.inout.records"}


# The level lattice (section 4.1, R1): ``ir.core`` is the base; ``ir.fixed`` and
# ``ir.tuple`` each depend on it alone and **not on one another**; ``ir.data``
# implies ``ir.tuple`` (and does *not* imply ``ir.fixed``); ``ir.list`` implies
# ``ir.data``.  The implication is a rule on the levels a *lane declares*, which is
# why a lane checks its own set (the generator and the oracle never see one).
LEVEL_NEEDS: dict[str, frozenset[str]] = {
    "ir.fixed": frozenset({"ir.core"}),
    "ir.tuple": frozenset({"ir.core"}),
    "ir.data": frozenset({"ir.core", "ir.tuple"}),
    "ir.list": frozenset({"ir.core", "ir.tuple", "ir.data"}),
}


def check_declared_levels(levels: Collection[str]) -> frozenset[str]:
    """The levels a lane declares, if they respect the lattice, else ``corpus``.

    Every level named in ``LEVEL_NEEDS`` must come with the levels it needs, so a
    lane that declares ``ir.data`` without ``ir.tuple`` (or ``ir.fixed`` without
    ``ir.core``) is refused; ``ir.fixed`` and ``ir.tuple`` do not need each other.
    """
    declared = frozenset(levels)
    for level in declared:
        if level not in DEFINED_LEVELS or level == "ir.inout.records":
            raise Refusal("corpus", f"{level} is not a level a lane may declare")
        if not LEVEL_NEEDS.get(level, frozenset()) <= declared:
            raise Refusal("corpus", f"{level} needs {sorted(LEVEL_NEEDS[level])}")
    return declared


IMPLEMENTED_LEVELS = check_declared_levels({"ir.core", "ir.fixed", "ir.tuple"})
"""The levels this lane implements (VER04r section 4.1, slice S10e).

``ir.fixed`` and ``ir.tuple`` are siblings: each depends on ``ir.core`` alone, so a
case or a corpus may use either without the other.  ``ir.data`` (which implies
``ir.tuple``), ``ir.list`` and ``ir.seq`` are *defined* but not implemented: their
tags are ``level-missing`` here.
"""


def check_requires(requires: object, code: str, what: str) -> list[str]:
    """A ``requires`` list: a list of defined level names, else a refusal ``code``.

    VER04r section 4.2 rule 7: an undefined level name is a corpus error in every
    consumer.  It is never accepted, and it is never ``level-missing`` either --
    that code is for a level this document *defines* and the runner lacks.
    """
    if type(requires) is not list or not all(
        type(level) is str and level in DEFINED_LEVELS for level in requires
    ):
        raise Refusal(code, f"{what} names a level section 4.1 does not define")
    return requires


TAG_LEVEL = {tag: level for level, tags in LEVEL_TAGS.items() for tag in tags}

# The calls-per-case of a node: a constant, or a function of the environment.
Calls = int | Callable[[dict], int]


def tag_level(tag: str) -> str | None:
    """The level a tag belongs to, or ``None`` for a tag of no level."""
    return TAG_LEVEL.get(tag)


def type_level(ty: Ty, types: dict) -> set[str]:
    """The levels a *type* needs: a tuple ``ir.tuple``; option, record, variant more."""
    needs: set[str] = set()
    stack = [ty]
    while stack:
        current = stack.pop()
        if current.head == "()":
            needs.add("ir.tuple")
        elif current.head == "List":
            needs.add("ir.list")
        elif current.head == "Option" or types.get(current.head, {}).get("kind") in (
            "record",
            "variant",
        ):
            needs.add("ir.data")
        stack.extend(a for a in current.args if isinstance(a, Ty))
    return needs


# ---------------------------------------------------------------------------
# Walking a tree without recursion
# ---------------------------------------------------------------------------
def is_node(value: object) -> bool:
    """An IR node is a non-empty list whose first element is the tag text."""
    return type(value) is list and bool(value) and type(value[0]) is str


# The tags whose operands are not simply "everything after the tag": a literal,
# a variable and an error case carry data, not sub-expressions, and a call
# carries a *name*, an array of argument expressions and (``subject_op``) an
# array of numbers.  Reading each position for what it is (F13) means a stray
# string or number in an argument array is a malformed node, never a tag.
_DATA_TAGS = frozenset({"lit", "var", "err"})
_CALL_TAGS = frozenset({"subject_op", "model_call"})
# Tags whose single operand is a *list* of expressions.  A tag-name check that
# only followed `node[1:]` would never see inside it, and an unknown tag there
# would be hidden behind the tag of the construct that holds it (section 4.2
# rule 7: "an unknown tag is never skipped"; R69 (d)).
_LIST_TAGS = frozenset({"tuple", "list"})


def operands(node: list) -> list:
    """The positions of ``node`` that are meant to hold sub-expressions, in order."""
    tag = node[0]
    if tag in _DATA_TAGS:
        return []
    if tag in _CALL_TAGS:
        arguments = node[2] if len(node) > 2 else None
        return arguments if type(arguments) is list else []
    if tag in _LIST_TAGS:  # ["tuple", [e1, .., en]]: the operands are in a list
        listed = node[1] if len(node) > 1 else None
        return listed if type(listed) is list else []
    if tag == "ctor":  # ["ctor", "Name", [e1, .., en]]
        listed = node[2] if len(node) > 2 else None
        return listed if type(listed) is list else []
    if tag == "match":  # ["match", scrutinee, [[pattern, arm], ..]]
        arms = node[2] if len(node) > 2 else None
        bodies = (
            [arm[1] for arm in arms if type(arm) is list and len(arm) == 2]
            if type(arms) is list
            else []
        )
        return node[1:2] + bodies
    return node[1:]


def walk(root: object) -> Iterator[tuple[list, int]]:
    """Yield ``(node, depth)`` for every IR node under ``root``, pre-order, iteratively.

    A node's *depth* is the number of nodes on the path from the root to it, the
    root being 1 (a leaf alone is one level).  The argument array of a call is
    not a node and adds no depth.  Only the positions :func:`operands` names are
    followed, and an item there that is not a node is *skipped* (the compiler
    refuses it as ``ill-typed``).  Iterative on purpose: a hostile corpus may nest
    far deeper than Python's recursion limit and must be *refused*, not crash
    the runner.
    """
    pending: list[tuple[object, int]] = [(root, 1)]
    while pending:
        item, depth = pending.pop()
        if not is_node(item):
            continue
        yield item, depth  # type: ignore[misc]
        pending.extend((child, depth + 1) for child in reversed(operands(item)))  # type: ignore[arg-type]


def scan_tags(
    root: object,
    role: str,
    models: Collection[str] | None = None,
    subjects: bool = True,
) -> None:
    """The checks that read tag names alone, for one top-level expression.

    In the order :meth:`Program.scan` uses: ``depth`` (a path of more than
    :data:`MAX_DEPTH` nodes), ``unknown-tag`` over every node, ``model-order`` (a
    ``model_call`` to a name not in ``models``, when given) and then
    ``subject-forbidden`` (a ``subject_op`` outside a claim body, ``role`` other
    than ``"body"``): R119 (c) puts the three codes before it wherever they
    stand.  No level is consulted, so a suite can run this
    over each expression it holds before it is recorded as needing a level the
    runner lacks (R107 d, R69 c tier 2).  With ``subjects=False`` the
    ``subject-forbidden`` pass is left to the caller, which runs it over every
    expression of a row after this one has read them all (R119 c).
    """
    nodes = []
    for node, depth in walk(root):
        if depth > MAX_DEPTH:
            raise Refusal("depth", f"an expression nested more than {MAX_DEPTH} deep")
        nodes.append(node)
    for node in nodes:
        if tag_level(node[0]) is None:
            raise Refusal("unknown-tag", node[0])
    for node in nodes:  # model-order wherever it stands (R119 c)
        if node[0] == "model_call" and models is not None:
            callee = node[1] if len(node) > 1 else None
            if type(callee) is not str or callee not in models:
                raise Refusal(
                    "model-order", "a model_call names a model that does not exist"
                )
    if not subjects:  # the caller runs the subject pass itself, after other rows
        return
    for node in nodes:
        if node[0] == "subject_op" and role != "body":
            raise Refusal(
                "subject-forbidden", "subject calls may appear only in a claim body"
            )


# ---------------------------------------------------------------------------
# Compiled expressions
# ---------------------------------------------------------------------------
@dataclass
class Compiled:
    """One checked, compiled expression.

    ``ty`` is its static type; ``fn`` maps an environment (variable name to
    value) to its value; ``cost`` its worst-path node count; ``subject`` whether a
    ``subject_op`` sits beneath it; ``frames`` the deepest chain of ``model_call``
    frames below it; ``calls`` the subject calls it makes per case (section 4.4:
    a constant, or a function of the environment when a pure guard decides);
    ``nest`` how deeply its closures nest, which is what decides whether the
    evaluation needs more Python stack than the default.

    ``written`` is only set on a bytes literal: the type the corpus *wrote*,
    ``Bytes``.  The node's static type ``ty`` is ``Bytes<len(value)>`` (R51,
    [D60], R73), so it flows into a parameter of that width and no other, and
    **that** is what a report prints as ``expect.type``.  The written type is
    "written nowhere" in the corpus IR (R51); it is not the type of the
    expression.
    """

    ty: Ty
    fn: Callable[[dict], object]
    cost: int
    subject: bool = False
    frames: int = 0
    calls: Calls = 0
    nest: int = 1
    written: Ty | None = None


@dataclass
class ModelDef:
    """A compiled model: its parameter names and types and its body."""

    params: list[tuple[str, Ty]]
    body: Compiled


@dataclass
class OpSig:
    """An operation signature from a section 6 ``ops`` row.

    ``width_params`` are the names of the op's static widths, in order; the
    signature mentions them as ``Bytes<N>`` and a call binds them to numbers.
    """

    params: list[tuple[str, Ty]]
    ret: Ty
    width_params: tuple[str, ...] = ()

    def unbound(self) -> tuple[list[tuple[str, Ty]], Ty]:
        """The signature with every named width left unstated: a plain ``Bytes`` (R70).

        A fault row has no call site and so no widths; its expressions are typed
        as if each ``Bytes<N>`` were ``Bytes``.
        """
        return (
            [(name, forget_widths(ty)) for name, ty in self.params],
            forget_widths(self.ret),
        )

    def bind(self, widths: list[int]) -> tuple[list[tuple[str, Ty]], Ty]:
        """The parameters and result with the static widths substituted (B2)."""
        table = dict(zip(self.width_params, widths, strict=True))
        return (
            [(name, bind_widths(ty, table)) for name, ty in self.params],
            bind_widths(self.ret, table),
        )


def _sum_calls(parts: list[Calls]) -> Calls:
    """The total of per-node call counts, staying a constant while every part is one."""
    constant = sum(p for p in parts if isinstance(p, int))
    dynamic = [p for p in parts if not isinstance(p, int)]
    if not dynamic:
        return constant
    return lambda env: constant + sum(p(env) for p in dynamic)  # type: ignore[operator]


def _branch_calls(cond: Callable[[dict], object], taken: Calls, other: Calls) -> Calls:
    """Calls of ``if cond then A else B``: those of the branch the pure guard takes."""
    if isinstance(taken, int) and taken == other:
        return taken
    return lambda env: calls_of(taken if cond(env) else other, env)


def calls_of(calls: Calls, env: dict) -> int:
    return calls if isinstance(calls, int) else calls(env)


_ABSENT: object = object()  # "no table given", unlike an explicit ``None``

# The nine nodes that *produce* an Option, which only ``unwrap_or`` consumes at
# ``ir.fixed`` (section 4.1, [D23]).
_PRODUCERS = frozenset(
    {
        "try",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "div",
        "rem",
        "div_trunc",
        "rem_trunc",
        "at",
    }
)
_FIXED_TAGS = LEVEL_TAGS["ir.fixed"]
_BITWISE = {
    "bit_and": operator.and_,
    "bit_or": operator.or_,
    "bit_xor": operator.xor,
}
_SHIFTS = frozenset({"shl", "shr", "sar"})
_ROTATIONS = frozenset({"rotl", "rotr"})
_COUNTS = frozenset({"popcount", "clz", "ctz"})
_CASTS = frozenset({"widen", "wrap", "try", "from_be", "from_le"})
_EXTREMES = frozenset({"max_value", "min_value"})
_CHECKED = {"checked_add": "add", "checked_sub": "sub", "checked_mul": "mul"}
_SOME = fixed.SOME
_NONE = fixed.NONE


def _holds_option(ty: Ty) -> bool:
    """Is ``ty`` an ``Option``, or does one stand anywhere inside it?"""
    return ty.head == "Option" or any(
        isinstance(arg, Ty) and _holds_option(arg) for arg in ty.args
    )


class Program:
    """The models and ops of one corpus (or one conformance case), checked on demand.

    ``levels`` are the levels the evaluator is configured with; only those this
    runner :data:`IMPLEMENTED_LEVELS` take effect, so a case that names a level
    the runner lacks sees ``level-missing`` for that level's tags.

    **Levels are decided per expression, from what it reaches** ([D64]).  A corpus
    holds models and ops of every level its adopter needs, and a lane that lacks
    one of those levels must still run the suites that do not need it: the
    others are recorded not-live, not refused.  So nothing here is checked up
    front.  Compiling an expression checks its environment, its literals and its
    tags, then the *call closure*: every model it calls (and those models'
    callees, and so on) and every op it calls.  A model or op nothing reaches is
    never looked at, and a missing level it would have needed goes unnoticed --
    which is exactly the point.  Only facts about the corpus as a whole (the
    cost bound, ``model_order`` being a permutation of ``models``) are checked
    when the program is built.
    """

    def __init__(
        self,
        *,
        types: object = _ABSENT,
        ops: dict | None = None,
        models: dict | None = None,
        model_order: list | None = None,
        levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
        max_ir_cost: object = MAX_IR_COST,
        sem: Semantics = STANDARD,
    ) -> None:
        # Absent means ``{}``; a *present* non-object (``null``, a list) is kept so
        # that the table check below refuses it (section 11, section 2.2).
        self.types = {} if types is _ABSENT else types
        self.sem = sem
        self.levels = frozenset(levels) & IMPLEMENTED_LEVELS
        if (
            type(max_ir_cost) is not int
            or not MAX_IR_COST <= max_ir_cost <= MAX_IR_COST_CEILING
        ):
            raise Refusal(
                "ir-cost", "max_ir_cost lies outside the permitted budget range"
            )
        self.max_cost = max_ir_cost
        # Section 2.2: the limits of the types table hold "again by every
        # consumer", whether or not anything reaches the row.
        check_types_table(self.types, "ill-typed")
        self.subject_hook: Callable[[str, list], object] | None = None
        self._raw_models = models if models is not None else {}
        self._raw_ops = ops if ops is not None else {}
        self._order = list(model_order) if model_order is not None else []
        # model_order is the declaration order of exactly the models there are
        # (F9): a name twice, a name with no model and a model with no name are
        # all the same defect, and each would let a call slip past rule 1.
        if (
            type(self._raw_models) is not dict
            or type(self._raw_ops) is not dict
            or not all(type(name) is str for name in self._order)
            or len(set(self._order)) != len(self._order)
            or set(self._order) != set(self._raw_models)
        ):
            raise Refusal(
                "model-order", "model_order and models do not name the same models"
            )
        self._position = {name: place for place, name in enumerate(self._order)}
        self._check_corpus_structure()
        self._impure_guards = 0  # enclosing guards whose condition calls the subject
        # A fault row is typed against the type it replaces (section 16.17 R67, section
        # 16.20 R96 to R98): `plain_bytes` holds the `id`s of the bytes literals that
        # stand where a plain `Bytes` is expected (they are typed `Bytes`, not
        # `Bytes<len>`), and `recorded`, when a dict, receives every node's type so
        # the caller can compare each position afterwards.
        self.plain_bytes: set[int] = set()
        self.handed: dict[
            int, bool
        ] = {}  # id of an if/match at such a position -> narrow
        self.recorded: dict[int, Ty] | None = None
        self.models: dict[str, ModelDef] = {}  # the ones compiled so far
        self.reached_ops: dict[str, OpSig] = {}  # the ops whose signature was read
        # The ops a compiled tree *calls* (`subject_op` nodes).  Reading a
        # signature (the line above) is not reaching: the corpus check reads every
        # op's.  Only a called op has a stand-in that counts (R100).
        self.called_ops: set[str] = set()
        self.not_live_models: dict[str, str] = {}  # name -> why (a missing level)
        self.not_live_ops: dict[str, str] = {}
        self.errors: dict[str, tuple[str, ...]] = {
            name: tuple(row["cases"])
            for name, row in self.types.items()
            if isinstance(row, dict)
            and row.get("kind") == "error"
            and isinstance(row.get("cases"), list)
        }
        self._validate_what_the_lane_can_run()

    # -- what is wrong with a corpus whatever level the lane implements ------------
    def _check_corpus_structure(self) -> None:
        """The checks that are not a question of level, over *every* model (R69 d, R77).

        A model the lane cannot run because it needs a missing level is still
        part of the corpus, and a corpus may not hide a defect behind a level:
        an unknown tag, a subject call in a model body, a call to a model not
        declared earlier, an expression nested too deep, a model body costing
        more than the bound and a chain of models more than 64 deep are corpus
        errors wherever they stand.  Nothing here looks at a type, so nothing
        here needs a level.  Models are taken in declaration order, so each
        model's cost and chain depth are known before any caller needs them.
        """
        chain: dict[str, int] = {}
        for name in self._order:
            row = self._raw_models[name]
            if type(row) is not dict or "body" not in row:
                raise Refusal("ill-typed", "a model row has no body")
            body = row["body"]
            callees = self.scan(body, owner=self._position[name], levels=False)
            if any(self._position[c] >= self._position[name] for c in callees):
                raise Refusal(
                    "model-order",
                    "a model calls one that comes no earlier in the order",
                )
            chain[name] = 1 + max((chain[c] for c in callees), default=0)
            if chain[name] > MAX_FRAMES:
                raise Refusal(
                    "depth", "models call each other in a chain longer than the limit"
                )

    def _validate_what_the_lane_can_run(self) -> None:
        """Compile every model and read every op whose levels the lane has (R69 i).

        A model or op whose own tags and types are inside the lane's levels is
        validated in full whether or not anything reaches it: every code of
        section 11 applies (an unused model with a subject call, a model body over
        the cost bound ...).  One that needs a level the lane lacks cannot be
        compiled; it is *classified* -- recorded in ``not_live_models`` or
        ``not_live_ops`` with the level it needs -- and the suites and controls
        that reach it are not-live (R69 c, [D65]: the model or op has no row of
        its own).  The tag-name checks of tier (ii) already ran, over all models.
        """
        for name in self._order:
            try:
                self._reach_models([name], len(self._order))
            except Refusal as refusal:
                if refusal.code != "level-missing":
                    raise
                self.not_live_models[name] = refusal.message
        for name in self._raw_ops:
            try:
                self.op(name)
            except Refusal as refusal:
                if refusal.code != "level-missing":
                    raise
                self.not_live_ops[name] = refusal.message

    # -- declared types and levels ---------------------------------------------
    def need_levels(self, needed: set[str], what: str) -> None:
        missing = needed - self.levels
        if missing:
            raise Refusal("level-missing", f"{what} needs {sorted(missing)[0]}")

    def _declared(
        self,
        text: object,
        widths: frozenset[str] = frozenset(),
        *,
        allow_int: bool = False,
    ) -> Ty:
        """A type written in a signature or binder; a bad text is ``ill-typed``.

        ``allow_int`` is for a *model parameter* alone ([D75], R111): a model row
        is a signature and never a value, so it may name ``Int`` (also under
        ``Option``, ``Result``, a tuple or ``List``).  An op row, a binder, a
        field and every value still refuse it.
        """
        try:
            ty = parse_type(text, self.types, widths=widths, allow_int=allow_int)
        except Refusal as error:
            raise Refusal("ill-typed", f"bad declared type: {error.message}") from error
        self.need_levels(type_level(ty, self.types), str(ty))
        return ty

    def _scope_types(self, scope: dict[str, Ty]) -> None:
        """The environment of an expression is part of what it reaches."""
        for name, ty in scope.items():
            self.need_levels(type_level(ty, self.types), f"the type of {name}")

    # -- ops: read when a subject call reaches them -----------------------------
    def op(self, name: object) -> OpSig:
        """The signature of the op ``name``: parsed, level-checked and remembered."""
        if type(name) is not str or name not in self._raw_ops:
            raise Refusal(
                "ill-typed", "a subject call names an op the corpus does not declare"
            )
        if name in self.reached_ops:
            return self.reached_ops[name]
        try:
            row = self._raw_ops[name]
            widths = row.get("width_params", [])
            if (
                type(widths) is not list
                or not all(type(w) is str for w in widths)
                or len(set(widths)) != len(widths)
            ):
                raise Refusal(
                    "ill-typed", "width_params must be a list of different names"
                )
            if "requires" in row:  # section 6: always present in a real corpus
                check_requires(row["requires"], "ill-typed", "an op's requires")
            names = frozenset(widths)
            params = [(p, self._declared(t, names)) for p, t in row["params"]]
            given = [p for p, _t in params]
            if not all(type(p) is str for p in given) or len(set(given)) != len(given):
                raise Refusal("ill-typed", "an op parameter name is repeated")
            sig = OpSig(params, self._declared(row["ret"], names), tuple(widths))
        except (KeyError, TypeError, IndexError, ValueError, AttributeError) as error:
            raise Refusal("ill-typed", "an op row has the wrong shape") from error
        self.reached_ops[name] = sig
        return sig

    # -- models: compiled when a call reaches them -------------------------------
    def model(self, name: str) -> ModelDef:
        """Compile the model ``name`` and what it calls (a corpus reader's door)."""
        if name not in self._raw_models:
            raise Refusal("model-order", "a model_call to an unknown model")
        self._reach_models([name], len(self._order))
        return self.models[name]

    def _reach_models(self, called: list[str], caller: int) -> None:
        """Make every model in the call closure of ``called`` checked and compiled.

        ``caller`` is the position, in ``model_order``, of whoever makes the calls
        (``len(order)`` for a top-level expression, which comes after every
        model).  Rule 1 of section 4.2: a call must go to a model declared
        *earlier*.  The closure is walked with a work list, not recursion, so a
        corpus whose models call one another in a chain thousands long is
        refused (or accepted) without touching Python's stack; the models are
        then compiled in declaration order, so each finds its callees ready.
        """
        work = [(name, caller) for name in called]
        closure: set[str] = set()
        while work:
            name, here = work.pop()
            if name not in self._position or self._position[name] >= here:
                raise Refusal(
                    "model-order",
                    "a model calls one that comes no earlier in the order",
                )
            if name in closure or name in self.models:
                continue
            closure.add(name)
            try:
                body = self._raw_models[name]["body"]
            except (KeyError, TypeError) as error:
                raise Refusal("ill-typed", "a model row has no body") from error
            work.extend(
                (callee, self._position[name])
                for callee in self.scan(body, owner=self._position[name])
            )
        for name in sorted(closure, key=self._position.__getitem__):
            self._compile_model(name)

    def _compile_model(self, name: str) -> None:
        try:
            row = self._raw_models[name]
            scope: dict[str, Ty] = {}
            for param, text in row["params"]:
                if param in scope or type(param) is not str:
                    raise Refusal("ill-typed", "a model parameter name is repeated")
                scope[param] = self._declared(text, allow_int=True)  # [D75]
            body = self._compile(row["body"], scope)
        except (KeyError, TypeError, IndexError, ValueError, AttributeError) as error:
            raise Refusal("ill-typed", "a model row has the wrong shape") from error
        self.models[name] = ModelDef(list(scope.items()), body)

    # -- the static scan ----------------------------------------------------------
    def scan(
        self,
        root: object,
        *,
        owner: int | None,
        role: str = "body",
        levels: bool = True,
    ) -> list[str]:
        """Depth, tags and subject placement of one expression; its model callees.

        ``owner`` is the position of the model whose body this is in
        ``model_order``, or ``None`` for a top-level expression.  The checks are
        structural: they look at tags, never at types, and at *every* node of
        the expression including the ones an evaluation would never reach,
        because an untaken branch is still part of what the corpus says.  The
        names of the models called are returned for the closure walk.
        """
        nodes = []
        for node, depth in walk(root):
            if depth > MAX_DEPTH:
                raise Refusal(
                    "depth", f"an expression nested more than {MAX_DEPTH} deep"
                )
            nodes.append(node)
        callees: list[str] = []
        for node in nodes:  # a tag of no level is an error before any level question
            if tag_level(node[0]) is None:
                raise Refusal("unknown-tag", node[0])
        for node in nodes:
            if levels and tag_level(node[0]) not in self.levels:
                raise Refusal("level-missing", node[0])
        for node in nodes:  # model-order wherever it stands, before subject-forbidden
            if node[0] == "model_call":
                callee = node[1] if len(node) > 1 else None
                if type(callee) is not str or callee not in self._position:
                    raise Refusal(
                        "model-order", "a model_call names a model that does not exist"
                    )
                callees.append(callee)
        for node in nodes:
            if node[0] == "subject_op" and (owner is not None or role != "body"):
                raise Refusal(
                    "subject-forbidden", "subject calls may appear only in a claim body"
                )
        return callees

    # -- compiling ---------------------------------------------------------------
    def compile(self, ir: object, scope: dict[str, Ty], role: str = "body") -> Compiled:
        """Check and compile a top-level expression over the variables in ``scope``.

        ``role`` is ``"body"`` (subject calls allowed) or any other word for an
        ``admit``, ``expected`` or fault expression, where a subject call is
        ``subject-forbidden`` (section 4.2 rule 3).  Everything the expression
        reaches is checked here and nothing else is.
        """
        try:
            self._scope_types(scope)
            self._reach_models(self.scan(ir, owner=None, role=role), len(self._order))
            return self._compile(ir, scope)
        except (KeyError, TypeError, IndexError, ValueError, AttributeError) as error:
            raise Refusal("ill-typed", "an expression has the wrong shape") from error

    def _compile(
        self, node: object, scope: dict[str, Ty], *, intermediate: bool = False
    ) -> Compiled:
        """Check and compile ``node``.

        ``intermediate`` is set by ``unwrap_or`` for its first operand alone: the
        one place an ``Option`` may stand without ``ir.data`` (section 4.1,
        [D23]).  Any other node whose static type holds an ``Option``, and a
        producer that is not directly under ``unwrap_or``, needs that level.
        """
        try:
            compiled = self._node(node, scope)
        except RecursionError as error:  # pragma: no cover - depth is capped at 64
            raise Refusal(
                "depth", "the expression nests too deeply to compile"
            ) from error
        if self.recorded is not None:
            self.recorded[id(node)] = compiled.ty
        if _holds_option(compiled.ty) and not (
            intermediate and is_node(node) and node[0] in _PRODUCERS  # type: ignore[index]
        ):
            self.need_levels({"ir.data"}, "an Option that unwrap_or does not consume")
        if compiled.cost > self.max_cost:
            raise Refusal(
                "ir-cost", f"worst-path cost {compiled.cost} exceeds {self.max_cost}"
            )
        if compiled.frames > MAX_FRAMES:
            raise Refusal("depth", "model calls nest deeper than the limit")
        return compiled

    def _klass(self, subject_beneath: bool) -> str:
        """The class of an overflow at a node (section 3.2, R71).

        ``corpus`` exactly when the generator evaluates the node: nothing it
        calls the subject through, and no guard above it whose condition does.
        A pure node reached only under such a guard is not on the pure trace, so
        its overflow depends on what the subject answered: a case failure.
        """
        return "case" if subject_beneath or self._impure_guards else "corpus"

    def _branch_type(self, node: list, first: Ty, second: Ty) -> Ty | None:
        """The common type of two branches or arms of the ``if`` / ``match`` ``node``.

        R51's ``unify``, except at a position of a fault row the declared type is
        handed to, where the fault relation applies (R96 to R98: tuples relate
        component by component).
        """
        if id(node) in self.handed:
            return fault_unify(first, second, self.handed[id(node)])
        return unify(first, second)

    def _under_guard(
        self, guard: Compiled, node: object, scope: dict[str, Ty]
    ) -> Compiled:
        """Compile ``node``, a branch or operand, under the condition ``guard``."""
        step = 1 if guard.subject else 0
        self._impure_guards += step
        try:
            return self._compile(node, scope)
        finally:
            self._impure_guards -= step

    def _kids(self, node: list, scope: dict[str, Ty], count: int) -> list[Compiled]:
        """Compile the ``count`` operands of ``node`` (positions 1..count)."""
        if len(node) != count + 1:
            raise Refusal("ill-typed", f"{node[0]} takes {count} operand(s)")
        return [self._compile(child, scope) for child in node[1:]]

    def _node(self, node: object, scope: dict[str, Ty]) -> Compiled:
        if not is_node(node):
            raise Refusal(
                "ill-typed", "an IR node must be a list that starts with a tag name"
            )
        assert isinstance(node, list)
        tag = node[0]
        if tag == "lit":
            return self._literal(node)
        if tag == "var":
            if len(node) != 2 or not isinstance(node[1], str) or node[1] not in scope:
                raise Refusal("ill-typed", "an unbound variable")
            name = node[1]
            return Compiled(scope[name], lambda env: env[name], 1)
        if tag in ("subject_op", "model_call"):
            return self._call(node, scope)
        if tag == "err":
            return self._err(node)
        if tag in ("len", "ok", "not", "neg", "is_err"):
            (arg,) = self._kids(node, scope, 1)
            return self._unary(tag, arg)
        if tag == "if":
            return self._if(node, scope)
        if tag in ("and", "or", "implies"):
            return self._logic(node, scope)
        if tag in _FIXED_TAGS:
            return self._fixed(tag, node, scope)
        if tag == "tuple":
            return self._tuple(node, scope)
        if tag == "project":
            return self._project(node, scope)
        if tag == "match":
            return self._match(node, scope)
        left, right = self._kids(node, scope, 2)
        return self._binary(tag, left, right)

    # -- node families -------------------------------------------------------------
    @staticmethod
    def _combine(
        ty: Ty,
        fn: Callable[[dict], object],
        kids: list[Compiled],
        *,
        calls: Calls | None = None,
        cost: int | None = None,
    ) -> Compiled:
        """Assemble a node: cost ``1 + sum``, subject/frames from the children."""
        return Compiled(
            ty,
            fn,
            1 + sum(k.cost for k in kids) if cost is None else cost,
            any(k.subject for k in kids),
            max((k.frames for k in kids), default=0),
            _sum_calls([k.calls for k in kids]) if calls is None else calls,
            1 + max((k.nest for k in kids), default=0),
        )

    def _logic(self, node: list, scope: dict[str, Ty]) -> Compiled:
        """``and``, ``or``, ``implies``: short-circuit; the left operand is a guard.

        The calls of ``left OP right`` are those of the left operand plus those
        of the right operand *unless the left operand ends the evaluation*:
        ``and`` stops at ``false``, ``or`` at ``true``, ``implies`` at ``false``.
        A guard that itself calls the subject cannot be traced (the generator
        never learns what the call returned), so such a guard may not guard
        another call: ``impure-guard`` (section 4.4).
        """
        tag = node[0]
        if len(node) != 3:
            raise Refusal("ill-typed", f"{tag} takes 2 operand(s)")
        left = self._compile(node[1], scope)
        right = self._under_guard(left, node[2], scope)
        if left.ty != BOOL or right.ty != BOOL:
            raise Refusal("ill-typed", f"{tag} takes two Bool operands")
        if left.subject and right.subject:
            raise Refusal(
                "impure-guard",
                "a guard that calls the subject sits over a subject call",
            )
        lf, rf = left.fn, right.fn
        sem = self.sem
        run = {"and": sem.and_, "or": sem.or_, "implies": sem.implies}[tag]
        ends = {
            "and": lambda v: not v,
            "or": lambda v: bool(v),
            "implies": lambda v: not v,
        }[tag]
        calls = (
            _sum_calls([left.calls, right.calls])
            if left.subject
            else _short_calls(lf, ends, right.calls)
        )
        return self._combine(
            BOOL,
            lambda env: run(lambda: lf(env), lambda: rf(env)),
            [left, right],
            calls=calls,
        )

    def _literal(self, node: list) -> Compiled:
        """``true``/``false``, or a typed literal ``{"type": T, "value": V}``.

        A Bool is written ``["lit", true]`` and no other way (the object form with
        ``"type": "Bool"`` is ``ill-typed``).  A bytes literal is written with the
        type ``Bytes`` -- ``Bytes<N>`` is refused -- and its *static* type is
        ``Bytes<len(value)>`` (R51, [D60]); see :class:`Compiled`.
        """
        if len(node) == 2 and type(node[1]) is bool:
            value: object = node[1]
            return Compiled(BOOL, lambda _env: value, 1)
        spec = node[1] if len(node) == 2 else None
        if type(spec) is not dict or set(spec) != {"type", "value"}:
            raise Refusal(
                "ill-typed",
                "a literal is a bare boolean or an object "
                "with exactly a type and a value",
            )
        try:
            ty = parse_type(spec["type"], self.types, allow_int=True)
            # The written type of a literal counts for levels (section 11): a tuple
            # type is a missing level before it is a bad literal.
            self.need_levels(type_level(ty, self.types), f"the literal type {ty}")
            if ty.head == "Bool" or (ty.head == "Bytes" and ty.args):
                raise Refusal(
                    "ill-typed", "booleans are written bare and Bytes without a width"
                )
            if not (ty.is_number or ty.head == "Bytes"):
                raise Refusal("ill-typed", "this type has no literal form here")
            if ty.head == "Int":
                value = self.sem.decode_int(spec["value"], ty)
            else:
                value = decode_value(spec["value"], ty, self.types, self.sem)
        except Refusal as error:
            if error.code == "level-missing":
                raise
            raise Refusal("ill-typed", f"bad literal: {error.message}") from error
        if ty.head == "Bytes":
            assert isinstance(value, bytes)
            # R67: where a fault row expects a plain `Bytes` the literal IS a `Bytes`.
            static = (
                BYTES if id(node) in self.plain_bytes else Ty("Bytes", (len(value),))
            )
            return Compiled(static, lambda _env: value, 1, written=ty)
        return Compiled(ty, lambda _env: value, 1)

    def _err(self, node: list) -> Compiled:
        if len(node) != 2 or not isinstance(node[1], str):
            raise Refusal("ill-typed", "err needs the name of an error case")
        case = node[1]
        owners = [n for n, cases in self.errors.items() if case in cases]
        if not owners:
            raise Refusal("ill-typed", "no error type declares that case")
        error = Ty(owners[0]) if len(owners) == 1 else UNKNOWN_TY
        value = ("err", case)
        return Compiled(Ty("Result", (UNKNOWN_TY, error)), lambda _env: value, 1)

    def _unary(self, tag: str, arg: Compiled) -> Compiled:
        sem, f, klass = self.sem, arg.fn, self._klass(arg.subject)
        if tag == "not":
            if arg.ty != BOOL:
                raise Refusal("ill-typed", "not takes a Bool")
            return self._combine(BOOL, lambda env: not f(env), [arg])
        if tag == "len":
            if arg.ty.head != "Bytes":
                raise Refusal("ill-typed", "len takes Bytes")
            return self._combine(NAT, lambda env: len(f(env)), [arg])  # type: ignore[arg-type]
        if tag == "ok":
            return self._combine(
                Ty("Result", (arg.ty, UNKNOWN_TY)), lambda env: ("ok", f(env)), [arg]
            )
        if tag == "is_err":
            if arg.ty.head != "Result":
                raise Refusal("ill-typed", "is_err takes a Result")
            return self._combine(BOOL, lambda env: f(env)[0] == "err", [arg])  # type: ignore[index]
        # neg
        if arg.ty.is_fixed:
            ty = arg.ty
            return self._combine(ty, lambda env: sem.fixed_neg(f(env), ty), [arg])  # type: ignore[arg-type]
        if arg.ty.is_exact:

            def neg_exact(env: dict) -> int:
                try:
                    return sem.exact(-f(env))  # type: ignore[operator]
                except WindowExceeded:
                    raise IrOverflow(klass, "an exact value left the window") from None

            return self._combine(INT, neg_exact, [arg])
        raise Refusal("ill-typed", "neg takes a number")

    def _if(self, node: list, scope: dict[str, Ty]) -> Compiled:
        if len(node) != 4:
            raise Refusal("ill-typed", "if takes 3 operand(s)")
        cond = self._compile(node[1], scope)
        then = self._under_guard(cond, node[2], scope)
        other = self._under_guard(cond, node[3], scope)
        if cond.ty != BOOL:
            raise Refusal("ill-typed", "an if condition must be true-or-false")
        common = self._branch_type(node, then.ty, other.ty)
        if common is None:
            raise Refusal("ill-typed", "the two branches of an if share no type")
        if cond.subject and (then.subject or other.subject):
            raise Refusal(
                "impure-guard",
                "a guard that calls the subject sits over a subject call",
            )
        cf, tf, ef, sem = cond.fn, then.fn, other.fn, self.sem
        # An impure condition may not guard a call (checked above), so the branches
        # call nothing and the calls are the condition's; a pure condition picks the
        # branch whose calls count.
        calls = (
            cond.calls if cond.subject else _branch_calls(cf, then.calls, other.calls)
        )
        # worst path: the condition, then the more expensive branch
        cost = 1 + cond.cost + max(then.cost, other.cost)
        return self._combine(
            common,
            lambda env: sem.if_(lambda: cf(env), lambda: tf(env), lambda: ef(env)),
            [cond, then, other],
            calls=calls,
            cost=cost,
        )

    def _binary(self, tag: str, left: Compiled, right: Compiled) -> Compiled:
        sem, lf, rf = self.sem, left.fn, right.fn
        klass = self._klass(left.subject or right.subject)
        a, b = left.ty, right.ty
        if tag in ("eq", "ne"):
            if unify(a, b) is None:
                raise Refusal("ill-typed", "equality compares values of one type only")
            if tag == "eq":
                return self._combine(
                    BOOL, lambda env: lf(env) == rf(env), [left, right]
                )
            return self._combine(BOOL, lambda env: lf(env) != rf(env), [left, right])
        if tag in ("lt", "gt", "le", "ge"):
            if not (a.is_number and b.is_number) or (
                (a.is_fixed or b.is_fixed) and a != b
            ):
                raise Refusal(
                    "ill-typed", "an ordering compares two numbers of the same kind"
                )
            compare = {
                "lt": lambda x, y: x < y,
                "gt": lambda x, y: x > y,
                "le": lambda x, y: x <= y,
                "ge": lambda x, y: x >= y,
            }[tag]
            return self._combine(
                BOOL,
                lambda env: compare(
                    sem.order_key(lf(env), a), sem.order_key(rf(env), b)
                ),  # type: ignore[arg-type]
                [left, right],
            )
        if tag in ("add", "sub", "mul"):
            if a.is_fixed and a == b:
                return self._combine(
                    a, self._fixed_arith(tag, a, lf, rf), [left, right]
                )
            if a.is_exact and b.is_exact:

                def exact(env: dict) -> int:
                    try:
                        return sem.exact(raw_binary(tag, lf(env), rf(env)))  # type: ignore[arg-type]
                    except WindowExceeded:
                        raise IrOverflow(
                            klass, "an exact value left the window"
                        ) from None

                return self._combine(INT, exact, [left, right])
            raise Refusal("ill-typed", f"{tag} needs two operands of one number type")
        return self._bytes(tag, left, right, klass)

    def _fixed_arith(
        self,
        tag: str,
        ty: Ty,
        lf: Callable[[dict], object],
        rf: Callable[[dict], object],
    ) -> Callable[[dict], object]:
        """``add``/``sub``/``mul`` on a fixed-width type: compute exactly, then wrap.

        The general form goes through the semantics hooks, so a deliberately wrong
        implementation can replace the rule.  When the semantics is the standard
        one (neither hook is overridden) the same rule is written inline -- one
        ``& mask`` for an unsigned type, ``& mask`` and a conditional subtraction
        for a signed one -- because a chunk of 65,536 cases runs this node
        hundreds of thousands of times and three extra calls per node dominate.
        """
        sem = self.sem
        cls = type(sem)
        if (
            cls.fixed_binary is not Semantics.fixed_binary
            or cls.wrap is not Semantics.wrap
        ):
            return lambda env: sem.fixed_binary(tag, lf(env), rf(env), ty)  # type: ignore[arg-type]
        mask, half, full = (1 << ty.width) - 1, 1 << (ty.width - 1), 1 << ty.width
        exact = {"add": operator.add, "sub": operator.sub, "mul": operator.mul}[tag]
        if not ty.signed:
            return lambda env: exact(lf(env), rf(env)) & mask  # type: ignore[arg-type]

        def signed(env: dict) -> int:
            value = exact(lf(env), rf(env)) & mask  # type: ignore[arg-type]
            return value - full if value >= half else value

        return signed

    def _bytes(self, tag: str, left: Compiled, right: Compiled, klass: str) -> Compiled:
        sem, lf, rf = self.sem, left.fn, right.fn
        if left.ty.head != "Bytes":
            raise Refusal("ill-typed", f"{tag} takes Bytes first")
        if tag == "concat":
            if right.ty.head != "Bytes":
                raise Refusal("ill-typed", "concat takes two Bytes")

            def concat(env: dict) -> bytes:
                x, y = lf(env), rf(env)
                # Checked BEFORE the allocation: section 14.
                if len(x) + len(y) > MAX_IR_VALUE:  # type: ignore[arg-type]
                    raise IrOverflow(klass, "a Bytes value would exceed MAX_IR_VALUE")
                return x + y  # type: ignore[operator]

            widths = [t.ty.args[0] for t in (left, right) if t.ty.args]
            sized = len(widths) == 2 and all(isinstance(w, int) for w in widths)
            ty = Ty("Bytes", (sum(widths),)) if sized else BYTES  # type: ignore[arg-type]
            return self._combine(ty, concat, [left, right])
        if tag not in ("take", "drop") or not right.ty.is_exact:
            raise Refusal("ill-typed", f"{tag} takes Bytes and a Nat or Int count")
        cut = sem.take if tag == "take" else sem.drop
        return self._combine(BYTES, lambda env: cut(lf(env), rf(env)), [left, right])  # type: ignore[arg-type]

    # -- ir.fixed (Appendix A.1) -----------------------------------------------------
    def _type_arg(self, text: object) -> Ty:
        """The static type argument of ``widen``, ``wrap``, ``try``, ``max_value``...

        It is a type text; one the reader refuses, or one that is not a
        fixed-width type, makes the node ``ill-typed``.  The type counts for
        levels like any type written in a node (section 16.18 R69 (b)).
        """
        try:
            ty = parse_type(text, self.types)
        except Refusal as error:
            raise Refusal("ill-typed", f"bad type argument: {error.message}") from error
        self.need_levels(type_level(ty, self.types), f"the type argument {ty}")
        if not ty.is_fixed:
            raise Refusal("ill-typed", "this node names a fixed-width type")
        return ty

    @staticmethod
    def _same_fixed(left: Compiled, right: Compiled, tag: str) -> Ty:
        """Two operands of one fixed-width type; mixed widths or signs are ill-typed."""
        if not left.ty.is_fixed or left.ty != right.ty:
            raise Refusal(
                "ill-typed", f"{tag} takes two fixed-width operands of one type"
            )
        return left.ty

    def _fixed(self, tag: str, node: list, scope: dict[str, Ty]) -> Compiled:
        """Every node of the ``ir.fixed`` level, the casts and ``unwrap_or`` aside.

        Typing is *inferred*, as for ``add`` (Appendix A.1): operands are typed
        from literals, variables and signatures, and a mismatch -- mixed widths,
        ``sar`` on an unsigned type, a count that is not a ``Nat`` -- is
        ``ill-typed``.  The glue is the same for all of them: compile the
        operands (cost ``1 + sum``), check their types, close over the one
        function of :mod:`.fixed` or :class:`~.semantics.Semantics` that does
        the arithmetic.
        """
        if tag in _CASTS:
            return self._cast(tag, node, scope)
        if tag in _EXTREMES:
            return self._extreme(tag, node)
        if tag == "unwrap_or":
            return self._unwrap_or(node, scope)
        sem = self.sem
        unary = tag in _COUNTS or tag in (
            "bit_not",
            "to_int",
            "to_nat",
            "be_bytes",
            "le_bytes",
        )
        kids = self._kids(node, scope, 1 if unary else 2)
        a = kids[0]
        af = a.fn
        if unary:
            if not a.ty.is_fixed:
                raise Refusal("ill-typed", f"{tag} takes a fixed-width operand")
            return self._unary_fixed(tag, a, af)
        b = kids[1]
        bf = b.fn
        if tag in _BITWISE:
            ty = self._same_fixed(a, b, tag)
            op = _BITWISE[tag]
            return self._combine(ty, lambda env: op(af(env), bf(env)), kids)
        if tag in _SHIFTS or tag in _ROTATIONS or tag == "bit":
            # The left operand is fixed-width, the count an exact Nat: a U8 or an
            # Int count is ill-typed, so no count is ever negative.
            if not a.ty.is_fixed or b.ty != NAT or (tag == "sar" and not a.ty.signed):
                raise Refusal(
                    "ill-typed", f"{tag} takes a fixed-width operand and a Nat count"
                )
            ty = a.ty
            if tag == "bit":
                return self._combine(
                    BOOL,
                    lambda env: fixed.bit(af(env), bf(env), ty),
                    kids,  # type: ignore[arg-type]
                )
            run = sem.shift if tag in _SHIFTS else sem.rotate
            return self._combine(ty, lambda env: run(tag, af(env), bf(env), ty), kids)  # type: ignore[arg-type]
        if tag in _CHECKED:
            ty = self._same_fixed(a, b, tag)
            word = _CHECKED[tag]
            return self._combine(
                Ty("Option", (ty,)),
                lambda env: sem.checked(word, af(env), bf(env), ty),  # type: ignore[arg-type]
                kids,
            )
        if tag in ("div", "rem", "div_trunc", "rem_trunc"):
            return self._division(tag, a, b, kids)
        if tag in ("imin", "imax"):
            if not (a.ty.is_exact and b.ty.is_exact):
                raise Refusal("ill-typed", f"{tag} takes Int or Nat operands")
            pick = min if tag == "imin" else max
            return self._combine(INT, lambda env: pick(af(env), bf(env)), kids)  # type: ignore[type-var]
        # at: Bytes, Nat or Int
        if a.ty.head != "Bytes" or not b.ty.is_exact:
            raise Refusal("ill-typed", "at takes Bytes and a Nat or Int index")

        def at(env: dict) -> tuple:
            data, index = af(env), bf(env)
            if index < 0 or index >= len(data):  # type: ignore[arg-type]
                return _NONE
            return (_SOME, data[index])  # type: ignore[index]

        return self._combine(Ty("Option", (Ty("U8"),)), at, kids)

    def _unary_fixed(
        self, tag: str, a: Compiled, af: Callable[[dict], object]
    ) -> Compiled:
        """The one-operand nodes: ``bit_not``, counts, ``to_int``/``to_nat``, bytes."""
        sem, ty = self.sem, a.ty
        if tag == "bit_not":
            if ty.signed:
                return self._combine(ty, lambda env: ~af(env), [a])  # type: ignore[operator]
            mask = (1 << ty.width) - 1
            return self._combine(ty, lambda env: af(env) ^ mask, [a])  # type: ignore[operator]
        if tag == "popcount":
            return self._combine(NAT, lambda env: fixed.popcount(af(env), ty), [a])  # type: ignore[arg-type]
        if tag in ("clz", "ctz"):
            return self._combine(
                NAT,
                lambda env: sem.count_zeros(tag, af(env), ty),
                [a],  # type: ignore[arg-type]
            )
        if tag == "to_int":
            return self._combine(INT, af, [a])
        if tag == "to_nat":
            if ty.signed:
                raise Refusal("ill-typed", "to_nat takes an unsigned operand")
            return self._combine(NAT, af, [a])
        order = "big" if tag == "be_bytes" else "little"
        return self._combine(
            Ty("Bytes", (ty.width // 8,)),
            lambda env: fixed.to_bytes(af(env), ty, order),  # type: ignore[arg-type]
            [a],
        )

    def _division(
        self, tag: str, a: Compiled, b: Compiled, kids: list[Compiled]
    ) -> Compiled:
        """``div``/``rem`` (floor), ``div_trunc``/``rem_trunc`` (toward zero).

        The typing is the grammar checker's (Appendix A.1, normative [D24]):
        ``Int``/``Nat`` operands give ``Option<Int>``, two matching *unsigned*
        fixed-width operands ``Option<U>``, and a signed pair is refused (use the
        truncating pair); the truncating pair takes two signed operands of one type.
        """
        sem, af, bf = self.sem, a.fn, b.fn
        truncating = tag in ("div_trunc", "rem_trunc")
        if truncating:
            if not (a.ty.is_fixed and a.ty.signed and a.ty == b.ty):
                raise Refusal(
                    "ill-typed", f"{tag} takes two signed operands of one type"
                )
            ty: Ty | None = a.ty
            result = Ty("Option", (a.ty,))
        elif a.ty.is_exact and b.ty.is_exact:
            ty, result = None, Ty("Option", (INT,))
        elif a.ty.is_fixed and not a.ty.signed and a.ty == b.ty:
            ty, result = None, Ty("Option", (a.ty,))
        else:
            raise Refusal(
                "ill-typed",
                f"{tag} takes Int or Nat operands or two unsigned operands of one type",
            )
        return self._combine(
            result,
            lambda env: sem.divide(tag, af(env), bf(env), ty),
            kids,  # type: ignore[arg-type]
        )

    def _cast(self, tag: str, node: list, scope: dict[str, Ty]) -> Compiled:
        """``widen``, ``wrap``, ``try``, ``from_be``, ``from_le``: ``[tag, "T", e]``."""
        if len(node) != 3:
            raise Refusal("ill-typed", f"{tag} takes a type and an operand")
        target = self._type_arg(node[1])
        operand = self._compile(node[2], scope)
        sem, f, source = self.sem, operand.fn, operand.ty
        if tag in ("from_be", "from_le"):
            width = target.width // 8
            # The static width is part of the operand's TYPE (section 16.18 R51): a
            # plain Bytes, or a Bytes<N> of another N, is ill-typed.
            if source != Ty("Bytes", (width,)):
                raise Refusal("ill-typed", f"{tag} {target} takes a Bytes<{width}>")
            order = "big" if tag == "from_be" else "little"
            klass = self._klass(operand.subject)
            return self._combine(
                target,
                lambda env: fixed.from_bytes(f(env), target, order, klass),
                [operand],  # type: ignore[arg-type]
            )
        if tag == "widen":
            if not source.is_fixed:
                raise Refusal("ill-typed", "widen takes a fixed-width operand")
            (low, high), (inside_low, inside_high) = (
                fixed_range(target),
                fixed_range(source),
            )
            if not low <= inside_low or inside_high > high:
                raise Refusal("ill-typed", "widen needs a range inside the target")
            return self._combine(target, f, [operand])  # the value is preserved
        if not (source.is_fixed or source.is_exact):
            raise Refusal("ill-typed", f"{tag} takes a fixed-width, Int or Nat operand")
        if tag == "wrap":
            return self._combine(
                target, lambda env: sem.wrap(f(env), target), [operand]
            )  # type: ignore[arg-type]
        low, high = fixed_range(target)

        def attempt(env: dict) -> tuple:
            value = f(env)
            return (_SOME, value) if low <= value <= high else _NONE  # type: ignore[operator]

        return self._combine(Ty("Option", (target,)), attempt, [operand])

    def _extreme(self, tag: str, node: list) -> Compiled:
        """``max_value`` / ``min_value``: ``[tag, "T"]``; never folded to a literal."""
        if len(node) != 2:
            raise Refusal("ill-typed", f"{tag} takes a type")
        ty = self._type_arg(node[1])
        value = fixed_range(ty)[1 if tag == "max_value" else 0]
        return Compiled(ty, lambda _env: value, 1)

    def _unwrap_or(self, node: list, scope: dict[str, Ty]) -> Compiled:
        """``unwrap_or(o, d)``: the payload of ``o``, else ``d``; **both** evaluated.

        There is no short circuit (Appendix A.1): ``d`` is evaluated, and its
        subject calls counted, even when ``o`` holds a value, and the cost is the
        sum.  The first operand may be a producer node directly (the intermediate
        use of section 4.1); an ``Option`` from anywhere else needs ``ir.data``.
        """
        if len(node) != 3:
            raise Refusal("ill-typed", "unwrap_or takes 2 operand(s)")
        option = self._compile(node[1], scope, intermediate=True)
        default = self._compile(node[2], scope)
        if option.ty.head != "Option":
            raise Refusal("ill-typed", "unwrap_or takes an Option first")
        payload = option.ty.args[0]
        if not flows(default.ty, payload):
            raise Refusal(
                "ill-typed", "the default of unwrap_or has the payload's type"
            )
        of, df = option.fn, default.fn

        def unwrap(env: dict) -> object:
            held, fallback = of(env), df(env)
            return held[1] if held[0] == _SOME else fallback  # type: ignore[index]

        return self._combine(payload, unwrap, [option, default])

    # -- ir.tuple: the literal, project, match on a Result -------------------------
    def _tuple(self, node: list, scope: dict[str, Ty]) -> Compiled:
        """``["tuple", [e1, .., en]]``, two to eight operands: a ``(T1,..,Tn)``.

        Nothing lifts under a tuple (R51): ``(b"ab", n)`` is a ``(Bytes<2>,Nat)``
        and ``(1 + 2, 0u8)`` an ``(Int,U8)``; the component types are the static
        types of the operands.
        """
        if len(node) != 2 or type(node[1]) is not list or not 2 <= len(node[1]) <= 8:
            raise Refusal("ill-typed", "a tuple literal has 2 to 8 operands")
        kids = [self._compile(child, scope) for child in node[1]]
        fns = [kid.fn for kid in kids]
        ty = Ty("()", tuple(kid.ty for kid in kids))
        return self._combine(ty, lambda env: tuple(f(env) for f in fns), kids)

    def _project(self, node: list, scope: dict[str, Ty]) -> Compiled:
        """``["project", e, i]``: component ``i`` (0-based JSON integer) of a tuple."""
        if len(node) != 3 or type(node[2]) is not int:
            raise Refusal("ill-typed", "project takes a tuple and an integer index")
        operand = self._compile(node[1], scope)
        index = node[2]
        if operand.ty.head != "()" or not 0 <= index < len(operand.ty.args):
            raise Refusal("ill-typed", "project takes a tuple and an index inside it")
        f = operand.fn
        return self._combine(
            operand.ty.args[index], lambda env: f(env)[index], [operand]
        )  # type: ignore[index]

    def _match(self, node: list, scope: dict[str, Ty]) -> Compiled:
        """``["match", s, [[pat, arm], ..]]`` on a ``Result``: exactly one arm runs.

        **The level is type-directed** (R76): the same JSON is ``ir.tuple`` when
        the scrutinee's static type is ``Result<X,E>`` and its patterns are
        those of Appendix A.1, and ``ir.data`` otherwise.  So the scrutinee is
        typed before anything else is decided, and a ``match`` on a bare tuple,
        a record, a variant, an ``Option`` or a ``Bool``, or one that holds a
        data pattern at any depth, is ``level-missing`` here.

        Then, in the order of R12: every pattern is typed against the type it
        tests (arity, declared error cases, no repeated or shadowing ``bind``);
        an arm no value reaches is refused; a ``match`` that can fall through is
        refused.  Only then are the arm bodies compiled, each in the scope its
        pattern extends.  The arms are guarded by the scrutinee: a subject call
        in an arm needs a pure scrutinee, as under an ``if``.

        Cost is ``1 +`` the scrutinee ``+`` the **largest** arm (section 4.2
        rule 6); calls are those of the arm the pure scrutinee selects.
        """
        sem = self.sem
        arms_raw = node[2] if len(node) == 3 else None
        if (
            type(arms_raw) is not list
            or not arms_raw
            or len(arms_raw) > patterns.MAX_ARMS
        ):
            raise Refusal("ill-typed", "match takes a scrutinee and one or more arms")
        if not all(type(arm) is list and len(arm) == 2 for arm in arms_raw):
            raise Refusal("ill-typed", "a match arm is a [pattern, body] pair")
        scrutinee = self._compile(node[1], scope)
        if scrutinee.ty.head != "Result" or any(
            patterns.holds_data_pattern(arm[0]) for arm in arms_raw
        ):
            self.need_levels({"ir.data"}, "a match that is not on a Result")
            raise Refusal(
                "level-missing", "ir.data"
            )  # pragma: no cover - not implemented
        if any(
            patterns.pattern_depth(arm[0]) > patterns.MAX_PATTERN_DEPTH
            for arm in arms_raw
        ):
            raise Refusal("depth", "a pattern nested more than 8 levels")
        taken = set(scope)
        parsed: list[tuple[patterns.Pattern, dict[str, Ty]]] = []
        for pattern_raw, _body in arms_raw:
            bound: dict[str, Ty] = {}
            parsed.append(
                (
                    patterns.parse(
                        pattern_raw, scrutinee.ty, self.errors, taken, bound
                    ),
                    bound,
                )
            )
        patterns.check_arms([p for p, _b in parsed], scrutinee.ty, self.errors)
        arms = [
            self._under_guard(scrutinee, body, {**scope, **bound})
            for (_pattern, body), (_p, bound) in zip(arms_raw, parsed, strict=True)
        ]
        common = arms[0].ty
        for arm in arms[1:]:
            merged = self._branch_type(node, common, arm.ty)
            if merged is None:
                raise Refusal("ill-typed", "the arms of a match share no type")
            common = merged
        if scrutinee.subject and any(arm.subject for arm in arms):
            raise Refusal(
                "impure-guard",
                "a guard that calls the subject sits over a subject call",
            )
        sf = scrutinee.fn
        tests = [pattern for pattern, _b in parsed]
        bodies = [arm.fn for arm in arms]

        def select(env: dict) -> tuple[int, dict]:
            value = sf(env)
            found: dict[int, dict] = {}
            for place, pattern in enumerate(tests):
                binds: dict = {}
                if pattern.test(value, binds):
                    found[place] = binds
            if (
                not found
            ):  # pragma: no cover - the usefulness check proves exhaustiveness
                raise KitError("a match fell through")
            place = sem.choose_arm(list(found))
            return place, found[place]

        def run(env: dict) -> object:
            place, binds = select(env)
            return bodies[place]({**env, **binds})

        arm_calls = [arm.calls for arm in arms]
        if scrutinee.subject:
            calls: Calls = scrutinee.calls
        elif all(isinstance(c, int) for c in arm_calls) and len(set(arm_calls)) == 1:
            calls = arm_calls[0]
        else:

            def calls(env: dict) -> int:  # type: ignore[misc]
                place, binds = select(env)
                return calls_of(arm_calls[place], {**env, **binds})

        cost = 1 + scrutinee.cost + max(arm.cost for arm in arms)
        return self._combine(common, run, [scrutinee, *arms], calls=calls, cost=cost)

    def _call(self, node: list, scope: dict[str, Ty]) -> Compiled:
        tag = node[0]
        shapes = (3, 4) if tag == "subject_op" else (3,)
        if (
            len(node) not in shapes
            or type(node[2]) is not list
            or type(node[1]) is not str
        ):
            raise Refusal("ill-typed", f"malformed {tag}")
        name = node[1]
        args = [self._compile(arg, scope) for arg in node[2]]
        if tag == "subject_op":
            wanted = self._bound_signature(name, node)
        else:
            model = self.models.get(name)
            if model is None:  # pragma: no cover - the closure walk compiled it
                raise Refusal("model-order", "a model_call to an unknown model")
            wanted = ([t for _p, t in model.params], model.body.ty)
        params, result = wanted
        if len(args) != len(params) or not all(
            flows(a.ty, t) for a, t in zip(args, params, strict=True)
        ):
            raise Refusal("ill-typed", f"{name}: wrong argument count or types")
        fns = [a.fn for a in args]
        tainted = any(a.subject for a in args)
        blame_case = tainted or self._impure_guards > 0
        deepest = max((a.nest for a in args), default=0)
        frames = max((a.frames for a in args), default=0)
        if tag == "subject_op":

            def call_subject(env: dict) -> object:
                hook = self.subject_hook
                if hook is None:
                    raise KitError("a subject call with no subject bound")
                return hook(name, [f(env) for f in fns])

            return Compiled(
                result,
                call_subject,
                1 + sum(a.cost for a in args),
                True,
                frames,
                _plus(1, _sum_calls([a.calls for a in args])),
                1 + deepest,
            )
        names = [p for p, _t in model.params]

        def call_model(env: dict) -> object:
            inner = dict(zip(names, [f(env) for f in fns], strict=True))
            if not blame_case:
                return model.body.fn(inner)
            try:
                return model.body.fn(inner)
            except IrOverflow:
                # The model body has no subject beneath it, but its input came
                # from one: the overflow is the subject's doing, a case failure.
                raise IrOverflow("case", "an exact value left the window") from None

        return Compiled(
            result,
            call_model,
            1 + sum(a.cost for a in args) + model.body.cost,
            tainted,
            max(frames, 1 + model.body.frames),
            _sum_calls([a.calls for a in args]),
            1 + max(deepest, model.body.nest),
        )

    def _bound_signature(self, name: str, node: list) -> tuple[list[Ty], Ty]:
        """The parameter and result types of the op a ``subject_op`` calls (B2).

        The fourth element of the node is the list of **static widths**: one
        integer in ``0..=4096`` for each entry of the op's ``width_params``, in
        order, substituted into the signature before the arguments are typed.  A
        missing entry, an extra one, an entry out of range or not an integer, and
        a list given to an op that has no ``width_params`` are all ``ill-typed``.
        """
        sig = self.op(name)
        self.called_ops.add(name)
        given = node[3] if len(node) == 4 else None
        if len(node) == 4 and not sig.width_params:
            raise Refusal("ill-typed", f"{name} has no static widths to bind")
        if sig.width_params:
            if (
                type(given) is not list
                or len(given) != len(sig.width_params)
                or not all(type(w) is int and 0 <= w <= MAX_FIXED_WIDTH for w in given)
            ):
                raise Refusal(
                    "ill-typed",
                    f"{name} takes {len(sig.width_params)} static width(s) in 0..=4096",
                )
            params, result = sig.bind(given)
        else:
            params, result = sig.params, sig.ret
        return [t for _p, t in params], result


def _plus(constant: int, calls: Calls) -> Calls:
    """``constant + calls``, for a call count that may depend on the environment."""
    if isinstance(calls, int):
        return constant + calls
    return lambda env: constant + calls(env)


def _short_calls(
    left: Callable[[dict], object], short: Callable[[object], bool], right: Calls
) -> Calls:
    """Calls of ``left OP right`` with a pure ``left``: the right's unless it ends."""
    if isinstance(right, int) and right == 0:
        return 0
    return lambda env: 0 if short(left(env)) else calls_of(right, env)


# ---------------------------------------------------------------------------
# Running a compiled expression
# ---------------------------------------------------------------------------
class recursion_guard:  # noqa: N801 - used like a function, behaves like a context manager
    """Allow the evaluator's deepest legal nesting, and no more.

    The limits of section 14 allow 64 nodes on a path *and* 64 active
    ``model_call`` frames, so a legal corpus can nest 64 x 64 nodes, each a few
    Python calls deep: more than CPython's default recursion limit.  The guard
    raises the limit for the duration of an evaluation and puts it back.
    """

    LIMIT = 60_000

    def __enter__(self) -> None:
        self._saved = sys.getrecursionlimit()
        sys.setrecursionlimit(max(self._saved, self.LIMIT))

    def __exit__(self, *_exc: object) -> None:
        sys.setrecursionlimit(self._saved)


def evaluate(compiled: Compiled, env: dict) -> object:
    """Run ``compiled`` on ``env``; a stack overflow is ``depth``, never a crash."""
    with recursion_guard():
        try:
            return compiled.fn(env)
        except (
            RecursionError
        ) as error:  # pragma: no cover - the static limits prevent it
            raise Refusal(
                "depth", "evaluation recursed past the stack bound"
            ) from error


# Python frames one nesting level of closures can use (a short-circuit operator
# is a closure that calls a hook that calls a closure) and the room a caller's
# own stack may already occupy.  Both are generous.
_FRAMES_PER_LEVEL = 4
_CALLER_SLACK = 300


def guarded(fn: Callable[[dict], object], nest: int) -> Callable[[dict], object]:
    """``fn``, run with only the stack its closure nesting ``nest`` needs.

    The recursion guard (F14) belongs to the *evaluation of IR*, not to the loop
    around it: an enumerator that calls the evaluator a million times must not
    run its own bookkeeping, hashing and descriptor arithmetic under a limit
    raised for someone else.  An expression whose closures nest shallowly runs
    as itself, under the ordinary limit; only one that could overflow it is
    wrapped, and the guard then lasts for that one call.
    """
    if nest * _FRAMES_PER_LEVEL + _CALLER_SLACK <= sys.getrecursionlimit():
        return fn

    def deep(env: dict) -> object:
        with recursion_guard():
            return fn(env)

    return deep


def evaluator(compiled: Compiled) -> Callable[[dict], object]:
    """A function running ``compiled`` on an environment (see :func:`guarded`)."""
    return guarded(compiled.fn, compiled.nest)


@dataclass
class SubjectTable:
    """Subject dispatch and the call counter (section 4.2 rule 3, section 4.4).

    The lane binds each op of the corpus to the real export and *counts
    invocations* in its wrapper; the manifest's ``calls`` must equal the sum.  In
    the conformance kit the "real export" is a **stand-in model**: ``subjects``
    maps an op name to the pure model that plays it.  ``calls`` is therefore
    *measured* (every dispatch adds one) and not computed, which is what lets a
    wrong evaluator -- one that evaluates an untaken branch -- be caught.

    Only the ops the program's expressions actually call have stand-ins that
    matter; an entry for any other op is ignored, as an unused binding is: it is
    not compiled and its signature is not compared (R100, R124).  An op whose
    signature was merely *read* (the corpus check reads every op's) is not
    called.  A called op whose stand-in needs a missing level, or has the wrong
    signature, fails the kit build: a :class:`KitError`, never ``level-missing``.
    """

    program: Program
    subjects: dict[str, str]
    calls: int = field(default=0)

    def __post_init__(self) -> None:
        for op in sorted(self.program.called_ops):
            sig = self.program.reached_ops[op]
            model = self.subjects.get(op)
            if model is None:
                continue
            try:
                stand_in = self.program.model(model)
            except Refusal as refusal:
                raise KitError(
                    f"the stand-in {model} for {op} is refused: {refusal}"
                ) from refusal
            # A stand-in is written without a call site, so an op's named widths
            # read as plain Bytes there (R70).
            params, ret = sig.unbound()
            same = [t for _p, t in stand_in.params] == [t for _p, t in params]
            if stand_in.body.ty != ret:
                same = False
            if not same:
                raise KitError(
                    f"the stand-in {model} does not have the signature of {op}"
                )
        self.program.subject_hook = self

    def __call__(self, name: str, args: list) -> object:
        self.calls += 1
        model = self.program.models[self.subjects[name]]
        return model.body.fn(
            dict(zip([p for p, _t in model.params], args, strict=True))
        )
