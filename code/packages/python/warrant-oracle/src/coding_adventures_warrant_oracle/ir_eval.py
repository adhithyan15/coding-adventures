"""The model-IR evaluator at feature level ``ir.core`` (VER04r sections 3, 4 and 14).

What this module is
-------------------
An independent, *statically typed* evaluator for the language-neutral model IR.
It is compiled to Python closures once per expression and then run once per
case.  It never sees a subject: the oracle has no implementation under test, so
it evaluates only the *pure* parts of a claim (admission, the expected side of
a reference claim, and the guards used to count subject calls).

One wrapping rule (VER04r section 3.2) [D4]
-------------------------------------------
Every ``+ - *`` and unary minus on a fixed-width type wraps modulo ``2**w``, in
two's complement for signed types::

    255u8 + 1u8      = 0u8          0u8 - 1u8   = 255u8       16u8 * 16u8 = 0u8
    127i8 + 1i8      = -128i8       -(-128i8)   = -128i8
    MAX_U64 * MAX_U64 = 1

``Nat`` and ``Int`` are *exact*, but not unbounded: a value outside
``-(2**127-1)..2**127-1`` is the error ``ir-overflow`` -- never a wrapped or
saturated value.  (A Python ``int`` cannot overflow, so the check is explicit.)

Resource bounds (VER04r section 14)
-----------------------------------
* ``MAX_IR_DEPTH = 64``: IR nesting.
* ``MAX_IR_COST = 2000``: a *static* bound on node evaluations, charging each
  ``model_call`` its body's full cost.  The IR has no loops or recursion, so
  this is the whole fuel.  ``cost_model`` selects how an ``if`` is charged:
  ``"all-branches"`` (the VER04i / VER04r wording and the default: both
  branches) or ``"taken-path"`` (the larger branch only).  See the README for
  why the difference matters to ``gf256``.
* ``MAX_IR_VALUE = 65536``: no ``Bytes`` value an evaluator produces may exceed
  this, checked *before* the allocation (``concat`` of two 40,000-byte values is
  refused, it is not built and then measured).

Unknown tags are errors, never skipped (VER04r section 4.2 rule 7).  A tag that
belongs to a level this evaluator does not implement raises a ``level`` error.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from . import ast as A
from .errors import OracleError
from .lower import BOOL, BYTES, FIXED, INT, NAT, _substitute, _unify
from .lower import _assignable as _strict_assignable

MAX_IR_DEPTH = 64
MAX_IR_COST = 2000
MAX_IR_VALUE = 65_536
EXACT_BOUND = (1 << 127) - 1
MAX_MEMO = 1 << 20

CORE_TAGS = frozenset(
    [
        "lit",
        "var",
        "subject_op",
        "model_call",
        "len",
        "ok",
        "err",
        "eq",
        "ne",
        "lt",
        "gt",
        "le",
        "ge",
        "add",
        "sub",
        "mul",
        "and",
        "or",
        "implies",
        "not",
        "if",
        "concat",
        "take",
        "drop",
        "is_err",
        "neg",
    ]
)
#: Tags and built-ins of the levels above core: recognised so they can be
#: *named*, never evaluated (VER04r section 10.4).
_FIXED_TAGS = frozenset(
    [
        "wrap",
        "widen",
        "try_nat",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "div",
        "rem",
        "shl",
        "shr",
        "sar",
        "rotl",
        "rotr",
        "bit_and",
        "bit_or",
        "bit_xor",
        "bit_not",
        "popcount",
        "clz",
        "ctz",
        "be_bytes",
        "le_bytes",
        "from_be",
        "from_le",
        "at",
    ]
)
_DATA_TAGS = frozenset(["tuple", "project", "match", "some", "none", "ctor", "list"])
_SEQ_TAGS = frozenset(["fold", "map", "all", "any", "range", "count"])

_COMPARE = {"eq", "ne", "lt", "gt", "le", "ge"}
_ARITH = {"add", "sub", "mul"}


_UNKNOWN = A.Ty("?")


def _assignable(src: A.Ty, dst: A.Ty) -> bool:
    """Assignability that tolerates the unknown error type of a bare ``ok``/``err``.

    An ``["ok", e]`` node has no way to know its error type (the IR carries
    only the case name), so its ``Result`` is ``Result<T, ?>``: compatible with
    any ``Result<T', E>`` whose payload type accepts ``T``.
    """
    if src.head == dst.head == "Result":
        payload = src.args[0] == _UNKNOWN or _assignable(src.args[0], dst.args[0])
        return payload and (_UNKNOWN in (src.args[1], dst.args[1]) or src.args[1] == dst.args[1])
    return _strict_assignable(src, dst)


def _merge_result(a: A.Ty, b: A.Ty) -> A.Ty:
    """The more informative of two ``Result`` types (``?`` is the unknown)."""
    return b if a.args[0] == _UNKNOWN else a


@dataclass
class ModelDef:
    """A pure model as the evaluator sees it: typed parameters, return type, IR."""

    name: str
    params: list  # [(name, Ty)]
    ret: A.Ty
    ir: object


@dataclass
class Compiled:
    """A compiled expression: its static type, closure, cost and subject flag."""

    ty: A.Ty
    fn: Callable
    cost: int
    subject: bool


class Stats:
    """Counters the evaluator maintains (``wraps``, section 3.3)."""

    def __init__(self) -> None:
        self.wraps = 0


def wrap_to(value: int, bits: int, signed: bool) -> int:
    """Reduce ``value`` modulo ``2**bits`` and read it in the type's range."""
    r = value & ((1 << bits) - 1)
    if signed and r >= 1 << (bits - 1):
        r -= 1 << bits
    return r


