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
* ``MAX_IR_DEPTH = 64``: IR nesting, counted in *levels* (the root is level 1, so a node
  at depth index 64 is the refused 65th level; S4 part 1 admitted 65).
* ``MAX_IR_COST = 2000`` (default; a corpus may declare up to 65,536 in
  ``corpus.profile``): a *static* bound on node evaluations on the **worst path**
  (VER04r 4.2 rule 6, as amended): one branch of an ``if`` (the larger), each
  ``model_call`` at its body's cost, memoisation never credited.  The IR has no
  loops or recursion, so this is the whole fuel.  There is no override other
  than the profile line.
* ``MAX_MODEL_DEPTH = 64``: nested model calls (VER04 section 2: 64 active frames).
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
MAX_MODEL_DEPTH = 64
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
    """Counters the evaluator maintains (``wraps``, section 3.3).

    ``wraps`` only moves while ``counting`` is true: the oracle runs one dedicated
    counting pass per suite (with its own memo table, which stores each pure model
    call's wrap count and replays it on a hit), so the total never depends on
    memoisation, suite order or the depth of the run.
    """

    def __init__(self) -> None:
        self.wraps = 0
        self.counting = False


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
        stats: Stats | None = None,
        memo: bool = True,
        max_cost: int = MAX_IR_COST,
    ) -> None:
        self.models = models
        self.ops = ops
        #: 2,000 unless the corpus profile declares more (never above 65,536).
        self.max_cost = max_cost
        self.stats = stats or Stats()
        self._compiled: dict[str, tuple[Compiled, list[str]]] = {}
        self._in_progress: set[str] = set()
        self._order: list[str] = list(models)
        self._ctx: int | None = None  # index of the model being compiled
        self._memo_on = memo
        self._memo: dict[str, dict] = {}
        self._memo_w: dict[str, dict] = {}
        self._memo_size = 0
        self._depth: dict[str, int] = {}
        self._dstack: list[list[int]] = []

    # -- public ----------------------------------------------------------------

    def compile(self, ir: object, tenv: dict[str, A.Ty], allow_subject: bool = True) -> Compiled:
        """Compile a top-level expression and enforce ``MAX_IR_COST`` on it."""
        if self.max_cost > 65_536:
            raise OracleError("limit", "max_ir_cost exceeds the 65,536 ceiling")
        self._dstack.append([])
        try:
            c = self._compile(ir, tenv, allow_subject, 0)
        except RecursionError as exc:
            raise OracleError("limit", "IR nests too deeply") from exc
        finally:
            self._dstack.pop()
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
        # Compile every earlier model first, in declaration order, so a long chain
        # of models never recurses (a 400-model chain would exhaust the stack).
        for earlier in self._order[: self._order.index(name)]:
            if earlier not in self._compiled and earlier not in self._in_progress:
                self._model(earlier)
        # Rule 4.2.1: a model may call only models declared *before* it.
        self._in_progress.add(name)
        saved, self._ctx = self._ctx, self._order.index(name)
        self._dstack.append([])
        try:
            body = self._compile(m.ir, dict(m.params), False, 0)
        finally:
            self._ctx = saved
            self._in_progress.discard(name)
            callee_depths = self._dstack.pop()
        self._depth[name] = 1 + max(callee_depths, default=0)
        if self._depth[name] > MAX_MODEL_DEPTH:
            raise OracleError("limit", "model calls nest deeper than 64 frames")
        if not _assignable(body.ty, m.ret):
            raise OracleError("ir", "a model body does not have its declared type")
        if body.cost > self.max_cost:
            raise OracleError("limit", "ir-cost: model exceeds MAX_IR_COST")
        entry = (body, [p for p, _ in m.params])
        self._compiled[name] = entry
        return entry

    # -- expressions ---------------------------------------------------------------

    def _compile(self, node: object, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        if depth >= MAX_IR_DEPTH:
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
        self._dstack[-1].append(self._depth[name])
        if 1 + self._depth[name] > MAX_MODEL_DEPTH + 1:
            raise OracleError("limit", "model calls nest deeper than 64 frames")
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
        table_w = self._memo_w.setdefault(name, {})
        prog = self
        stats = self.stats

        def call(env: dict) -> object:
            args = tuple(f(env) for f in fns)
            if not memo_ok:
                return body_fn(dict(zip(pnames, args)))
            if stats.counting:
                hit = table_w.get(args, table_w)
                if hit is not table_w:
                    stats.wraps += hit[1]  # replay the wraps the body performed
                    return hit[0]
                before = stats.wraps
                value = body_fn(dict(zip(pnames, args)))
                if prog._memo_size < MAX_MEMO:
                    table_w[args] = (value, stats.wraps - before)
                    prog._memo_size += 1
                return value
            hit = table.get(args, table)
            if hit is not table:
                return hit
            value = body_fn(dict(zip(pnames, args)))
            if prog._memo_size < MAX_MEMO:
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
                    if stats.counting and r != -x:
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
            cost = 1 + c.cost + max(t.cost, e.cost)  # worst path: one branch
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
                if stats.counting and r != x:
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
        if depth >= MAX_IR_DEPTH:
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
        if tag == "model_call":
            # The arguments are IR; the name is not.  ``inc(bump(a))`` has one call.
            return _sum(0, [count(a, depth + 1) for a in node[2]])
        children = [count(c, depth + 1) for c in node[1:] if isinstance(c, list)]
        return _sum(0, children)

    return count(ir)


#: Nodes that can wrap or fail when evaluated on the pure trace (section 3.3 rule 1):
#: arithmetic (wrap; exact ``Nat``/``Int`` can leave the 2^127 window) and ``concat``
#: (a result past ``MAX_IR_VALUE`` is ``ir-overflow``).  ``take``/``drop`` are total.
_ARITH_TAGS = frozenset({"add", "sub", "mul", "neg", "concat"})
_FIXED_ARITH_TAGS = frozenset({"add", "sub", "mul", "neg"})


def may_wrap(prog: Program, ir: object, _seen: set | None = None) -> bool:
    """Could evaluating ``ir`` (or a model it calls) wrap or fail (``+ - *``, ``neg``, ``concat``)?"""
    seen = _seen if _seen is not None else set()
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        tag = node[0]
        if tag in _ARITH_TAGS:
            return True
        if tag in ("lit", "var", "err"):
            continue
        if tag == "model_call":
            if node[1] not in seen and node[1] in prog.models:
                seen.add(node[1])
                if may_wrap(prog, prog.models[node[1]].ir, seen):
                    return True
            stack.extend(node[2])
        elif tag == "subject_op":
            stack.extend(node[2])
        else:
            stack.extend(node[1:])
    return False


def compile_trace(prog: Program, ir: object, tenv: dict) -> Callable | None:
    """A closure that evaluates, for effect, every *pure* node on the pure trace.

    This is what ``wraps`` is counted on (VER04r section 3.3): the generator
    evaluates the subject-independent nodes, which include the pure arguments of a
    subject call (``f(e + 1u64)``) and the expected side, but never a node whose
    evaluation depends on what a subject returned.  The walk mirrors
    :func:`compile_calls`: a pure guard is evaluated and the taken branch followed;
    an operand that only runs under a subject-dependent short circuit is skipped.
    Returns ``None`` when nothing on the trace can wrap.
    """

    def tr(node: object, depth: int = 0) -> list[Callable]:
        if depth >= MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            if may_wrap(prog, node):
                return [prog.compile(node, tenv, False).fn]
            return []
        tag = node[0]
        if tag == "subject_op" or tag == "model_call":
            return [f for a in node[2] for f in tr(a, depth + 1)]
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                cf = prog.compile(cond, tenv, False).fn
                tt, et = tr(then, depth + 1), tr(other, depth + 1)

                def run_if(env: dict) -> None:
                    for f in tt if cf(env) else et:
                        f(env)

                return [run_if]
            return tr(cond, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                lf = prog.compile(left, tenv, False).fn
                rt = tr(right, depth + 1)
                runs = (lambda v: not v) if tag == "or" else bool

                def run_sc(env: dict) -> None:
                    if runs(lf(env)):
                        for f in rt:
                            f(env)

                return [run_sc]
            return tr(left, depth + 1)
        return [f for c in node[1:] if isinstance(c, list) for f in tr(c, depth + 1)]

    fns = tr(ir)
    if not fns:
        return None

    def run_all(env: dict) -> None:
        for f in fns:
            f(env)

    return run_all


class OutsideAdmit(Exception):
    """A subject call's arguments are outside the callee's ``admit`` (VER04r section 6, [D19]).

    Internal to the oracle: :class:`~coding_adventures_warrant_oracle.suites.SuitePlan` turns
    it into the generation error, naming the claim, the op and the admitted-case ordinal.
    """

    def __init__(self, op: str) -> None:
        super().__init__(op)
        self.op = op


def _reads(ir: object) -> set[str]:
    """The variable names an IR tree reads (``["var", name]`` anywhere in it)."""
    seen: set[str] = set()
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        tag = node[0]
        if tag == "var":
            seen.add(node[1])
        elif tag in ("lit", "err"):
            continue
        elif tag in ("model_call", "subject_op"):
            stack.extend(node[2])
        else:
            stack.extend(node[1:])
    return seen


class WrapsInAdmit(Exception):
    """A fixed-width node wrapped while evaluating a callee's ``admit`` in a v0 document.

    VER04r 3.3 rule 2 applies to every evaluation the generator performs (owner decision
    [D18]); the wrap is refused but never added to the recorded ``wraps``.
    """

    def __init__(self, op: str) -> None:
        super().__init__(op)
        self.op = op


def run_admit(prog: Program, admit: Callable, penv: dict, v0: bool, op: str) -> object:
    """Evaluate a callee's compiled ``admit`` on ``penv``; in a v0 document refuse a wrap.

    The wraps are counted (so the evaluation takes the counting path) and then discarded:
    ``wraps`` of a suite row is unchanged by this check.
    """
    if not v0:
        return admit(penv)
    stats = prog.stats
    before, was = stats.wraps, stats.counting
    stats.counting = True
    try:
        verdict = admit(penv)
    finally:
        stats.counting = was
    wrapped = stats.wraps > before
    stats.wraps = before
    if wrapped:
        raise WrapsInAdmit(op)
    return verdict


def admit_check_cost(prog: Program, ir: object, tenv: dict) -> int:
    """The IR work of [D19]'s per-case check for ``ir``: for every ``subject_op`` whose op has
    an ``admit``, the worst-path cost of that ``admit`` plus the cost of each argument that
    does not call the subject.  A static sum over the whole tree, charged per product ordinal
    to the IR-work meter and **not** part of the recorded ``ir_cost``.
    """
    total = 0
    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        tag = node[0]
        if tag in ("lit", "var", "err"):
            continue
        if tag == "subject_op":
            op = prog.ops[node[1]]
            if op.admit is not None:
                statics = dict(zip(op.statics, node[3] if len(node) == 4 else []))
                ptenv = {n: _substitute(t, statics) for n, t in op.params}
                total += prog.compile(op.admit, ptenv, False).cost
                total += sum(prog.compile(a, tenv, False).cost for a in node[2] if not has_subject(a))
            stack.extend(node[2])
        elif tag == "model_call":
            stack.extend(node[2])
        else:
            stack.extend(node[1:])
    return total


def admit_fn(prog: Program, op: object, widths: list | None) -> Callable | None:
    """The compiled ``admit`` of subject op ``op`` over its parameter names, or ``None``.

    ``widths`` are the static widths of the call (``Bytes<N>`` parameters take their type
    from them); the closure takes a dict of parameter values.
    """
    if op.admit is None:
        return None
    statics = dict(zip(op.statics, widths or []))
    ptenv = {n: _substitute(t, statics) for n, t in op.params}
    return prog.compile(op.admit, ptenv, False).fn


def compile_admit_check(
    prog: Program, ir: object, tenv: dict, v0: bool = False
) -> Callable | None:
    """A closure ``fn(env)`` raising :class:`OutsideAdmit` when a subject call on the pure
    trace of ``ir`` has arguments outside its op's ``admit`` (VER04r section 6, [D19]).

    The walk mirrors :func:`compile_trace`: a pure guard is evaluated and the taken branch
    followed, so a call in an untaken branch is not checked.  A call of an op whose ``admit``
    reads a parameter whose argument contains a subject call cannot be checked at generation
    (the oracle never learns what the inner call returns): that is the generation error
    ``W0701``, raised here, statically, for every such call anywhere in ``ir``.  An argument
    that depends on a subject result for a parameter the ``admit`` does not read is simply not
    an input of the check (``power(add(a, b), 2u64)``).  Returns
    ``None`` when no call on the body is of an op with an ``admit``.
    """

    def tr(node: object, depth: int = 0) -> list[Callable]:
        if depth >= MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            return []
        tag = node[0]
        if tag == "subject_op":
            kids = [f for a in node[2] for f in tr(a, depth + 1)]
            op = prog.ops[node[1]]
            if op.admit is None:
                return kids
            names = [n for n, _ in op.params]
            dynamic = {n for n, a in zip(names, node[2]) if has_subject(a)}
            if dynamic & _reads(op.admit):
                raise OracleError(
                    "generation",
                    f"W0701: the admit of op `{op.name}` reads a parameter whose argument "
                    "depends on a subject result, so it cannot be checked at generation "
                    "(VER04r 6)",
                )
            admit = admit_fn(prog, op, node[3] if len(node) == 4 else None)
            # An argument that depends on a subject result is not an input of the check:
            # the admit does not read its parameter (just refused otherwise).
            args = [
                (n, prog.compile(a, tenv, False).fn)
                for n, a in zip(names, node[2])
                if n not in dynamic
            ]

            def check(env: dict, admit=admit, args=args, name=op.name) -> None:
                if not run_admit(prog, admit, {n: f(env) for n, f in args}, v0, name):
                    raise OutsideAdmit(name)

            return [check] + kids
        if tag == "model_call":
            return [f for a in node[2] for f in tr(a, depth + 1)]
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                cf = prog.compile(cond, tenv, False).fn
                tt, et = tr(then, depth + 1), tr(other, depth + 1)

                def run_if(env: dict) -> None:
                    for f in tt if cf(env) else et:
                        f(env)

                return [run_if] if tt or et else []
            return tr(cond, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                lf = prog.compile(left, tenv, False).fn
                rt = tr(right, depth + 1)
                runs = (lambda v: not v) if tag == "or" else bool

                def run_sc(env: dict) -> None:
                    if runs(lf(env)):
                        for f in rt:
                            f(env)

                return [run_sc] if rt else []
            return tr(left, depth + 1)
        return [f for c in node[1:] if isinstance(c, list) for f in tr(c, depth + 1)]

    fns = tr(ir)
    if not fns:
        return None

    def run_all(env: dict) -> None:
        for f in fns:
            f(env)

    return run_all


def _sum(base: int, parts: list) -> int | Callable:
    static = base + sum(p for p in parts if not callable(p))
    dynamic = [p for p in parts if callable(p)]
    if not dynamic:
        return static
    return lambda env: static + sum(p(env) for p in dynamic)


def fixed_arith_over_subject(prog: Program, ir: object, tenv: dict) -> bool:
    """VER04r section 3.3 rule 3: a fixed-width ``+ - *``/unary ``-`` above a subject call?

    A **v0 document's** Rust harness evaluates ``U8`` as ``Nat``, so whether such a node
    wraps depends on what the subject returns and no generation-time count can prove
    the two evaluators agree: the claim is refused statically.  The rule covers
    ``model_call`` too: ``inc(subject(..))`` where ``model inc(x: U8) -> U8 = x + 1u8``
    has a subject beneath the call and a fixed-width node in the callee's *call
    closure* (the model's body and, transitively, every model it calls).
    """
    closure: dict[str, bool] = {}

    def model_has_fixed(name: str) -> bool:
        if name in closure:
            return closure[name]
        closure[name] = False  # models are acyclic (rule 4.2.1); this only guards hostile input
        m = prog.models[name]
        mtenv = dict(m.params)
        result = False
        stack = [m.ir]
        while stack and not result:
            node = stack.pop()
            if not (isinstance(node, list) and node and isinstance(node[0], str)):
                continue
            tag = node[0]
            if tag in _FIXED_ARITH_TAGS and prog.compile(node[1], mtenv, False).ty.head in FIXED or tag == "model_call" and model_has_fixed(node[1]):
                result = True
            elif tag not in ("lit", "var", "err"):
                stack.extend(node[2] if tag in ("model_call", "subject_op") else node[1:])
        closure[name] = result
        return result

    stack = [ir]
    while stack:
        node = stack.pop()
        if not (isinstance(node, list) and node and isinstance(node[0], str)):
            continue
        tag = node[0]
        if tag in ("lit", "var", "err"):
            continue
        if tag in _FIXED_ARITH_TAGS and has_subject(node):
            if prog.compile(node[1], tenv, True).ty.head in FIXED:
                return True
        elif tag == "model_call" and has_subject(node) and model_has_fixed(node[1]):
            return True
        stack.extend(node[2] if tag in ("model_call", "subject_op") else node[1:])
    return False
