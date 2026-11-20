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

``ir.tuple`` (slice S10t-1)
---------------------------
The tuple literal ``["tuple", [e1, .., en]]`` (2 to 8 components, type ``(T1,..,Tn)``, cost
``1 +`` the operands), ``["project", e, i]`` (a 0-based JSON integer ``i``, type ``Ti``) and
``["match", s, [[pat, arm], ..]]`` on a ``Result`` are typed, costed and evaluated.  A tuple value
is a Python tuple and equality is component-wise.  ``match`` evaluates **exactly one arm**, the arm of
the first pattern that matches, with the pattern's binders in scope (:mod:`.patterns`); its cost is
``1 +`` cost(scrutinee) ``+`` the **maximum** over the arms (section 4.2 rule 6).  A ``match`` whose
scrutinee is not a ``Result`` is the level ``ir.data``.  The corpus errors of Appendix A.1 (a match that
falls through, an arm no value can reach, the same ``bind`` twice in a pattern, a ``bind`` that shadows
a binder, a parameter, ``result`` or an outer ``bind``) are ``corpus`` errors.  A ``match`` arm is a
*guard* for call counting (section 4.4): a subject call in an arm needs a pure scrutinee.

``ir.data`` (slice S11a)
-----------------------
With ``Program(..., analyze_data=True)`` (what the lowering dump uses) the four tags ``some none ctor field``
and ``match`` on any scrutinee with the data patterns are **typed and costed** (Appendix A.2: ``some``,
``ctor`` and ``field`` cost ``1 +`` their operands, ``none`` costs 1, ``match`` ``1 +`` the scrutinee
``+`` the worst arm) and ``Option`` may be the type of any node; their closures *refuse* to run
(``level ir.data``), because evaluating them is slice S11c-1.  Without ``analyze_data`` every data tag and
every ``Option`` outside the intermediate use of ``ir.fixed`` is the level ``ir.data``, as before.  A list
(``ir.list``, reserved) is refused in both modes.

``ir.fixed`` (slices S10a and S10c-1)
-------------------------------------
The 35 built-ins of Appendix A.1 are typed, costed (``1 +`` the operands) and **evaluated** on Python
integers with explicit masking (:meth:`Program._fixed_fn`).  The semantics, in one paragraph: a
fixed-width value is held as the mathematical integer it denotes, in the type's range; every bit
operation works on the ``w``-bit two's-complement *pattern* (``value & (2**w - 1)``) and re-reads the
result in the type (``wrap_to``); a shift count is an exact ``Nat`` and ``n >= w`` is compared
**before** any host shift (``shl``/``shr`` give 0, ``sar`` gives ``-1`` or 0, ``bit`` gives false),
while ``rotl``/``rotr`` reduce ``n mod w`` first; ``popcount``/``clz``/``ctz`` read the pattern
(``clz``/``ctz`` of 0 is ``w``); ``div``/``rem`` on ``Int``/``Nat`` are floor division and the
divisor-signed remainder, on unsigned fixed values plain, and ``div_trunc``/``rem_trunc`` truncate
toward zero; an ``Option`` is ``("none",)`` or ``("some", v)`` and its one consumer is ``unwrap_or``
(both operands are evaluated: no short circuit).  ``wraps`` counts only ``add sub mul neg`` [D31]: the
explicit truncations (``wrap``, ``shl``, ``bit_not`` ...) never count.  ``Program(..., analyze=True)``
is accepted for compatibility and changes nothing: there is no longer a mode that only types.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from functools import cached_property

from . import ast as A
from . import levels as L
from . import patterns as P
from .errors import OracleError, SizeError
from .irtree import children, is_node, match_arms
from .lower import BOOL, BYTES, FIXED, INT, NAT, _substitute, _unify
from .lower import _assignable as _strict_assignable
from .typenames import parse_type_name

MAX_IR_DEPTH = 64
MAX_MODEL_DEPTH = 64
MAX_IR_COST = 2000
MAX_IR_VALUE = 65_536
EXACT_BOUND = (1 << 127) - 1
MAX_MEMO = 1 << 20

CORE_TAGS = L.CORE_TAGS
#: Tags and built-ins of the levels above core.  The tuple tags and ``match`` on a ``Result``
#: (``ir.tuple``) are evaluated since slice S10t-1 and the 35 ``ir.fixed`` built-ins since S10c-1;
#: ``ir.data`` and ``ir.seq`` tags are recognised so they can be *named* (VER04r section 10.4).
_FIXED_TAGS = L.FIXED_TAGS
_TUPLE_TAGS = L.TUPLE_TAGS
_DATA_TAGS = frozenset(L.DATA_TAGS)
_SEQ_TAGS = L.SEQ_TAGS

_COMPARE = {"eq", "ne", "lt", "gt", "le", "ge"}
_ARITH = {"add", "sub", "mul"}


_UNKNOWN = A.Ty("?")


def _assignable(src: A.Ty, dst: A.Ty, lift: bool = True) -> bool:
    """Assignability that tolerates the unknown error type of a bare ``ok``/``err``.

    An ``["ok", e]`` node has no way to know its error type (the IR carries
    only the case name), so its ``Result`` is ``Result<T, ?>``: compatible with
    any ``Result<T', E>`` whose payload type accepts ``T``.

    Otherwise this is :func:`.lower._assignable`, the grammar checker's ``is_assignable_to`` ([D60]): ``Nat``
    to ``Int``, ``Bytes<N>`` to ``Bytes``, through an ``Option`` or ``Result`` payload.  **A tuple component
    never lifts** (``lift`` is off below a tuple, as in the checker): only the unknown error type is
    tolerated there.
    """
    if not lift:
        if src == dst:
            return True
        if src.head == dst.head == "Option":
            return _assignable(src.args[0], dst.args[0], False)
        if src.head == dst.head == "Result":
            payload = src.args[0] == _UNKNOWN or _assignable(src.args[0], dst.args[0], False)
            return payload and (_UNKNOWN in (src.args[1], dst.args[1]) or src.args[1] == dst.args[1])
        if src.head == dst.head == "()":
            return len(src.args) == len(dst.args) and all(
                _assignable(x, y, False) for x, y in zip(src.args, dst.args)
            )
        return False
    if src.head == dst.head == "Result":
        payload = src.args[0] == _UNKNOWN or _assignable(src.args[0], dst.args[0])
        return payload and (_UNKNOWN in (src.args[1], dst.args[1]) or src.args[1] == dst.args[1])
    if src.head == dst.head == "Option":
        return _assignable(src.args[0], dst.args[0])
    if src.head == dst.head == "()":
        return _assignable(src, dst, False)
    return _strict_assignable(src, dst)