def _check_exact(value: int) -> int:
    if value > EXACT_BOUND or value < -EXACT_BOUND:
        raise OracleError("ir-overflow", "an exact Nat/Int left the 128-bit window")
    return value


def decode_int_text(text: object, ty: str) -> int:
    """Decode a corpus decimal string for type ``ty`` (VER04r section 2.3).

    Digits only, one leading ``-`` for a *signed non-zero* value, no ``+``, no
    leading zero except ``"0"``, at most 20 digits, then range-checked.
    """
    if not isinstance(text, str):
        raise OracleError("ir", "an integer value is decimal text")
    body = text[1:] if text.startswith("-") else text
    if text.startswith("-") and (ty not in ("Int",) and (ty not in FIXED or not FIXED[ty][1])):
        raise OracleError("ir", "a negative value for an unsigned type")
    if not body or not body.isascii() or not body.isdigit() or len(body) > 20 + (ty == "Int") * 19:
        raise OracleError("ir", "malformed integer text")
    if len(body) > 1 and body[0] == "0":
        raise OracleError("ir", "a leading zero in an integer")
    if text == "-0":
        raise OracleError("ir", "negative zero")
    value = int(text)
    if ty in FIXED:
        bits, signed = FIXED[ty]
        low, high = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)
        if not low <= value <= high:
            raise OracleError("ir", "an integer value is out of range for its type")
    elif ty == "Nat":
        if not 0 <= value < 1 << 64:
            raise OracleError("ir", "a Nat value is in 0..2^64-1")
    elif ty == "Int":
        _check_exact(value)
    else:
        raise OracleError("ir", "not an integer type")
    return value


