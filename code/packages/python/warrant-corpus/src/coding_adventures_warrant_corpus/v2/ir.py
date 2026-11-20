"""The ``ir.core`` evaluator (VER04r section 4 and Appendix A).

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

The cost bound is the whole "fuel": the IR has no loop and no recursion, so a
static worst-path count of node evaluations is a sound bound and no run-time
meter exists.  An ``if`` costs its condition plus the *larger* branch (an
evaluator that visits more nodes than that is nonconforming), a ``model_call``
costs its arguments plus its body's cost, and a short circuit is never credited.
"""

from __future__ import annotations

import operator
import sys
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field

from .arith import WindowExceeded, raw_binary
from .errors import IrOverflow, KitError, Refusal
from .semantics import STANDARD, Semantics
from .values import decode_value
from .wtypes import (
    BOOL,
    BYTES,
    INT,
    NAT,
    UNKNOWN_TY,
    Ty,
    flows,
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

IMPLEMENTED_LEVELS = frozenset({"ir.core"})


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


def walk(root: object) -> Iterator[tuple[list, int]]:
    """Yield ``(node, depth)`` for every IR node under ``root``, pre-order, iteratively.

    A node's *depth* is the number of nodes on the path from the root to it, the
    root being 1 (a leaf alone is one level).  An argument array such as the
    ``[a, b]`` of a ``model_call`` is not a node and adds no depth.  Iterative on
    purpose: a hostile corpus may nest far deeper than Python's recursion limit
    and must be *refused*, not crash the runner.
    """
    stack: list[tuple[object, int]] = [(root, 1)]
    while stack:
        item, depth = stack.pop()
        if is_node(item):
            yield item, depth  # type: ignore[misc]
            children = item[1:]  # type: ignore[index]
            stack.extend((child, depth + 1) for child in reversed(children))
        elif type(item) is list:
            stack.extend((child, depth) for child in reversed(item))


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
    a constant, or a function of the environment when a pure guard decides).
    """

    ty: Ty
    fn: Callable[[dict], object]
    cost: int
    subject: bool = False
    frames: int = 0
    calls: Calls = 0


@dataclass
class ModelDef:
    """A compiled model: its parameter names and types and its body."""

    params: list[tuple[str, Ty]]
    body: Compiled


@dataclass
class OpSig:
    """An operation signature from a section 6 ``ops`` row."""

    params: list[tuple[str, Ty]]
    ret: Ty


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


class Program:
    """The models and ops of one corpus (or one conformance case), compiled.

    ``levels`` are the levels the evaluator is configured with; only those this
    runner :data:`IMPLEMENTED_LEVELS` take effect, so a case that names a level
    the runner lacks sees ``level-missing`` for that level's tags.  Models are
    checked and compiled in ``model_order``; everything about *all* of them
    (depth, tags, subject calls, call order) is verified before any is compiled.
    """

    def __init__(
        self,
        *,
        types: dict | None = None,
        ops: dict | None = None,
        models: dict | None = None,
        model_order: list | None = None,
        levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
        max_ir_cost: object = MAX_IR_COST,
        sem: Semantics = STANDARD,
    ) -> None:
        self.types = types or {}
        self.sem = sem
        self.levels = frozenset(levels) & IMPLEMENTED_LEVELS
        if (
            type(max_ir_cost) is not int
            or not MAX_IR_COST <= max_ir_cost <= MAX_IR_COST_CEILING
        ):
            raise Refusal("ir-cost", "max_ir_cost is outside 2,000..=65,536")
        self.max_cost = max_ir_cost
        self.subject_hook: Callable[[str, list], object] | None = None
        self._raw_models = models or {}
        self._order = list(model_order or [])
        self.ops: dict[str, OpSig] = {}
        self.models: dict[str, ModelDef] = {}
        self.errors: dict[str, tuple[str, ...]] = {}
        try:
            self.errors = {
                name: tuple(row["cases"])
                for name, row in self.types.items()
                if row.get("kind") == "error"
            }
            self._build(ops or {})
        except (KeyError, TypeError, IndexError, ValueError, AttributeError) as error:
            # A row of the wrong shape is a corpus defect, never a crash.
            raise Refusal(
                "ill-typed", "a type, model or op row has the wrong shape"
            ) from error

    # -- construction ---------------------------------------------------------
    def _type(self, text: object) -> Ty:
        """A declared type of an op or model signature; any refusal is ``ill-typed``."""
        try:
            ty = parse_type(text, self.types)
        except Refusal as error:
            raise Refusal("ill-typed", f"bad declared type: {error.message}") from error
        self._need_levels(type_level(ty, self.types), str(ty))
        return ty

    def _need_levels(self, needed: set[str], what: str) -> None:
        missing = needed - self.levels
        if missing:
            raise Refusal("level-missing", f"{what} needs {sorted(missing)[0]}")

    def _build(self, ops: dict) -> None:
        raw = self._raw_models
        for name in self._order:
            if name not in raw:
                raise Refusal(
                    "model-order", f"model_order names the missing model {name}"
                )
        # Static checks over EVERY model body before any is compiled.
        for position, name in enumerate(self._order):
            self.scan(raw[name]["body"], owner=position)
        self.ops = {
            name: OpSig(
                [(p, self._type(t)) for p, t in row["params"]], self._type(row["ret"])
            )
            for name, row in ops.items()
        }
        for name in self._order:
            row = raw[name]
            scope = {p: self._type(t) for p, t in row["params"]}
            body = self._compile(row["body"], scope)
            self.models[name] = ModelDef(list(scope.items()), body)

    # -- the static scans -------------------------------------------------------
    def scan(self, root: object, *, owner: int | None, role: str = "body") -> None:
        """Depth, tags, subject placement and call order of one expression.

        ``owner`` is the position of the model whose body this is in
        ``model_order``, or ``None`` for a top-level expression (which comes
        after every model).  All four checks are structural: they look at tags,
        never at types, and they look at *every* node including the ones an
        evaluation would never reach, because an untaken branch is still part of
        the corpus.
        """
        nodes = []
        for node, depth in walk(root):
            if depth > MAX_DEPTH:
                raise Refusal(
                    "depth", f"an expression nested more than {MAX_DEPTH} deep"
                )
            nodes.append(node)
        for node in nodes:
            tag = node[0]
            level = tag_level(tag)
            if level is None:
                raise Refusal("unknown-tag", tag)
            if level not in self.levels:
                raise Refusal("level-missing", tag)
        for node in nodes:
            if node[0] == "subject_op" and (owner is not None or role != "body"):
                raise Refusal(
                    "subject-forbidden", "a subject call outside a claim body"
                )
            if node[0] == "model_call":
                callee = node[1] if len(node) > 1 else None
                here = len(self._order) if owner is None else owner
                if (
                    not isinstance(callee, str)
                    or callee not in self._raw_models
                    or callee not in self._order
                    or self._order.index(callee) >= here
                ):
                    raise Refusal(
                        "model-order", "a model_call to a model not declared earlier"
                    )

    # -- compiling ---------------------------------------------------------------
    def compile(self, ir: object, scope: dict[str, Ty], role: str = "body") -> Compiled:
        """Check and compile a top-level expression over the variables in ``scope``.

        ``role`` is ``"body"`` (subject calls allowed) or any other word for an
        ``admit``, ``expected`` or fault expression, where a subject call is
        ``subject-forbidden`` (section 4.2 rule 3).
        """
        self.scan(ir, owner=None, role=role)
        return self._compile(ir, scope)

    def _compile(self, node: object, scope: dict[str, Ty]) -> Compiled:
        try:
            compiled = self._node(node, scope)
        except RecursionError as error:  # pragma: no cover - depth is capped at 64
            raise Refusal("depth", "the expression is too deep to compile") from error
        if compiled.cost > self.max_cost:
            raise Refusal(
                "ir-cost", f"worst-path cost {compiled.cost} exceeds {self.max_cost}"
            )
        if compiled.frames > MAX_FRAMES:
            raise Refusal("depth", "more than 64 active model_call frames")
        return compiled

    def _kids(self, node: list, scope: dict[str, Ty], count: int) -> list[Compiled]:
        """Compile the ``count`` operands of ``node`` (positions 1..count)."""
        if len(node) != count + 1:
            raise Refusal("ill-typed", f"{node[0]} takes {count} operand(s)")
        return [self._compile(child, scope) for child in node[1:]]

    def _node(self, node: object, scope: dict[str, Ty]) -> Compiled:
        if not is_node(node):
            raise Refusal("ill-typed", "an IR node is a list whose first item is a tag")
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
        left, right = self._kids(node, scope, 2)
        if left.ty != BOOL or right.ty != BOOL:
            raise Refusal("ill-typed", f"{tag} takes two Bool operands")
        if left.subject and right.subject:
            raise Refusal(
                "impure-guard", "a subject call under a guard that calls the subject"
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
        if len(node) == 2 and type(node[1]) is bool:
            value: object = node[1]
            return Compiled(BOOL, lambda _env: value, 1)
        spec = node[1] if len(node) == 2 else None
        if type(spec) is not dict or set(spec) != {"type", "value"}:
            raise Refusal("ill-typed", "a literal is true, false or {type, value}")
        try:
            ty = parse_type(spec["type"], self.types, allow_int=True)
            if not (ty.is_number or ty.head in ("Bool", "Bytes")):
                raise Refusal(
                    "ill-typed", "a literal of this type does not exist at ir.core"
                )
            if ty.head == "Int":
                value = self.sem.decode_int(spec["value"], ty)
            else:
                value = decode_value(spec["value"], ty, self.types, self.sem)
        except Refusal as error:
            raise Refusal("ill-typed", f"bad literal: {error.message}") from error
        return Compiled(ty, lambda _env: value, 1)

    def _err(self, node: list) -> Compiled:
        if len(node) != 2 or not isinstance(node[1], str):
            raise Refusal("ill-typed", "err takes the name of an error case")
        case = node[1]
        owners = [n for n, cases in self.errors.items() if case in cases]
        if not owners:
            raise Refusal("ill-typed", "an err case that no error type declares")
        error = Ty(owners[0]) if len(owners) == 1 else UNKNOWN_TY
        value = ("err", case)
        return Compiled(Ty("Result", (UNKNOWN_TY, error)), lambda _env: value, 1)

    def _unary(self, tag: str, arg: Compiled) -> Compiled:
        sem, f, klass = self.sem, arg.fn, "case" if arg.subject else "corpus"
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
        cond, then, other = self._kids(node, scope, 3)
        if cond.ty != BOOL:
            raise Refusal("ill-typed", "an if condition is Bool")
        common = unify(then.ty, other.ty)
        if common is None:
            raise Refusal("ill-typed", "the branches of an if have no common type")
        if cond.subject and (then.subject or other.subject):
            raise Refusal(
                "impure-guard", "a subject call under a guard that calls the subject"
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
        klass = "case" if (left.subject or right.subject) else "corpus"
        a, b = left.ty, right.ty
        if tag in ("eq", "ne"):
            if unify(a, b) is None:
                raise Refusal("ill-typed", "equality needs operands of one type")
            if tag == "eq":
                return self._combine(
                    BOOL, lambda env: lf(env) == rf(env), [left, right]
                )
            return self._combine(BOOL, lambda env: lf(env) != rf(env), [left, right])
        if tag in ("lt", "gt", "le", "ge"):
            if not (a.is_number and b.is_number) or (
                (a.is_fixed or b.is_fixed) and a != b
            ):
                raise Refusal("ill-typed", "an ordering needs two numbers of one kind")
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

    def _call(self, node: list, scope: dict[str, Ty]) -> Compiled:
        tag = node[0]
        if tag == "subject_op":
            ok_shape = len(node) in (3, 4) and type(node[2]) is list
        else:
            ok_shape = len(node) == 3 and type(node[2]) is list
        if not ok_shape or not isinstance(node[1], str):
            raise Refusal("ill-typed", f"malformed {tag}")
        name = node[1]
        args = [self._compile(arg, scope) for arg in node[2]]
        if tag == "subject_op":
            if name not in self.ops:
                raise Refusal("ill-typed", "a subject call to an op that is not in ops")
            sig = self.ops[name]
            if len(node) == 4:
                widths = node[3]
                if type(widths) is not list or not all(type(w) is int for w in widths):
                    raise Refusal("ill-typed", "static widths are a list of integers")
        else:
            sig = self.models.get(name)  # type: ignore[assignment]
            if sig is None:  # pragma: no cover - model-order refused it already
                raise Refusal("model-order", "a model_call to an unknown model")
        params = [t for _p, t in sig.params]
        if len(args) != len(params) or not all(
            flows(a.ty, t) for a, t in zip(args, params, strict=True)
        ):
            raise Refusal("ill-typed", f"{name}: wrong argument count or types")
        fns = [a.fn for a in args]
        if tag == "subject_op":

            def call_subject(env: dict) -> object:
                hook = self.subject_hook
                if hook is None:
                    raise KitError("a subject call with no subject bound")
                return hook(name, [f(env) for f in fns])

            return Compiled(
                sig.ret,  # type: ignore[union-attr]
                call_subject,
                1 + sum(a.cost for a in args),
                True,
                max((a.frames for a in args), default=0),
                _plus(1, _sum_calls([a.calls for a in args])),
            )
        model: ModelDef = sig  # type: ignore[assignment]
        names = [p for p, _t in model.params]
        tainted = any(a.subject for a in args)

        def call_model(env: dict) -> object:
            inner = dict(zip(names, [f(env) for f in fns], strict=True))
            if not tainted:
                return model.body.fn(inner)
            try:
                return model.body.fn(inner)
            except IrOverflow:
                # The model body has no subject beneath it, but its input came
                # from one: the overflow is the subject's doing, a case failure.
                raise IrOverflow("case", "an exact value left the window") from None

        return Compiled(
            model.body.ty,
            call_model,
            1 + sum(a.cost for a in args) + model.body.cost,
            tainted,
            max(max((a.frames for a in args), default=0), 1 + model.body.frames),
            _sum_calls([a.calls for a in args]),
        )


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
            raise Refusal("depth", "evaluation nested too deeply") from error


@dataclass
class SubjectTable:
    """Subject dispatch and the call counter (section 4.2 rule 3, section 4.4).

    The lane binds each op of the corpus to the real export and *counts
    invocations* in its wrapper; the manifest's ``calls`` must equal the sum.  In
    the conformance kit the "real export" is a **stand-in model**: ``subjects``
    maps an op name to the pure model that plays it.  ``calls`` is therefore
    *measured* (every dispatch adds one) and not computed, which is what lets a
    wrong evaluator -- one that evaluates an untaken branch -- be caught.
    """

    program: Program
    subjects: dict[str, str]
    calls: int = field(default=0)

    def __post_init__(self) -> None:
        for op, model in self.subjects.items():
            sig, stand_in = self.program.ops.get(op), self.program.models.get(model)
            if sig is None or stand_in is None:
                continue  # an unused entry; a call to a missing op is ill-typed anyway
            same = [t for _p, t in stand_in.params] == [t for _p, t in sig.params]
            if not same or stand_in.body.ty != sig.ret:
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