def _merge_result(a: A.Ty, b: A.Ty) -> A.Ty:
    """The more informative of two ``Result`` types (``?`` is the unknown)."""
    return b if a.args[0] == _UNKNOWN else a


def _common(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of two branch or operand types, or ``None``.

    ``Result`` types merge their unknowns (the lax rule ``if`` has always had); two ``Option`` types
    unify by payload ([D60]); two tuple types unify only component by component *without* lifting (the
    checker's ``common_type`` has no tuple rule: equal, or the unknown error type of a bare ``ok``/``err``
    merged); everything else is :func:`.lower._unify`.
    """
    if a.head == "Result" and b.head == "Result":
        return _merge_result(a, b)
    if a.head == b.head == "Option":
        inner = _common(a.args[0], b.args[0])
        return A.Ty("Option", (inner,)) if inner is not None else None
    if a.head == b.head == "()":
        if len(a.args) != len(b.args):
            return None
        parts = [_common_exact(x, y) for x, y in zip(a.args, b.args)]
        return None if any(p is None for p in parts) else A.Ty("()", tuple(parts))
    return _unify(a, b)


def _common_exact(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of two tuple *components*: equal, or merged through the unknown error type; nothing lifts."""
    if a == b:
        return a
    if a.head == b.head == "Result":
        return _merge_result(a, b) if _assignable(a, b, False) or _assignable(b, a, False) else None
    if a.head == b.head == "Option":
        inner = _common_exact(a.args[0], b.args[0])
        return A.Ty("Option", (inner,)) if inner is not None else None
    if a.head == b.head == "()" and len(a.args) == len(b.args):
        parts = [_common_exact(x, y) for x, y in zip(a.args, b.args)]
        return None if any(p is None for p in parts) else A.Ty("()", tuple(parts))
    return None


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


def _has_option(ty: A.Ty) -> bool:
    """Does ``ty`` mention ``Option`` anywhere?"""
    stack = [ty]
    while stack:
        t = stack.pop()
        if isinstance(t, A.Ty):
            if t.head == "Option":
                return True
            stack.extend(t.args)
    return False


def _unevaluated(*_args: object) -> object:
    """The closure of an ``ir.data`` node in analysis mode: typing and cost only (slice S11c-1 evaluates)."""
    raise OracleError("level", "ir.data is typed and costed here, not evaluated yet", level=L.DATA)


def _inside(source: str, target: str) -> bool:
    """Is the range of fixed type ``source`` inside that of ``target``?"""

    def span(name: str) -> tuple[int, int]:
        bits, signed = FIXED[name]
        return (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)

    return span(target)[0] <= span(source)[0] and span(source)[1] <= span(target)[1]


_NONE = ("none",)


def _fixed_fn(tag: str, static: str | None, kids: list[Compiled]) -> Callable:
    """The closure of one ``ir.fixed`` node (Appendix A.1), operand types already checked.

    Values are Python ints in the type's range; a bit operation masks to the ``w``-bit pattern and
    re-reads the result with :func:`wrap_to`.  Nothing here counts a ``wrap`` [D31].  The operand
    closures are called left to right, every one of them (``unwrap_or`` evaluates both).
    """
    fns = [k.fn for k in kids]
    # The width and signedness the operation works at: the static type argument for the tags that
    # have one, otherwise the first operand's own type (``bit_*``, shifts, counts, endian bytes).
    ty = static if static is not None else (kids[0].ty.head if kids else "")
    bits, signed = FIXED.get(ty, (0, False))
    mask = (1 << bits) - 1 if bits else 0
    lo, hi = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, mask)

    def reread(pattern: int) -> int:
        return wrap_to(pattern, bits, signed)

    if tag in ("max_value", "min_value"):
        const = hi if tag == "max_value" else lo
        return lambda env: const
    a = fns[0] if fns else None
    b = fns[1] if len(fns) > 1 else None
    if tag in ("bit_and", "bit_or", "bit_xor"):
        op = {
            "bit_and": lambda x, y: x & y,
            "bit_or": lambda x, y: x | y,
            "bit_xor": lambda x, y: x ^ y,
        }[tag]
        return lambda env: reread(op(a(env) & mask, b(env) & mask))
    if tag == "bit_not":
        return lambda env: reread(~(a(env) & mask) & mask)
    if tag in ("shl", "shr", "sar", "rotl", "rotr", "bit"):
        return _shift_fn(tag, bits, signed, mask, a, b)
    if tag == "popcount":
        return lambda env: bin(a(env) & mask).count("1")
    if tag == "clz":
        return lambda env: bits - (a(env) & mask).bit_length()
    if tag == "ctz":

        def ctz(env: dict) -> int:
            p = a(env) & mask
            return bits if p == 0 else (p & -p).bit_length() - 1

        return ctz
    if tag == "widen":
        return a
    if tag == "wrap":
        return lambda env: reread(a(env))
    if tag == "try":

        def try_(env: dict) -> tuple:
            v = a(env)
            return ("some", v) if lo <= v <= hi else _NONE

        return try_
    if tag in ("to_int", "to_nat"):
        return a
    if tag in ("checked_add", "checked_sub", "checked_mul"):
        exact = {
            "checked_add": lambda x, y: x + y,
            "checked_sub": lambda x, y: x - y,
            "checked_mul": lambda x, y: x * y,
        }[tag]

        def checked(env: dict) -> tuple:
            v = exact(a(env), b(env))
            return ("some", v) if lo <= v <= hi else _NONE

        return checked
    if tag in ("div", "rem"):
        # Unsigned fixed pairs and Int/Nat pairs only (a signed fixed pair is refused by the typer):
        # Python's ``//`` and ``%`` are floor division and the divisor-signed remainder, which is
        # what ``Int``/``Nat`` need and, for non-negative operands, plain division.
        want_div = tag == "div"

        def divrem(env: dict) -> tuple:
            x, y = a(env), b(env)
            if y == 0:
                return _NONE
            return ("some", x // y if want_div else x % y)

        return divrem
    if tag in ("div_trunc", "rem_trunc"):
        want_div = tag == "div_trunc"

        def divrem_trunc(env: dict) -> tuple:
            x, y = a(env), b(env)
            if y == 0 or (x == lo and y == -1):
                return _NONE
            q = abs(x) // abs(y)
            if (x < 0) != (y < 0):
                q = -q
            return ("some", q if want_div else x - q * y)

        return divrem_trunc
    if tag in ("imin", "imax"):
        pick = min if tag == "imin" else max
        return lambda env: pick(a(env), b(env))
    if tag in ("be_bytes", "le_bytes"):
        order = "big" if tag == "be_bytes" else "little"
        width = bits // 8
        return lambda env: (a(env) & mask).to_bytes(width, order)
    if tag in ("from_be", "from_le"):
        order = "big" if tag == "from_be" else "little"

        def from_bytes(env: dict) -> int:
            data = a(env)
            if len(data) * 8 != bits:
                # A corpus error, never a truncation or a zero extension (Appendix A.1).
                raise OracleError("ir", "from_be/from_le need exactly the target's width in bytes")
            return reread(int.from_bytes(data, order))

        return from_bytes
    if tag == "at":

        def at(env: dict) -> tuple:
            data, i = a(env), b(env)
            return ("some", data[i]) if 0 <= i < len(data) else _NONE

        return at
    if tag == "unwrap_or":

        def unwrap_or(env: dict) -> object:
            option = a(env)
            default = b(env)  # evaluated whether or not it is used: no short circuit
            return option[1] if option[0] == "some" else default

        return unwrap_or
    raise OracleError("ir", "an unknown ir.fixed tag")  # pragma: no cover (FIXED_ARITY is exhaustive)


def _shift_fn(tag: str, bits: int, signed: bool, mask: int, a: Callable, b: Callable) -> Callable:
    """``shl shr sar rotl rotr bit``: the count is an exact ``Nat`` (Appendix A.1, notes).

    ``n >= w`` is compared **before** any shift, so the result never depends on a host's masking
    of the count: ``shl(1u8, 8) == 0``, ``sar(-1i8, 300) == -1``, ``bit(x, 64) == false``.  The
    rotations reduce ``n mod w`` first.
    """
    if tag == "shl":

        def shl(env: dict) -> int:
            x, n = a(env), b(env)
            return 0 if n >= bits else wrap_to((x & mask) << n, bits, signed)

        return shl
    if tag == "shr":

        def shr(env: dict) -> int:
            x, n = a(env), b(env)
            return 0 if n >= bits else wrap_to((x & mask) >> n, bits, signed)

        return shr
    if tag == "sar":

        def sar(env: dict) -> int:
            x, n = a(env), b(env)
            if n >= bits:
                return -1 if x < 0 else 0
            return x >> n  # Python's shift of a negative int fills with the sign

        return sar
    if tag in ("rotl", "rotr"):
        left = tag == "rotl"

        def rotate(env: dict) -> int:
            x, n = a(env), b(env)
            k = n % bits
            if not left:
                k = (bits - k) % bits
            p = x & mask
            return wrap_to(((p << k) | (p >> (bits - k))) & mask, bits, signed)

        return rotate

    def bit(env: dict) -> bool:
        x, i = a(env), b(env)
        return i < bits and ((x & mask) >> i) & 1 == 1

    return bit


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
        analyze: bool = False,
        errors: dict | None = None,
        adts: dict | None = None,
        analyze_data: bool = False,
    ) -> None:
        self.models = models
        self.ops = ops
        #: Error type name -> its cases, for ``match``: an ``["err", "CASE"]`` pattern is checked
        #: against it and the exhaustiveness check completes a ``Result`` with it.  An error type
        #: that is not listed is an open set (:mod:`.patterns`).
        self.errors = errors or {}
        #: The declared records and variants, ``name -> ((constructor, (field type, ..)), ..)``
        #: (:attr:`.lower.Spec.adts`), for ``ctor``, ``field`` and the constructor patterns.
        self.types = P.Types(self.errors, adts)
        #: Kept for compatibility: since S10c-1 the ``ir.fixed`` built-ins are always evaluated.
        self.analyze = analyze
        #: True for the lowering dump (slice S11a): ``ir.data`` is typed and costed but never evaluated, and
        #: an ``Option`` may be the type of any node.  Off, every data tag and every ``Option`` outside the
        #: intermediate use of ``ir.fixed`` is the level ``ir.data``, as it was before S11a.
        self.analyze_data = analyze_data
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
        #: Compiled callee ``admit`` rows, one per ``(op, static widths)`` (section 16.9
        #: follow-up): the D19 pass and its cost both ask for the same row at every call site.
        self._admits: dict[tuple, Compiled] = {}

    # -- the Option and type rules (D23 as amended by D48; R27) --------------------------------

    def _no_option(self, c: Compiled, where: str) -> None:
        """``Option<T>`` outside the intermediate use is ``ir.data``: refused unless ``ir.data`` is analysed."""
        if c.ty.head == "Option" and not self.analyze_data:
            raise OracleError(
                "level",
                f"an Option is not allowed as {where}; only unwrap_or consumes one",
                level=L.DATA,
            )

    def _refuse_type(self, ty: A.Ty) -> None:
        """A type of a binder, parameter, result or value must be allowed: a list is ``ir.list`` (R27)."""
        level = L.type_refusal(ty)
        if level is not None:
            raise OracleError("level", "a type that needs a reserved level", level=level)
        if not self.analyze_data and _has_option(ty):
            raise OracleError("level", "a type that needs a higher level", level=L.DATA)

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
        self._no_option(c, "an expression")
        if c.cost > self.max_cost:
            raise OracleError("limit", "ir-cost: expression exceeds MAX_IR_COST")
        return c

    def compile_node(self, ir: object, tenv: dict[str, A.Ty]) -> Compiled:
        """Compile a pure expression **without** the top-level ``Option`` check.

        :meth:`compile` refuses an ``Option`` result (an IR-only intermediate, D23); the committed
        value tables of slice S10c-1 and the tests print what the nine producers return, so they
        compile the producer node itself through here.  No cost bound is applied either.
        """
        return self._compile(ir, tenv, False, 0)

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
        for _, pty in m.params:
            self._refuse_type(pty)
        self._refuse_type(m.ret)
        self._no_option(body, "a model result")
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
                return self._fixed(node, tenv, allow_subject, depth)
            if tag == "tuple":
                return self._tuple(node, tenv, allow_subject, depth)
            if tag == "project":
                return self._project(node, tenv, allow_subject, depth)
            if tag == "match":
                return self._match(node, tenv, allow_subject, depth)
            if tag in _DATA_TAGS:
                return self._data(node, tenv, allow_subject, depth)
            if tag in L.LIST_TAGS:
                raise OracleError("level", "a tag of the reserved level ir.list", level=L.LIST)
            if tag in _SEQ_TAGS:
                raise OracleError("level", "a tag of level ir.seq", level="ir.seq")
            raise OracleError("ir", "an unknown IR tag")
        if tag == "lit":
            return self._lit(node)
        if tag == "var":
            if len(node) != 2 or not isinstance(node[1], str) or node[1] not in tenv:
                raise OracleError("ir", "an unbound IR variable")
            name = node[1]
            self._refuse_type(tenv[name])
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
        for k in kids:
            self._no_option(k, f"an operand of {tag}")
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
                    raise SizeError("a Bytes literal exceeds MAX_IR_VALUE")
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
        for _, pty in op.params:
            self._refuse_type(pty)
        self._refuse_type(op.ret)
        kids = [self._compile(a, tenv, True, depth + 1) for a in node[2]]
        for k, (_, pty) in zip(kids, op.params):
            self._no_option(k, "an argument of a subject call")
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
            self._no_option(k, "an argument of a model call")
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

    # -- ir.tuple: the tuple literal, project and match on a Result (slice S10t-1) -------------

    def _tuple(self, node: list, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        """``["tuple", [e1, .., en]]``: 2 to 8 operands, type ``(T1,..,Tn)``, cost ``1 +`` the operands."""
        if len(node) != 2 or not isinstance(node[1], list):
            raise OracleError("ir", "a malformed tuple node")
        operands = node[1]
        if not P.MAX_TUPLE_ARITY >= len(operands) >= 2:
            raise OracleError("ir", "a tuple has 2 to 8 components")
        kids = [self._compile(child, tenv, allow_subject, depth + 1) for child in operands]
        for k in kids:
            self._no_option(k, "a tuple component")
        fns = [k.fn for k in kids]
        return Compiled(
            A.Ty("()", tuple(k.ty for k in kids)),
            lambda env: tuple(f(env) for f in fns),
            1 + sum(k.cost for k in kids),
            any(k.subject for k in kids),
        )

    def _project(self, node: list, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        """``["project", e, i]``: ``i`` is a 0-based JSON integer inside the tuple's arity."""
        if len(node) != 3 or type(node[2]) is not int or not isinstance(node[1], list):
            raise OracleError("ir", "a malformed project node")
        base = self._compile(node[1], tenv, allow_subject, depth + 1)
        self._no_option(base, "an operand of project")
        index = node[2]
        if base.ty.head != "()":
            raise OracleError("ir", "project needs a tuple")
        if not 0 <= index < len(base.ty.args):
            raise OracleError("ir", "a projection index outside the tuple")
        bf = base.fn
        return Compiled(base.ty.args[index], lambda env: bf(env)[index], 1 + base.cost, base.subject)

    def match_arms(
        self, node: list, tenv: dict, allow_subject: bool = False, depth: int = 0
    ) -> tuple[Compiled, list]:
        """Type the scrutinee and patterns of a ``match``: ``(scrutinee, [(matcher, binders, body)])``.

        On a ``Result`` with the six patterns of A.1 this is ``ir.tuple``; any other scrutinee, or a data
        pattern, is ``ir.data``: refused unless the program analyses (typing and cost only, S11a).

        ``binders`` is the arm's ``{name: type}``; ``matcher(value, binds)`` fills ``binds`` and says
        whether the pattern matches.  This is everything about a ``match`` that does not compile an
        arm body, so the call-counting analyses (a ``match`` arm is a guard, section 4.4) can compile
        each body in its own scope.  The corpus errors of Appendix A.1 are raised here.
        """
        arms = match_arms(node)
        s = self._compile(node[1], tenv, allow_subject, depth + 1)
        self._no_option(s, "a match scrutinee")
        if not self.analyze_data and (
            s.ty.head != "Result" or any(L.has_data_pattern(pattern) for pattern, _ in arms)
        ):
            raise OracleError(
                "level", "match on a scrutinee that is not a Result, or with a data pattern", level=L.DATA
            )
        if len(arms) > P.MAX_ARMS:
            raise OracleError("limit", "a match has more than 256 arms")
        out: list = []
        patterns: list = []
        for pattern, body in arms:
            binds: list = []
            P.type_pattern(pattern, s.ty, self.types, binds)
            names = [n for n, _ in binds]
            if len(set(names)) != len(names):
                raise OracleError("corpus", "the same bind name twice in one pattern")
            for name in names:
                if name in tenv or name == "result":
                    raise OracleError(
                        "corpus",
                        "a bind name shadows a binder, a parameter, result or an outer bind",
                    )
            patterns.append(pattern)
            matcher = _unevaluated if L.has_data_pattern(pattern) else P.compile_pattern(pattern)
            out.append((matcher, dict(binds), body))
        P.check_arms(patterns, s.ty, self.types)
        return s, out

    def _match(self, node: list, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        """``["match", s, [[pat, arm], ..]]`` on a ``Result``; one arm is evaluated (rule 2).

        Cost: ``1 +`` cost(scrutinee) ``+`` the maximum over the arms (rule 6).  The arms must have
        one type (:func:`_common`).
        """
        s, arms = self.match_arms(node, tenv, allow_subject, depth)
        compiled: list = []
        common: A.Ty | None = None
        worst = 0
        subject = s.subject
        for matcher, binders, body in arms:
            kid = self._compile(body, {**tenv, **binders}, allow_subject, depth + 1)
            self._no_option(kid, "a match arm")
            common = kid.ty if common is None else _common(common, kid.ty)
            if common is None:
                raise OracleError("ir", "the arms of a match have no common type")
            compiled.append((matcher, kid.fn))
            worst = max(worst, kid.cost)
            subject = subject or kid.subject
        sf = s.fn

        def run(env: dict) -> object:
            value = sf(env)
            for matcher, arm in compiled:
                binds: dict = {}
                if matcher(value, binds):
                    return arm({**env, **binds})
            raise OracleError("ir", "a match fell through")  # pragma: no cover (exhaustive)

        return Compiled(common, run, 1 + s.cost + worst, subject)

    # -- ir.data: some, none, ctor and field, typed and costed (slice S11a) ---------------------

    def _data(self, node: list, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        """Type and cost one of the four tags of Appendix A.2 (never evaluated before slice S11c-1).

        ``["some", e]`` is ``Option<type(e)>`` and costs ``1 +`` the operand; ``["none", "T"]`` is
        ``Option<T>``, ``T`` read back from its canonical name, and costs 1; ``["ctor", "Name", [e..]]`` is
        the record or variant that declares ``Name``, one operand per declared field, each assignable to
        its field, and costs ``1 +`` the operands; ``["field", e, i]`` is the declared type of field ``i``
        of a record and costs ``1 +`` the operand.  A mismatch is an ``ir`` error (a corpus error).  Without
        ``analyze_data`` the tag is the level ``ir.data``.
        """
        if not self.analyze_data:
            raise OracleError("level", "a tag of level ir.data", level=L.DATA)
        tag = node[0]
        if tag == "none":
            if len(node) != 2:
                raise OracleError("ir", "a malformed none node")
            try:
                payload = parse_type_name(node[1])
            except ValueError as exc:
                raise OracleError("ir", "a none needs a canonical type name") from exc
            self._declared_type(payload)
            return Compiled(A.Ty("Option", (payload,)), _unevaluated, 1, False)
        if tag == "some":
            if len(node) != 2:
                raise OracleError("ir", "a malformed some node")
            kid = self._compile(node[1], tenv, allow_subject, depth + 1)
            return Compiled(A.Ty("Option", (kid.ty,)), _unevaluated, 1 + kid.cost, kid.subject)
        if tag == "ctor":
            if len(node) != 3 or not isinstance(node[1], str) or not isinstance(node[2], list):
                raise OracleError("ir", "a malformed ctor node")
            owner = self.types.ctors.get(node[1])
            if owner is None:
                raise OracleError("ir", "a ctor names no declared constructor")
            type_name, index = owner
            fields = self.types.adts[type_name][index][1]
            if len(node[2]) != len(fields):
                raise OracleError("ir", "a ctor has the wrong number of operands")
            kids = [self._compile(child, tenv, allow_subject, depth + 1) for child in node[2]]
            for kid, field_ty in zip(kids, fields):
                if not _assignable(kid.ty, field_ty):
                    raise OracleError("ir", "a ctor operand does not have its field's type")
            return Compiled(
                A.Ty(type_name), _unevaluated, 1 + sum(k.cost for k in kids), any(k.subject for k in kids)
            )
        # field
        if len(node) != 3 or type(node[2]) is not int or not isinstance(node[1], list):
            raise OracleError("ir", "a malformed field node")
        base = self._compile(node[1], tenv, allow_subject, depth + 1)
        if base.ty.head not in self.types.adts or base.ty.head not in self._record_names:
            raise OracleError("ir", "field needs a record")
        fields = self.types.adts[base.ty.head][0][1]
        if not 0 <= node[2] < len(fields):
            raise OracleError("ir", "a field index outside the record")
        return Compiled(fields[node[2]], _unevaluated, 1 + base.cost, base.subject)

    @cached_property
    def _record_names(self) -> frozenset[str]:
        """The record names: an ADT with exactly one constructor named like itself (computed once)."""
        return frozenset(
            name
            for name, ctors in self.types.adts.items()
            if len(ctors) == 1 and ctors[0][0] == name
        )

    def _declared_type(self, ty: A.Ty) -> None:
        """A type named in IR (a ``none``'s payload) must be allowed and declared: ``ir`` otherwise, a list is ``ir.list``."""
        level = L.type_refusal(ty)
        if level is not None:
            raise OracleError("level", "a type that needs a reserved level", level=level)
        stack = [ty]
        while stack:
            t = stack.pop()
            if t.head == "Bytes":
                if t.args and not (isinstance(t.args[0], int) and t.args[0] <= 4096):
                    raise OracleError("ir", "a Bytes width is at most 4096")
                continue
            if t.head == "Result":
                if t.args[1].head not in self.errors:
                    raise OracleError("ir", "a Result names an undeclared error type")
                stack.append(t.args[0])
                continue
            if t.head in ("()", "Option"):
                stack.extend(t.args)
                continue
            if (
                t.head in ("Bool", "Int", "Nat")
                or t.head in FIXED
                or t.head in self.types.adts
            ):
                continue
            raise OracleError("ir", "a type name that is not declared")

    # -- the ir.fixed built-ins, typed and costed (slice S10a) -----------------------------

    def _fixed(self, node: list, tenv: dict, allow_subject: bool, depth: int) -> Compiled:
        """Type, cost and compile one ``ir.fixed`` node of Appendix A.1.

        The shape is checked first (arity, and the static type argument in position 1 of
        ``widen wrap try from_be from_le max_value min_value``), then the operands are compiled
        and typed.  Every tag costs ``1 +`` the sum of its operands (section 4.2 rule 6); a
        static type argument is not an operand, so ``max_value`` costs ``1``.  A mismatch is an
        ``ir`` error (a corpus error), an ``Option`` anywhere but ``unwrap_or``'s first operand
        is the level ``ir.data`` (D23, R8).  The closure is :func:`_fixed_fn`.
        """
        tag = node[0]
        static: str | None = None
        first = 1
        if tag in L.STATIC_TYPE_TAGS:
            if len(node) < 2 or not isinstance(node[1], str) or node[1] not in FIXED:
                raise OracleError("ir", "a static type argument is a fixed-width type name")
            static = node[1]
            first = 2
        operands = node[first:]
        if len(operands) != L.FIXED_ARITY[tag]:
            raise OracleError("ir", "a malformed IR node")
        kids = [self._compile(child, tenv, allow_subject, depth + 1) for child in operands]
        for i, k in enumerate(kids):
            if k.ty.head == "Option":
                # R29 (e): the intermediate use is the evaluated one; ``unwrap_or`` over any other Option,
                # and every other consumer, is ``ir.data`` (analysed in the lowering dump, S11a).
                direct = tag == "unwrap_or" and i == 0 and operands[0][0] in L.OPTION_PRODUCERS
                if not direct:
                    self._no_option(k, f"an operand of {tag}")
        ty = self._fixed_type(tag, static, kids)
        cost = 1 + sum(k.cost for k in kids)
        return Compiled(ty, _fixed_fn(tag, static, kids), cost, any(k.subject for k in kids))

    def _fixed_type(self, tag: str, static: str | None, kids: list[Compiled]) -> A.Ty:
        """The result type of an ``ir.fixed`` node (operand types are those of Appendix A.1)."""

        def bad(why: str) -> OracleError:
            return OracleError("ir", why)

        types = [k.ty for k in kids]
        fixed = lambda t: t.head in FIXED  # noqa: E731
        signed = lambda t: FIXED[t.head][1]  # noqa: E731
        if tag in ("max_value", "min_value"):
            return A.Ty(static)
        if tag in ("widen", "wrap", "try"):
            (x,) = types
            if fixed(x):
                if tag == "widen" and not _inside(x.head, static):
                    raise bad("widen needs a source range inside the target")
            elif tag == "widen" or x.head not in ("Int", "Nat"):
                raise bad("a conversion needs a fixed-width, Int or Nat operand")
            return A.Ty("Option", (A.Ty(static),)) if tag == "try" else A.Ty(static)
        if tag in ("to_int", "to_nat"):
            (x,) = types
            if not fixed(x) or (tag == "to_nat" and signed(x)):
                raise bad("to_int needs a fixed-width operand and to_nat an unsigned one")
            return INT if tag == "to_int" else NAT
        if tag in ("from_be", "from_le"):
            (x,) = types
            if x != A.Ty("Bytes", (FIXED[static][0] // 8,)):
                raise bad("from_be/from_le need Bytes of exactly the target's width")
            return A.Ty(static)
        if tag in ("bit_not", "popcount", "clz", "ctz", "be_bytes", "le_bytes"):
            (x,) = types
            if not fixed(x):
                raise bad("a bit operation needs a fixed-width operand")
            if tag == "bit_not":
                return x
            if tag in ("be_bytes", "le_bytes"):
                return A.Ty("Bytes", (FIXED[x.head][0] // 8,))
            return NAT
        if tag in ("shl", "shr", "sar", "rotl", "rotr", "bit"):
            x, n = types
            if not fixed(x) or n != NAT:
                raise bad("a shift needs a fixed-width operand and a Nat count")
            if tag == "sar" and not signed(x):
                raise bad("sar is for signed operands")
            return BOOL if tag == "bit" else x
        if tag in ("bit_and", "bit_or", "bit_xor", "checked_add", "checked_sub", "checked_mul"):
            a, b = types
            if not fixed(a) or a != b:
                raise bad("operands must be fixed-width values of one type")
            return a if tag.startswith("bit_") else A.Ty("Option", (a,))
        if tag in ("div", "rem"):
            a, b = types
            if fixed(a) or fixed(b):
                if a != b or signed(a):
                    raise bad("div and rem take two unsigned operands of one type, or Int/Nat")
                return A.Ty("Option", (a,))
            if a.head not in ("Int", "Nat") or b.head not in ("Int", "Nat"):
                raise bad("div and rem take two unsigned operands of one type, or Int/Nat")
            return A.Ty("Option", (INT,))
        if tag in ("div_trunc", "rem_trunc"):
            a, b = types
            if not fixed(a) or a != b or not signed(a):
                raise bad("div_trunc and rem_trunc take two signed operands of one type")
            return A.Ty("Option", (a,))
        if tag in ("imin", "imax"):
            if any(t.head not in ("Int", "Nat") for t in types):
                raise bad("imin and imax take Int or Nat operands")
            return INT
        if tag == "at":
            b, i = types
            if b.head != "Bytes" or i.head not in ("Nat", "Int"):
                raise bad("at needs Bytes and a Nat or Int index")
            return A.Ty("Option", (A.Ty("U8"),))
        # unwrap_or(o, d)
        o, d = types
        if o.head != "Option" or not _assignable(d, o.args[0]):
            raise bad("unwrap_or needs an Option and a default of its type")
        return o.args[0]

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
            common = _common(t.ty, e.ty)
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
                    raise SizeError("a Bytes value exceeds MAX_IR_VALUE")
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
            elif _common(a.ty, b.ty) is None:
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
        if not is_node(node):
            continue
        if node[0] == "subject_op":
            return True
        stack.extend(children(node))
    return False


def compile_calls(prog: Program, ir: object, tenv: dict) -> int | Callable:
    """The number of subject calls per case, as ``int`` (static) or ``fn(env)``.

    The count is taken on the *pure trace* (VER04r section 4.4): a subject call
    may sit under a guard only if the guard's condition is pure, so that the
    count never depends on what a subject returned.  Anything else is the
    generation error ``W0701``.  A ``match`` arm is a guard whose condition is the scrutinee
    (the arm taken is decided by the scrutinee's value and the patterns): a subject call in an
    arm needs a pure scrutinee, and then only the taken arm counts.
    """

    def count(node: object, env_types: dict, depth: int = 0) -> int | Callable:
        if depth >= MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            return 0
        tag = node[0]
        if tag == "subject_op":
            parts = [count(a, env_types, depth + 1) for a in node[2]]
            return _sum(1, parts)
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                if has_subject(cond):
                    raise OracleError("generation", "W0701: a subject call under an impure guard")
                cf = prog.compile(cond, env_types, False).fn
                tc, ec = count(then, env_types, depth + 1), count(other, env_types, depth + 1)
                return lambda env: (
                    (tc(env) if callable(tc) else tc)
                    if cf(env)
                    else (ec(env) if callable(ec) else ec)
                )
            return count(cond, env_types, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                if has_subject(left):
                    raise OracleError("generation", "W0701: a subject call under an impure guard")
                lf = prog.compile(left, env_types, False).fn
                rc = count(right, env_types, depth + 1)
                # ``and`` and ``implies`` evaluate the right operand when the left is true
                # (implies: when ``not left`` is false), ``or`` when it is false.
                runs_when = (lambda v: not v) if tag == "or" else (lambda v: bool(v))
                return lambda env: (rc(env) if callable(rc) else rc) if runs_when(lf(env)) else 0
            return count(left, env_types, depth + 1)
        if tag == "match":
            return count_match(node, env_types, depth)
        if tag == "model_call":
            # The arguments are IR; the name is not.  ``inc(bump(a))`` has one call.
            return _sum(0, [count(a, env_types, depth + 1) for a in node[2]])
        return _sum(0, [count(c, env_types, depth + 1) for c in children(node)])

    def count_match(node: list, env_types: dict, depth: int) -> int | Callable:
        scrutinee = node[1]
        arm_subject = any(has_subject(body) for _p, body in match_arms(node))
        if not arm_subject:
            return count(scrutinee, env_types, depth + 1)
        if has_subject(scrutinee):
            raise OracleError("generation", "W0701: a subject call under an impure guard")
        s, arms = prog.match_arms(node, env_types)
        counts = [(m, count(body, {**env_types, **b}, depth + 1)) for m, b, body in arms]
        sf = s.fn

        def taken(env: dict) -> int:
            value = sf(env)
            for matcher, c in counts:
                binds: dict = {}
                if matcher(value, binds):
                    return c({**env, **binds}) if callable(c) else c
            raise OracleError("ir", "a match fell through")  # pragma: no cover (exhaustive)

        return taken

    return count(ir, tenv)


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
        if not is_node(node):
            continue
        tag = node[0]
        if tag in _ARITH_TAGS:
            return True
        if tag == "model_call" and node[1] not in seen and node[1] in prog.models:
            seen.add(node[1])
            if may_wrap(prog, prog.models[node[1]].ir, seen):
                return True
        stack.extend(children(node))
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

    def tr(node: object, env_types: dict, depth: int = 0) -> list[Callable]:
        if depth >= MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            if may_wrap(prog, node):
                return [prog.compile(node, env_types, False).fn]
            return []
        tag = node[0]
        if tag == "subject_op" or tag == "model_call":
            return [f for a in node[2] for f in tr(a, env_types, depth + 1)]
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                cf = prog.compile(cond, env_types, False).fn
                tt, et = tr(then, env_types, depth + 1), tr(other, env_types, depth + 1)

                def run_if(env: dict) -> None:
                    for f in tt if cf(env) else et:
                        f(env)

                return [run_if]
            return tr(cond, env_types, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                lf = prog.compile(left, env_types, False).fn
                rt = tr(right, env_types, depth + 1)
                runs = (lambda v: not v) if tag == "or" else bool

                def run_sc(env: dict) -> None:
                    if runs(lf(env)):
                        for f in rt:
                            f(env)

                return [run_sc]
            return tr(left, env_types, depth + 1)
        if tag == "match":
            return trace_match(node, env_types, depth)
        return [f for c in children(node) for f in tr(c, env_types, depth + 1)]

    def trace_match(node: list, env_types: dict, depth: int) -> list[Callable]:
        scrutinee = node[1]
        if not any(has_subject(body) for _p, body in match_arms(node)):
            return tr(scrutinee, env_types, depth + 1)
        if has_subject(scrutinee):
            raise OracleError("generation", "W0701: a subject call under an impure guard")
        s, arms = prog.match_arms(node, env_types)
        traces = [(m, tr(body, {**env_types, **b}, depth + 1)) for m, b, body in arms]
        sf = s.fn

        def run_match(env: dict) -> None:
            value = sf(env)
            for matcher, fns in traces:
                binds: dict = {}
                if matcher(value, binds):
                    inner = {**env, **binds}
                    for f in fns:
                        f(inner)
                    return

        return [run_match]

    fns = tr(ir, tenv)
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
        if not is_node(node):
            continue
        if node[0] == "var":
            seen.add(node[1])
        else:
            stack.extend(children(node))
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


def compiled_admit(prog: Program, op: object, widths: list | None) -> Compiled | None:
    """The compiled ``admit`` of subject op ``op`` for static ``widths``, or ``None``.

    Compiled **once per** ``(op, widths)`` and kept on ``prog``: the D19 check (one closure per
    call site) and its static cost (:func:`admit_check_cost`) used to compile the same row
    again at every call site.  ``widths`` give the ``Bytes<N>`` parameters their types; two
    call sites with the same widths share one :class:`Compiled`, whose closure is pure.
    """
    if op.admit is None:
        return None
    key = (op.name, tuple(widths or ()))
    hit = prog._admits.get(key)
    if hit is None:
        statics = dict(zip(op.statics, widths or []))
        ptenv = {n: _substitute(t, statics) for n, t in op.params}
        hit = prog.compile(op.admit, ptenv, False)
        prog._admits[key] = hit
    return hit


def walk_scoped(prog: Program, ir: object, env_types: dict, allow_subject: bool = True):
    """Yield ``(node, types)`` for every node of ``ir``; ``types`` is the scope the node is typed in.

    Inside a ``match`` arm the scope gains the arm's pattern binders (their types come from the
    scrutinee and the patterns), so an analysis that must *compile* a sub-expression (to learn its
    static type) can do so there.  Patterns are never yielded.
    """
    stack = [(ir, env_types)]
    while stack:
        node, types = stack.pop()
        if not is_node(node):
            continue
        yield node, types
        if node[0] == "match":
            _s, arms = prog.match_arms(node, types, allow_subject)
            stack.append((node[1], types))
            stack.extend((body, {**types, **binders}) for _m, binders, body in arms)
        else:
            stack.extend((c, types) for c in children(node))


def admit_check_cost(prog: Program, ir: object, tenv: dict) -> int:
    """The IR work of [D19]'s per-case check for ``ir``: for every ``subject_op`` whose op has
    an ``admit``, the worst-path cost of that ``admit`` plus the cost of each argument that
    does not call the subject.  A static sum over the whole tree, charged per product ordinal
    to the IR-work meter and **not** part of the recorded ``ir_cost``.
    """
    total = 0
    for node, types in walk_scoped(prog, ir, tenv):
        if node[0] == "subject_op":
            op = prog.ops[node[1]]
            if op.admit is not None:
                total += compiled_admit(prog, op, node[3] if len(node) == 4 else None).cost
                total += sum(prog.compile(a, types, False).cost for a in node[2] if not has_subject(a))
    return total


def admit_fn(prog: Program, op: object, widths: list | None) -> Callable | None:
    """The compiled ``admit`` of subject op ``op`` over its parameter names, or ``None``.

    ``widths`` are the static widths of the call (``Bytes<N>`` parameters take their type
    from them); the closure takes a dict of parameter values.
    """
    compiled = compiled_admit(prog, op, widths)
    return None if compiled is None else compiled.fn


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

    def tr(node: object, env_types: dict, depth: int = 0) -> list[Callable]:
        if depth >= MAX_IR_DEPTH:
            raise OracleError("limit", "IR nesting exceeds 64 levels")
        if not has_subject(node):
            return []
        tag = node[0]
        if tag == "subject_op":
            kids = [f for a in node[2] for f in tr(a, env_types, depth + 1)]
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
                (n, prog.compile(a, env_types, False).fn)
                for n, a in zip(names, node[2])
                if n not in dynamic
            ]

            def check(env: dict, admit=admit, args=args, name=op.name) -> None:
                if not run_admit(prog, admit, {n: f(env) for n, f in args}, v0, name):
                    raise OutsideAdmit(name)

            return [check] + kids
        if tag == "model_call":
            return [f for a in node[2] for f in tr(a, env_types, depth + 1)]
        if tag == "if":
            cond, then, other = node[1], node[2], node[3]
            if has_subject(then) or has_subject(other):
                cf = prog.compile(cond, env_types, False).fn
                tt, et = tr(then, env_types, depth + 1), tr(other, env_types, depth + 1)

                def run_if(env: dict) -> None:
                    for f in tt if cf(env) else et:
                        f(env)

                return [run_if] if tt or et else []
            return tr(cond, env_types, depth + 1)
        if tag in ("and", "or", "implies"):
            left, right = node[1], node[2]
            if has_subject(right):
                lf = prog.compile(left, env_types, False).fn
                rt = tr(right, env_types, depth + 1)
                runs = (lambda v: not v) if tag == "or" else bool

                def run_sc(env: dict) -> None:
                    if runs(lf(env)):
                        for f in rt:
                            f(env)

                return [run_sc] if rt else []
            return tr(left, env_types, depth + 1)
        if tag == "match":
            return check_match(node, env_types, depth)
        return [f for c in children(node) for f in tr(c, env_types, depth + 1)]

    def check_match(node: list, env_types: dict, depth: int) -> list[Callable]:
        scrutinee = node[1]
        if not any(has_subject(body) for _p, body in match_arms(node)):
            return tr(scrutinee, env_types, depth + 1)
        if has_subject(scrutinee):
            raise OracleError("generation", "W0701: a subject call under an impure guard")
        s, arms = prog.match_arms(node, env_types)
        checks = [(m, tr(body, {**env_types, **b}, depth + 1)) for m, b, body in arms]
        sf = s.fn

        def run_match(env: dict) -> None:
            value = sf(env)
            for matcher, fns in checks:
                binds: dict = {}
                if matcher(value, binds):
                    inner = {**env, **binds}
                    for f in fns:
                        f(inner)
                    return

        return [run_match] if any(fns for _m, fns in checks) else []

    fns = tr(ir, tenv)
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
        result = False
        for node, types in walk_scoped(prog, m.ir, dict(m.params), allow_subject=False):
            tag = node[0]
            if (
                tag in _FIXED_ARITH_TAGS
                and prog.compile(node[1], types, False).ty.head in FIXED
                or tag == "model_call"
                and model_has_fixed(node[1])
            ):
                result = True
                break
        closure[name] = result
        return result

    for node, types in walk_scoped(prog, ir, tenv):
        tag = node[0]
        if tag in _FIXED_ARITH_TAGS and has_subject(node):
            if prog.compile(node[1], types, True).ty.head in FIXED:
                return True
        elif tag == "model_call" and has_subject(node) and model_has_fixed(node[1]):
            return True
    return False