class Program:
    """A set of models and subject-op signatures against which IR is compiled."""

    def __init__(
        self,
        models: dict[str, ModelDef],
        ops: dict,
        *,
        cost_model: str = "all-branches",
        stats: Stats | None = None,
        memo: bool = True,
        max_cost: int = MAX_IR_COST,
    ) -> None:
        if cost_model not in ("all-branches", "taken-path"):
            raise ValueError("unknown cost model")
        self.models = models
        self.ops = ops
        self.cost_model = cost_model
        #: The spec value is 2,000.  A larger value is a *diagnostic* setting: it
        #: lets the oracle evaluate a document the spec bound would refuse (gf256's
        #: ``ginv`` costs far more than 2,000) so the finding can be measured.
        self.max_cost = max_cost
        self.stats = stats or Stats()
        self._compiled: dict[str, tuple[Compiled, list[str]]] = {}
        self._in_progress: set[str] = set()
        self._order: list[str] = list(models)
        self._ctx: int | None = None  # index of the model being compiled
        self._memo_on = memo
        self._memo: dict[str, dict] = {}
        self._memo_size = 0

    # -- public ----------------------------------------------------------------

    def compile(self, ir: object, tenv: dict[str, A.Ty], allow_subject: bool = True) -> Compiled:
        """Compile a top-level expression and enforce ``MAX_IR_COST`` on it."""
        c = self._compile(ir, tenv, allow_subject, 0)
        if c.cost > self.max_cost:
            raise OracleError("limit", "ir-cost: expression exceeds MAX_IR_COST")
        return c

    def model(self, name: str) -> Compiled:
        """Compile (once) the body of model ``name``."""
        return self._model(name)[0]

    # -- models ------------------------------------------------------------------

    def _model(self, name: str) -> tuple[Compiled, list[str]]:
        if name in self._compiled:
            return self._compiled[name]
        if name not in self.models:
            raise OracleError("ir", "a call to an unknown model")
        if name in self._in_progress:
            raise OracleError("ir", "a model calls itself")
        m = self.models[name]
        # Rule 4.2.1: a model may call only models declared *before* it.
        self._in_progress.add(name)
        saved, self._ctx = self._ctx, self._order.index(name)
        try:
            body = self._compile(m.ir, dict(m.params), False, 0)
        finally:
            self._ctx = saved
            self._in_progress.discard(name)
        if not _assignable(body.ty, m.ret):
            raise OracleError("ir", "a model body does not have its declared type")
        if body.cost > self.max_cost:
            raise OracleError("limit", "ir-cost: model exceeds MAX_IR_COST")
        entry = (body, [p for p, _ in m.params])
        self._compiled[name] = entry
        return entry

    # -- expressions ---------------------------------------------------------------

    def _compile(self, node: object, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        if depth > MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            raise OracleError("ir", "a malformed IR node")
        tag = node[0]
        sub = lambda child: self._compile(child, tenv, allow_subject, depth + 1)  # noqa: E731
        if tag not in CORE_TAGS:
            if tag in _FIXED_TAGS:
                raise OracleError("level", "a tag of level ir.fixed", level="ir.fixed")
            if tag in _DATA_TAGS:
                raise OracleError("level", "a tag of level ir.data", level="ir.data")
            if tag in _SEQ_TAGS:
                raise OracleError("level", "a tag of level ir.seq", level="ir.seq")
            raise OracleError("ir", "an unknown IR tag")
        if tag == "lit":
            return self._lit(node)
        if tag == "var":
            if len(node) != 2 or not isinstance(node[1], str) or node[1] not in tenv:
                raise OracleError("ir", "an unbound IR variable")
            name = node[1]
            return Compiled(tenv[name], lambda env: env[name], 1, False)
        if tag == "subject_op":
            return self._subject(node, tenv, allow_subject, depth)
        if tag == "model_call":
            return self._model_call(node, tenv, allow_subject, depth)
        if tag == "err":
            if len(node) != 2 or not isinstance(node[1], str):
                raise OracleError("ir", "a malformed err node")
            case = node[1]
            ty = A.Ty("Result", (_UNKNOWN, _UNKNOWN))
            return Compiled(ty, lambda env: ("err", case), 1, False)
        arity = {"len": 1, "ok": 1, "not": 1, "neg": 1, "is_err": 1, "if": 3, "take": 2, "drop": 2}
        n = arity.get(tag, 2)
        if len(node) != n + 1:
            raise OracleError("ir", "a malformed IR node")
        kids = [sub(child) for child in node[1:]]
        cost = 1 + sum(k.cost for k in kids)
        subj = any(k.subject for k in kids)
        return self._node(tag, kids, cost, subj)

    def _lit(self, node: list) -> Compiled:
        if len(node) != 2:
            raise OracleError("ir", "a malformed literal")
        payload = node[1]
        if isinstance(payload, bool):
            return Compiled(BOOL, lambda env: payload, 1, False)
        if isinstance(payload, dict) and set(payload) == {"type", "value"}:
            tname = payload["type"]
            if tname == "Bytes":
                text = payload["value"]
                if (
                    not isinstance(text, str)
                    or len(text) % 2
                    or any(c not in "0123456789abcdef" for c in text)
                ):
                    raise OracleError("ir", "a Bytes literal is even lowercase hex")
                if len(text) // 2 > MAX_IR_VALUE:
                    raise OracleError("ir-overflow", "a Bytes literal exceeds MAX_IR_VALUE")
                data = bytes.fromhex(text)
                # A literal's width is its length, so it can flow to a ``Bytes<N>``.
                return Compiled(A.Ty("Bytes", (len(data),)), lambda env: data, 1, False)
            if isinstance(tname, str) and (tname in FIXED or tname in ("Nat", "Int")):
                value = decode_int_text(payload["value"], tname)
                return Compiled(A.Ty(tname), lambda env: value, 1, False)
        raise OracleError("ir", "a malformed literal")

    def _subject(self, node: list, tenv: dict, allow: bool, depth: int) -> Compiled:
        if not allow:
            raise OracleError("ir", "a subject call is forbidden here")
        if len(node) not in (3, 4) or not isinstance(node[1], str) or not isinstance(node[2], list):
            raise OracleError("ir", "a malformed subject_op")
        op = self.ops.get(node[1])
        if op is None or len(node[2]) != len(op.params):
            raise OracleError("ir", "an unknown subject op or wrong arity")
        statics: dict = {}
        if len(node) == 4:
            widths = node[3]
            if not isinstance(widths, list) or len(widths) != len(op.statics):
                raise OracleError("ir", "wrong number of static widths")
            for sname, w in zip(op.statics, widths):
                if type(w) is not int or w < 0 or w > 4096:
                    raise OracleError("ir", "a bad static width")
                statics[sname] = w
        elif op.statics:
            raise OracleError("ir", "a missing static width")
        kids = [self._compile(a, tenv, True, depth + 1) for a in node[2]]
        for k, (_, pty) in zip(kids, op.params):
            if not _assignable(k.ty, _substitute(pty, statics)):
                raise OracleError("ir", "a subject argument has the wrong type")
        ret = _substitute(op.ret, statics)

        def boom(env: dict) -> object:
            raise OracleError("ir", "the oracle never calls a subject")

        return Compiled(ret, boom, 1 + sum(k.cost for k in kids), True)

    def _model_call(self, node: list, tenv: dict, allow: bool, depth: int) -> Compiled:
        if len(node) != 3 or not isinstance(node[1], str) or not isinstance(node[2], list):
            raise OracleError("ir", "a malformed model_call")
        name = node[1]
        if name not in self.models:
            raise OracleError("ir", "a call to an unknown model")
        if self._ctx is not None and self._order.index(name) >= self._ctx:
            raise OracleError("ir", "a model calls itself or a later model (rule 4.2.1)")
        body, pnames = self._model(name)
        m = self.models[name]
        if len(node[2]) != len(m.params):
            raise OracleError("ir", "a model call has the wrong arity")
        kids = [self._compile(a, tenv, allow, depth + 1) for a in node[2]]
        for k, (_, pty) in zip(kids, m.params):
            if not _assignable(k.ty, pty):
                raise OracleError("ir", "a model argument has the wrong type")
        if any(k.subject for k in kids) and not allow:
            raise OracleError("ir", "a subject call inside a model")
        fns = [k.fn for k in kids]
        body_fn = body.fn
        memo_ok = self._memo_on and all(
            p.head in FIXED or p.head in ("Bool", "Nat", "Int", "Bytes") for _, p in m.params
        )
        table = self._memo.setdefault(name, {})
        prog = self

        def call(env: dict) -> object:
            args = tuple(f(env) for f in fns)
            if memo_ok:
                hit = table.get(args, table)
                if hit is not table:
                    return hit
            value = body_fn(dict(zip(pnames, args)))
            if memo_ok and prog._memo_size < MAX_MEMO:
                table[args] = value
                prog._memo_size += 1
            return value

        cost = 1 + sum(k.cost for k in kids) + body.cost
        return Compiled(m.ret, call, cost, any(k.subject for k in kids))

    # -- the operators -------------------------------------------------------------

    def _node(self, tag: str, kids: list[Compiled], cost: int, subj: bool) -> Compiled:
        stats = self.stats
        if tag == "len":
            (a,) = kids
            if a.ty.head != "Bytes":
                raise OracleError("ir", "len needs Bytes at ir.core")
            af = a.fn
            return Compiled(NAT, lambda env: len(af(env)), cost, subj)
        if tag == "ok":
            (a,) = kids
            af = a.fn
            return Compiled(
                A.Ty("Result", (a.ty, _UNKNOWN)), lambda env: ("ok", af(env)), cost, subj
            )
        if tag == "not":
            (a,) = kids
            if a.ty != BOOL:
                raise OracleError("ir", "not needs Bool")
            af = a.fn
            return Compiled(BOOL, lambda env: not af(env), cost, subj)
        if tag == "is_err":
            (a,) = kids
            if a.ty.head != "Result":
                raise OracleError("ir", "is_err needs a Result")
            af = a.fn
            return Compiled(BOOL, lambda env: af(env)[0] == "err", cost, subj)
        if tag == "neg":
            (a,) = kids
            if a.ty.head in FIXED:
                bits, signed = FIXED[a.ty.head]
                af = a.fn

                def neg_fixed(env: dict) -> int:
                    x = af(env)
                    r = wrap_to(-x, bits, signed)
                    if r != -x:
                        stats.wraps += 1
                    return r

                return Compiled(a.ty, neg_fixed, cost, subj)
            if a.ty.head in ("Nat", "Int"):
                af = a.fn
                return Compiled(INT, lambda env: _check_exact(-af(env)), cost, subj)
            raise OracleError("ir", "neg needs a number")
        if tag in ("and", "or", "implies"):
            a, b = kids
            if a.ty != BOOL or b.ty != BOOL:
                raise OracleError("ir", "a logical operator needs Bool operands")
            af, bf = a.fn, b.fn
            if tag == "and":
                return Compiled(BOOL, lambda env: af(env) and bf(env), cost, subj)
            if tag == "or":
                return Compiled(BOOL, lambda env: af(env) or bf(env), cost, subj)
            return Compiled(BOOL, lambda env: (not af(env)) or bf(env), cost, subj)
        if tag == "if":
            c, t, e = kids
            if c.ty != BOOL:
                raise OracleError("ir", "an if condition is Bool")
            if t.ty.head == "Result" and e.ty.head == "Result":
                common = _merge_result(t.ty, e.ty)
            else:
                common = _unify(t.ty, e.ty)
            if common is None:
                raise OracleError("ir", "the branches of an if have no common type")
            cf, tf, ef = c.fn, t.fn, e.fn
            if self.cost_model == "taken-path":
                cost = 1 + c.cost + max(t.cost, e.cost)
            return Compiled(common, lambda env: tf(env) if cf(env) else ef(env), cost, subj)
        if tag in ("concat", "take", "drop"):
            return self._bytes_op(tag, kids, cost, subj)
        if tag in _COMPARE:
            return self._compare(tag, kids, cost, subj)
        if tag in _ARITH:
            return self._arith(tag, kids, cost, subj)
        raise OracleError("ir", "an unknown IR tag")  # pragma: no cover

    def _bytes_op(self, tag: str, kids: list[Compiled], cost: int, subj: bool) -> Compiled:
        if tag == "concat":
            a, b = kids
            if a.ty.head != "Bytes" or b.ty.head != "Bytes":
                raise OracleError("ir", "concat needs Bytes")
            af, bf = a.fn, b.fn

            def concat(env: dict) -> bytes:
                x, y = af(env), bf(env)
                if len(x) + len(y) > MAX_IR_VALUE:
                    raise OracleError("ir-overflow", "a Bytes value exceeds MAX_IR_VALUE")
                return x + y

            widths = [t.ty.args[0] for t in (a, b) if t.ty.args and isinstance(t.ty.args[0], int)]
            out_ty = A.Ty("Bytes", (sum(widths),)) if len(widths) == 2 else BYTES
            return Compiled(out_ty, concat, cost, subj)
        a, n = kids
        if a.ty.head != "Bytes" or n.ty.head not in ("Nat", "Int"):
            raise OracleError("ir", "take/drop need Bytes and a count")
        af, nf = a.fn, n.fn
        if tag == "take":
            return Compiled(BYTES, lambda env: af(env)[: max(nf(env), 0)], cost, subj)
        return Compiled(BYTES, lambda env: af(env)[max(nf(env), 0) :], cost, subj)

    def _compare(self, tag: str, kids: list[Compiled], cost: int, subj: bool) -> Compiled:
        a, b = kids
        af, bf = a.fn, b.fn
        if tag in ("eq", "ne"):
            if a.ty.head == "Result" or b.ty.head == "Result":
                if a.ty.head != b.ty.head:
                    raise OracleError("ir", "comparing a Result with a non-Result")
            elif _unify(a.ty, b.ty) is None:
                raise OracleError("ir", "equality needs operands of one type")
            if tag == "eq":
                return Compiled(BOOL, lambda env: af(env) == bf(env), cost, subj)
            return Compiled(BOOL, lambda env: af(env) != bf(env), cost, subj)
        numeric = lambda t: t.head in FIXED or t.head in ("Nat", "Int")  # noqa: E731
        if not (numeric(a.ty) and numeric(b.ty)) or (
            (a.ty.head in FIXED or b.ty.head in FIXED) and a.ty != b.ty
        ):
            raise OracleError("ir", "an ordering needs two numbers of one kind")
        if tag == "lt":
            return Compiled(BOOL, lambda env: af(env) < bf(env), cost, subj)
        if tag == "le":
            return Compiled(BOOL, lambda env: af(env) <= bf(env), cost, subj)
        if tag == "gt":
            return Compiled(BOOL, lambda env: af(env) > bf(env), cost, subj)
        return Compiled(BOOL, lambda env: af(env) >= bf(env), cost, subj)

    def _arith(self, tag: str, kids: list[Compiled], cost: int, subj: bool) -> Compiled:
        a, b = kids
        af, bf = a.fn, b.fn
        stats = self.stats
        exact = {
            "add": lambda x, y: x + y,
            "sub": lambda x, y: x - y,
            "mul": lambda x, y: x * y,
        }[tag]
        if a.ty.head in FIXED or b.ty.head in FIXED:
            if a.ty != b.ty:
                raise OracleError("ir", "a fixed-width operation needs operands of one type")
            bits, signed = FIXED[a.ty.head]
            mask = (1 << bits) - 1
            half = 1 << (bits - 1)
            span = 1 << bits

            def fixed(env: dict) -> int:
                x = exact(af(env), bf(env))
                r = x & mask
                if signed and r >= half:
                    r -= span
                if r != x:
                    stats.wraps += 1
                return r

            return Compiled(a.ty, fixed, cost, subj)
        if a.ty.head not in ("Nat", "Int") or b.ty.head not in ("Nat", "Int"):
            raise OracleError("ir", "arithmetic needs numbers")
        return Compiled(INT, lambda env: _check_exact(exact(af(env), bf(env))), cost, subj)


# -- call counting (VER04r section 4.4) ------------------------------------------


def has_subject(ir: object) -> bool:
    """Does the IR tree contain a ``subject_op`` node?"""
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        tag = node[0]
        if tag == "subject_op":
            return True
        if tag in ("lit", "var", "err"):
            continue
        if tag == "model_call":
            stack.extend(node[2])
            continue
        stack.extend(node[1:])
    return False


def compile_calls(prog: Program, ir: object, tenv: dict) -> int | Callable:
    """The number of subject calls per case, as ``int`` (static) or ``fn(env)``.

    The count is taken on the *pure trace* (VER04r section 4.4): a subject call
    may sit under a guard only if the guard's condition is pure, so that the
    count never depends on what a subject returned.  Anything else is the
    generation error ``W0701``.
    """

    def count(node: object, depth: int = 0) -> int | Callable:
        if depth > MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            return 0
        tag = node[0]
        if tag == "subject_op":
            parts = [count(a, depth + 1) for a in node[2]]
            return _sum(1, parts)
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                if has_subject(cond):
                    raise OracleError("generation", "W0701: a subject call under an impure guard")
                cf = prog.compile(cond, tenv, False).fn
                tc, ec = count(then, depth + 1), count(other, depth + 1)
                return lambda env: (
                    (tc(env) if callable(tc) else tc)
                    if cf(env)
                    else (ec(env) if callable(ec) else ec)
                )
            return count(cond, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                if has_subject(left):
                    raise OracleError("generation", "W0701: a subject call under an impure guard")
                lf = prog.compile(left, tenv, False).fn
                rc = count(right, depth + 1)
                # ``and`` and ``implies`` evaluate the right operand when the left is true
                # (implies: when ``not left`` is false), ``or`` when it is false.
                runs_when = (lambda v: not v) if tag == "or" else (lambda v: bool(v))
                return lambda env: (rc(env) if callable(rc) else rc) if runs_when(lf(env)) else 0
            return count(left, depth + 1)
        children = [count(c, depth + 1) for c in node[1:] if isinstance(c, list)]
        return _sum(0, children)

    return count(ir)


def _sum(base: int, parts: list) -> int | Callable:
    static = base + sum(p for p in parts if not callable(p))
    dynamic = [p for p in parts if callable(p)]
    if not dynamic:
        return static
    return lambda env: static + sum(p(env) for p in dynamic)
