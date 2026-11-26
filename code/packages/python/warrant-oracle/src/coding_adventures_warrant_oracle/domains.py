"""Domain descriptors, ``size``/``nth``, budget demotion (VER04r section 5; VER04h 3).

A *descriptor tree* names, for one quantified binder, the finite list of values
the claim is checked over.  Every descriptor defines two total functions:

``size``   how many values it has;
``nth(i)`` the ``i``-th value (0-based), computed in ``O(depth + value size)``
           without enumerating the values before it.

``nth`` is what makes *chunk jumping* possible: a lane (or this oracle) can start
enumerating at product ordinal ``c * chunk_size`` without touching earlier chunks.

The leaf orders are the ones the specifications fix, written out once:

=============  ===========================================================
``bool``       ``false, true``
``u8_all``     ``0..255``
``u64_edge``   ``0..255``; then ``1<<i`` for ``i = 8..63``; then ``MAX``;
               then ``MAX ^ (1<<i)`` for ``i = 0..63``            (377 values)
``fx_full``    every value of an 8- or 16-bit type, ascending
``fx_edge``    the same edge pattern at width ``w``; for a **signed** type the set
               ``wrap_S(E_U(w)) | {-255..-1}``, de-duplicated and ascending
               [D21]                       (u16: 281, u32: 313; i16: 527, i32: 559, i64: 623).
               16-bit only by budget demotion, never for 8 bits [D25]
``nat_count``  ``0..K``                                     (``nat_0_64``: K=64)
``nat_shift``  ``0..64`` then ``65 127 128 129 255 256 65535 65536`` (73 values) [D26]; a
               ``Nat`` binder that *is* the count operand of ``shl shr sar rotl rotr bit``
``bytes_d1``   the 347-string set of VER04h section 3
``bytes_n``    ``Bytes<N>``: the length-``N`` members of ``bytes_d1`` for
               ``N <= 3``, else ``4 + 3N`` strings (fills, then one-position edits)
=============  ===========================================================

Composites follow VER04r section 5.1: a tuple/record is the Cartesian product with
the *last* component varying fastest; a variant is the concatenation of each
constructor's field product; an option is ``None`` then ``Some(x)``; a result is
``Ok(x)`` for each ``x`` then each error case; a list is every list of length 0,
then lengths 1 and 2 over the first 16 values, then length 3 over the first 4.

Values are plain Python data: ``bool``, ``int``, ``bytes``, tuples (records and
tuples), ``("none",)``/``("some", v)`` options, ``("ok", v)``/``("err", name)``
results and ``(ctor_name, fields)`` variants.
"""

from __future__ import annotations

import itertools
import re
from dataclasses import dataclass

from . import ast as A
from .errors import OracleError
from .irtree import children, is_node
from .lower import FIXED, Spec

MAX_DESC_DEPTH = 16
MAX_SUITE_PRODUCT = 1 << 28
DOMAIN_BYTES_BUDGET = 64 << 20
VALUE_OVERHEAD = 32
MAX_LIST_ELEMENTS = 4096
_ALPHABET = (0, 1, 128, 255)


def _edge_unsigned(width: int) -> list[int]:
    """``E_U(w)``: small values, single bits, MAX, then MAX with one bit cleared."""
    maximum = (1 << width) - 1
    head = list(range(min(256, 1 << width)))
    bits = [1 << i for i in range(8, width)]
    return head + bits + [maximum] + [maximum ^ (1 << i) for i in range(width)]


def _bytes_d1() -> list[bytes]:
    out = [b""] + [bytes((i,)) for i in range(256)]
    for n in (2, 3):
        out += [bytes(p) for p in itertools.product(_ALPHABET, repeat=n)]
    for n in range(4, 9):
        out += [bytes([v]) * n for v in (0, 0xA5)]
    return out


_D1 = _bytes_d1()


def _bytes_n(n: int) -> list[bytes]:
    if n <= 3:
        return [v for v in _D1 if len(v) == n]
    out = [bytes([v]) * n for v in _ALPHABET]
    for pos in range(n):
        for symbol in (1, 128, 255):
            value = bytearray(n)
            value[pos] = symbol
            out.append(bytes(value))
    return out


class Desc:
    """Base class: a finite ordered set of values of one type."""

    ty: A.Ty
    label = "full"  # full | bounded-set | sample

    def size(self) -> int:  # pragma: no cover - abstract
        raise NotImplementedError

    def nth(self, i: int) -> object:  # pragma: no cover - abstract
        raise NotImplementedError

    def to_json(self) -> dict:  # pragma: no cover - abstract
        raise NotImplementedError

    def depth(self) -> int:
        return 1

    def bytes_cost(self) -> int:
        """A safe over-estimate of the bytes one value costs (domain byte budget)."""
        return VALUE_OVERHEAD


class _Listed(Desc):
    """A leaf backed by an explicit list (every leaf except the composites)."""

    def __init__(
        self, name: str, ty: A.Ty, values: list, label: str, extra: dict | None = None
    ) -> None:
        self.name = name
        self.ty = ty
        self.values = values
        self.label = label
        self.extra = extra or {}

    def size(self) -> int:
        return len(self.values)

    def nth(self, i: int) -> object:
        if not 0 <= i < len(self.values):
            raise IndexError("domain index out of range")
        return self.values[i]

    def to_json(self) -> dict:
        return {"d": self.name, **self.extra, "size": len(self.values)}

    def bytes_cost(self) -> int:
        if self.ty.head == "Bytes":
            longest = max((len(v) for v in self.values), default=0)
            return VALUE_OVERHEAD + longest
        return VALUE_OVERHEAD


def leaf_bool() -> Desc:
    return _Listed("bool", A.Ty("Bool"), [False, True], "full")


def leaf_u8_all() -> Desc:
    return _Listed("u8_all", A.Ty("U8"), list(range(256)), "full")


def leaf_u64_edge() -> Desc:
    return _Listed("u64_edge", A.Ty("U64"), _edge_unsigned(64), "bounded-set")


def leaf_nat_count(k: int) -> Desc:
    if not 0 <= k <= 4096:
        raise OracleError("generation", "nat_count K must be 0..4096")
    name = "nat_0_64" if k == 64 else "nat_count"
    extra = {} if k == 64 else {"k": k}
    return _Listed(name, A.Ty("Nat"), list(range(k + 1)), "bounded-set", extra)


def leaf_bytes_d1() -> Desc:
    return _Listed("bytes_d1", A.Ty("Bytes"), list(_D1), "bounded-set")


def leaf_bytes_n(n: int) -> Desc:
    if not 0 <= n <= 4096:
        raise OracleError("generation", "W0707: Bytes width exceeds 4096")
    # VER04r section 9.3: ``Bytes<0>`` and ``Bytes<1>`` are ``full``; ``bytes_n(N >= 2)``
    # is a ``sample`` (VER04i called it ``bounded-set``; VER04j 14.4 and VER04r
    # follow the later and stricter word).
    label = "full" if n <= 1 else "sample"
    return _Listed("bytes_n", A.Ty("Bytes", (n,)), _bytes_n(n), label, {"n": n})


def leaf_fx_full(ty: A.Ty) -> Desc:
    bits, signed = FIXED[ty.head]
    if bits > 16:
        raise OracleError("generation", "fx_full is for 8- and 16-bit types")
    lo, hi = (-(1 << (bits - 1)), (1 << (bits - 1))) if signed else (0, 1 << bits)
    return _Listed("fx_full", ty, list(range(lo, hi)), "full", {"t": ty.head})


#: The boundary shift counts a ``nat_shift`` adds to ``0..=64`` (VER04j 3.6, [D26]).
_SHIFT_EXTRAS = (65, 127, 128, 129, 255, 256, 65535, 65536)


def leaf_nat_shift() -> Desc:
    """``{"d":"nat_shift","size":73}``: the shift-count domain of VER04j 3.6 [D26] (a leaf, no ``k``)."""
    return _Listed("nat_shift", A.Ty("Nat"), [*range(65), *_SHIFT_EXTRAS], "bounded-set")


def edge_values(ty: A.Ty) -> list[int]:
    """The ``fx_edge`` value list of a 16-, 32- or 64-bit fixed-width type (VER04r 5.1, [D21], [D25]).

    Unsigned width ``w``: ``E_U(w)`` in the order of :func:`_edge_unsigned`.  Signed: those bit
    patterns read in two's complement, **plus** ``-255..=-1``, de-duplicated and in **ascending
    numeric order** (``I16`` 527, ``I32`` 559, ``I64`` 623).  An 8-bit type has no ``fx_edge``
    [D25]: the unsigned formula repeats values there and no resolver produces one.
    """
    bits, signed = FIXED[ty.head]
    if bits < 16:
        raise OracleError("generation", "fx_edge is illegal for 8-bit types (VER04r 5.1, D25)")
    values = _edge_unsigned(bits)
    if not signed:
        return values
    half, span = 1 << (bits - 1), 1 << bits
    negatives = range(-255, 0)
    return sorted({v - span if v >= half else v for v in values} | set(negatives))


def leaf_fx_edge(ty: A.Ty) -> Desc:
    return _Listed("fx_edge", ty, edge_values(ty), "bounded-set", {"t": ty.head})


class TupleDesc(Desc):
    """Cartesian product, last component fastest (tuples and records)."""

    def __init__(self, ty: A.Ty, parts: list[Desc], record: str | None = None) -> None:
        self.ty = ty
        self.parts = parts
        self.record = record
        self._size: int | None = None
        self.label = _weakest(p.label for p in parts)

    def size(self) -> int:
        if self._size is None:
            total = 1
            for p in self.parts:
                total = _checked_mul(total, p.size())
            self._size = total
        return self._size

    def nth(self, i: int) -> object:
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        out = []
        for p in reversed(self.parts):
            i, r = divmod(i, p.size())
            out.append(p.nth(r))
        return tuple(reversed(out))

    def to_json(self) -> dict:
        return {"d": "tuple", "of": [p.to_json() for p in self.parts], "size": self.size()}

    def depth(self) -> int:
        return 1 + max(p.depth() for p in self.parts)

    def bytes_cost(self) -> int:
        return VALUE_OVERHEAD + sum(p.bytes_cost() for p in self.parts)


class VariantDesc(Desc):
    def __init__(self, ty: A.Ty, ctors: list[tuple[str, list[Desc]]]) -> None:
        self.ty = ty
        self.ctors = ctors
        self._sizes: list[int] = []
        for _, fields in ctors:
            total = 1
            for f in fields:
                total = _checked_mul(total, f.size())
            self._sizes.append(total)
        self.label = (
            _weakest(f.label for _, fs in ctors for f in fs)
            if any(fs for _, fs in ctors)
            else "full"
        )

    def size(self) -> int:
        return sum(self._sizes)

    def nth(self, i: int) -> object:
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        for (name, fields), n in zip(self.ctors, self._sizes):
            if i < n:
                out = []
                for f in reversed(fields):
                    i, r = divmod(i, f.size())
                    out.append(f.nth(r))
                return (name, tuple(reversed(out)))
            i -= n
        raise IndexError("domain index out of range")  # pragma: no cover

    def to_json(self) -> dict:
        return {
            "d": "variant",
            "ctors": [[n, [f.to_json() for f in fs]] for n, fs in self.ctors],
            "size": self.size(),
        }

    def depth(self) -> int:
        return 1 + max((f.depth() for _, fs in self.ctors for f in fs), default=0)

    def bytes_cost(self) -> int:
        return VALUE_OVERHEAD + max(
            (sum(f.bytes_cost() for f in fs) for _, fs in self.ctors), default=0
        )


class OptionDesc(Desc):
    def __init__(self, ty: A.Ty, inner: Desc) -> None:
        self.ty = ty
        self.inner = inner
        self.label = _weakest([inner.label])

    def size(self) -> int:
        return 1 + self.inner.size()

    def nth(self, i: int) -> object:
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        return ("none",) if i == 0 else ("some", self.inner.nth(i - 1))

    def to_json(self) -> dict:
        return {"d": "option", "of": self.inner.to_json(), "size": self.size()}

    def depth(self) -> int:
        return 1 + self.inner.depth()

    def bytes_cost(self) -> int:
        return VALUE_OVERHEAD + self.inner.bytes_cost()


class ResultDesc(Desc):
    def __init__(self, ty: A.Ty, ok: Desc, errs: list[str]) -> None:
        self.ty = ty
        self.ok = ok
        self.errs = errs
        self.label = ok.label

    def size(self) -> int:
        return self.ok.size() + len(self.errs)

    def nth(self, i: int) -> object:
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        n = self.ok.size()
        return ("ok", self.ok.nth(i)) if i < n else ("err", self.errs[i - n])

    def to_json(self) -> dict:
        return {
            "d": "result",
            "ok": self.ok.to_json(),
            "errs": list(self.errs),
            "size": self.size(),
        }

    def depth(self) -> int:
        return 1 + self.ok.depth()

    def bytes_cost(self) -> int:
        return VALUE_OVERHEAD + self.ok.bytes_cost()


class ListDesc(Desc):
    """Lists of length 0; 1 and 2 over the first 16 values; 3 over the first 4."""

    def __init__(self, ty: A.Ty, inner: Desc) -> None:
        self.ty = ty
        self.inner = inner
        self.d16 = min(16, inner.size())
        self.d4 = min(4, inner.size())
        self.label = "bounded-set" if inner.label != "sample" else "sample"

    def _sizes(self) -> list[int]:
        return [1, self.d16, self.d16**2, self.d4**3]

    def size(self) -> int:
        return sum(self._sizes())

    def nth(self, i: int) -> object:
        if not 0 <= i < self.size():
            raise IndexError("domain index out of range")
        for length, n in enumerate(self._sizes()):
            if i < n:
                base = self.d16 if length <= 2 else self.d4
                idx = []
                for _ in range(length):
                    i, r = divmod(i, base)
                    idx.append(r)
                return ("list", tuple(self.inner.nth(r) for r in reversed(idx)))
            i -= n
        raise IndexError("domain index out of range")  # pragma: no cover

    def to_json(self) -> dict:
        return {"d": "list", "of": self.inner.to_json(), "size": self.size()}

    def depth(self) -> int:
        return 1 + self.inner.depth()

    def bytes_cost(self) -> int:
        return VALUE_OVERHEAD + 3 * self.inner.bytes_cost()


def _weakest(labels) -> str:
    order = {"full": 0, "bounded-set": 1, "sample": 2}
    return max(labels, key=lambda lab: order[lab], default="full")


def _checked_mul(a: int, b: int) -> int:
    """Domain arithmetic is exact; the bound is the 2**28 product cap."""
    r = a * b
    if r > MAX_SUITE_PRODUCT * (1 << 20):
        raise OracleError("limit", "a domain product is far beyond budget")
    return r


# -- from a type to a descriptor ------------------------------------------------


def descriptor_for(spec: Spec, ty: A.Ty, depth: int = 0) -> Desc:
    """The (undemoted) descriptor of a binder type (VER04h section 3, VER04j 2.6/4.7)."""
    # VER04r 14: a path has at most 16 *nodes*, a leaf alone counting as one; ``depth`` is the
    # number of ancestors, so this node is the ``depth + 1``-th on its path (as ``descriptor_size``).
    if depth + 1 > MAX_DESC_DEPTH:
        raise OracleError("limit", "descriptor tree deeper than 16 levels")
    head = ty.head
    if head == "Bool":
        return leaf_bool()
    if head == "U8":
        return leaf_u8_all()
    if head == "U64":
        return leaf_u64_edge()
    if head in FIXED:
        bits = FIXED[head][0]
        return leaf_fx_full(ty) if bits <= 16 else leaf_fx_edge(ty)
    if head == "Nat":
        return leaf_nat_count(64)
    if head == "Bytes":
        if not ty.args:
            return leaf_bytes_d1()
        width = ty.args[0]
        if not isinstance(width, int):
            raise OracleError("generation", "a binder cannot have a named width")
        return leaf_bytes_n(width)
    if head == "()":
        return TupleDesc(ty, [descriptor_for(spec, t, depth + 1) for t in ty.args])
    if head == "Option":
        return OptionDesc(ty, descriptor_for(spec, ty.args[0], depth + 1))
    if head == "List":
        return ListDesc(ty, descriptor_for(spec, ty.args[0], depth + 1))
    if head == "Result":
        return ResultDesc(
            ty, descriptor_for(spec, ty.args[0], depth + 1), list(spec.errors[ty.args[1].head])
        )
    if head in spec.records:
        parts = [descriptor_for(spec, t, depth + 1) for _, t in spec.records[head]]
        return TupleDesc(ty, parts, record=head)
    if head in spec.variants:
        ctors = [
            (name, [descriptor_for(spec, t, depth + 1) for _, t in fields])
            for name, fields in spec.variants[head]
        ]
        return VariantDesc(ty, ctors)
    raise OracleError("generation", "a binder of this type has no domain")


#: The built-ins whose second operand is a shift count or bit index (VER04j 3.6, [D26]).
COUNT_TAGS = frozenset(["shl", "shr", "sar", "rotl", "rotr", "bit"])


def count_binders(body: object) -> set[str]:
    """The names a claim body uses **directly** as the count operand of a shift, rotation or ``bit``.

    VER04r 5.1 and 16.10 R10: only the count operand itself selects ``nat_shift``.  ``shl(x, n)``
    selects ``n``; ``shl(x, n + 1)``, a ``model_call`` argument and a count that is a projection
    or any other expression do not, and neither does a binder that reaches a count only through a
    model body or an ``admit`` (the caller passes the claim *body*, never its ``admit``).  The walk
    uses :func:`.irtree.children`, the one place that knows which parts of a node are
    sub-expressions (a tuple literal's operands, a ``match`` arm body; never a pattern).
    """
    found: set[str] = set()
    stack = [body]
    while stack:
        node = stack.pop()
        if not is_node(node):
            continue
        if node[0] in COUNT_TAGS and len(node) == 3:
            count = node[2]
            if is_node(count) and count[0] == "var" and len(count) == 2 and isinstance(count[1], str):
                found.add(count[1])
        stack.extend(children(node))
    return found


def refine_counts(descs: list[Desc], binders: list, body: object, version: str) -> list[str]:
    """Give every ``Nat`` binder that is directly a shift count the ``nat_shift`` leaf (in place).

    ``binders`` is the suite's ``[(name, Ty)]``.  Only a binder whose declared type is exactly
    ``Nat`` is refined, and only in a ``v1`` document (a ``v0`` document has no shifts), before
    any budget demotion.  Returns the refined binder names in binder order.
    """
    if version == "v0":
        return []
    counts = count_binders(body)
    refined = []
    for k, (name, ty) in enumerate(binders):
        if ty.head == "Nat" and not ty.args and name in counts:
            descs[k] = leaf_nat_shift()
            refined.append(name)
    return refined


#: The name grammar of VER04r section 2.2 rule 4 (an error case name, ``errs`` of a ``result`` descriptor).
_CASE_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def descriptor_size(d: object, depth: int = 1) -> int:
    """Recompute the ``size`` of a descriptor tree and check it as a consumer must (VER04r 5.1).

    Every node's declared ``size`` must equal the recomputed one (checked arithmetic: a Python
    ``int`` cannot overflow, and a size past ``2**64`` is refused as ``W0706`` would be), the tree
    is at most 16 levels deep, and the legality rules of the leaves are enforced: ``fx_full`` is
    for 8- and 16-bit types other than ``U8``; ``fx_edge`` for 16-, 32- and 64-bit types other
    than ``U64``, **never 8-bit** [D25]; ``nat_shift`` has exactly 73 values [D26].  Returns the
    size; raises ``OracleError("generation", ..)`` otherwise.
    """
    if depth > MAX_DESC_DEPTH:
        raise OracleError("limit", "descriptor tree deeper than 16 levels")
    if not isinstance(d, dict) or not isinstance(d.get("d"), str) or type(d.get("size")) is not int:
        raise OracleError("generation", "a malformed descriptor")
    kind = d["d"]

    def need(keys: set) -> None:
        if set(d) != {"d", "size", *keys}:
            raise OracleError("generation", "a descriptor has the wrong keys")

    def fixed_type() -> A.Ty:
        name = d.get("t")
        if name not in FIXED:
            raise OracleError("generation", "a descriptor names no fixed-width type")
        return A.Ty(name)

    if kind in ("bool", "u8_all", "u64_edge", "nat_0_64", "bytes_d1", "nat_shift"):
        need(set())
        size = {
            "bool": 2,
            "u8_all": 256,
            "u64_edge": len(_edge_unsigned(64)),
            "nat_0_64": 65,
            "bytes_d1": len(_D1),
            "nat_shift": 73,
        }[kind]
    elif kind == "bytes_n":
        need({"n"})
        if type(d["n"]) is not int or not 0 <= d["n"] <= 4096:
            raise OracleError("generation", "bytes_n needs a width in 0..4096")
        size = len(_bytes_n(d["n"]))
    elif kind == "nat_count":
        need({"k"})
        if type(d["k"]) is not int or not 0 <= d["k"] <= 4096:
            raise OracleError("generation", "nat_count needs K in 0..4096")
        size = d["k"] + 1
    elif kind == "fx_full":
        need({"t"})
        ty = fixed_type()
        if ty.head == "U8" or FIXED[ty.head][0] > 16:
            raise OracleError("generation", "fx_full is for 8- and 16-bit types other than U8")
        size = 1 << FIXED[ty.head][0]
    elif kind == "fx_edge":
        need({"t"})
        ty = fixed_type()
        if ty.head == "U64" or FIXED[ty.head][0] < 16:
            raise OracleError(
                "generation", "fx_edge is for 16-, 32- and 64-bit types other than U64 (D25)"
            )
        size = len(edge_values(ty))
    elif kind == "tuple":
        need({"of"})
        parts = d["of"]
        # VER04r 5.1 and 16.20 R103 (b), CV2-F17: 1 to 32 entries (the record bound, 2.2 rule 3).  The
        # exact component or field count of the binder type is the caller's, where the type is known.
        # (The wording for an empty or non-list ``of`` is the committed one: data_eval_v1.oracle-domains.json.)
        if not isinstance(parts, list) or not parts:
            raise OracleError("generation", "a tuple descriptor has components")
        if len(parts) > 32:
            raise OracleError("generation", "a tuple descriptor has at most 32 components")
        size = 1
        for part in parts:
            size = _checked_size(size * descriptor_size(part, depth + 1))
    elif kind == "option":
        need({"of"})
        size = _checked_size(1 + descriptor_size(d["of"], depth + 1))
    elif kind == "result":
        need({"ok", "errs"})
        errs = d["errs"]
        if not isinstance(errs, list) or not all(isinstance(e, str) for e in errs):
            raise OracleError("generation", "a result descriptor lists error cases")
        # VER04r 5.1 and 16.18 R72 (CV2-F17): 1 to 64 names of the name grammar, no duplicates.
        if not 1 <= len(errs) <= 64:
            raise OracleError("generation", "a result descriptor lists 1 to 64 error cases")
        if len(set(errs)) != len(errs):
            raise OracleError("generation", "a result descriptor repeats an error case")
        if not all(_CASE_NAME.fullmatch(e) for e in errs):
            raise OracleError("generation", "an error case name is outside the name grammar")
        size = _checked_size(descriptor_size(d["ok"], depth + 1) + len(d["errs"]))
    elif kind == "list":
        need({"of"})
        inner = descriptor_size(d["of"], depth + 1)
        d16, d4 = min(16, inner), min(4, inner)
        size = _checked_size(1 + d16 + d16**2 + d4**3)
    elif kind == "variant":
        need({"ctors"})
        # VER04r 5.1 and 16.20 R103 (a), CV2-F17: 2 to 64 constructors, each ``[name, [desc..]]`` with a
        # name of the name grammar (no duplicates) and 0 to 8 fields.  Equality with the declared
        # constructors is the caller's, where the ``types`` table is present.
        # (The wording for an empty or non-list ``ctors`` is the committed one, as for a tuple.)
        if not isinstance(d["ctors"], list) or not d["ctors"]:
            raise OracleError("generation", "a variant descriptor has constructors")
        if not 2 <= len(d["ctors"]) <= 64:
            raise OracleError("generation", "a variant descriptor lists 2 to 64 constructors")
        size = 0
        seen: set[str] = set()
        for ctor in d["ctors"]:
            if not (isinstance(ctor, list) and len(ctor) == 2 and isinstance(ctor[1], list)):
                raise OracleError("generation", "a malformed variant constructor")
            if not isinstance(ctor[0], str) or not _CASE_NAME.fullmatch(ctor[0]):
                raise OracleError("generation", "a constructor name is not a name of the name grammar")
            if ctor[0] in seen:
                raise OracleError("generation", "a variant descriptor repeats a constructor")
            seen.add(ctor[0])
            if len(ctor[1]) > 8:
                raise OracleError("generation", "a constructor has 0 to 8 field descriptors")
            fields = 1
            for field in ctor[1]:
                fields = _checked_size(fields * descriptor_size(field, depth + 1))
            size = _checked_size(size + fields)
    else:
        raise OracleError("generation", "an unknown descriptor kind")
    if d["size"] != size:
        raise OracleError("generation", "a descriptor's size is not its recomputed size")
    return size


def _checked_size(size: int) -> int:
    if size >= 1 << 64:
        raise OracleError("generation", "W0706: a descriptor size does not fit 64 bits")
    return size


def demote(binders: list[Desc], budget: int) -> list[str]:
    """Apply the VER04j section 2.6 demotion rule in place; return what was demoted.

    1. every binder starts at ``full`` where it has one, otherwise ``edge``;
    2. while the product exceeds ``budget``, demote the *leftmost* binder that is
       currently ``fx_full`` of a 16-bit type to ``fx_edge``;
    3. still over budget (or over the byte budget): ``W0706``.

    Composites are demoted leaf by leaf, depth first, left to right (VER04j 14.1
    item 20).  ``U8`` is never demoted.
    """
    demoted: list[str] = []

    def product() -> int:
        total = 1
        for b in binders:
            total *= b.size()
        return total

    def leaves(d: Desc, path: str):
        """Yield ``(path, holder, key)`` for every demotable leaf, depth first."""
        if isinstance(d, TupleDesc):
            for k, p in enumerate(d.parts):
                yield from _leaf_or_recurse(d.parts, k, p, f"{path}.{k}")
        elif isinstance(d, VariantDesc):
            for name, fields in d.ctors:
                for k, f in enumerate(fields):
                    yield from _leaf_or_recurse(fields, k, f, f"{path}.{name}.{k}")
        elif isinstance(d, OptionDesc):
            yield from _leaf_or_recurse_attr(d, "inner", f"{path}.some")
        elif isinstance(d, ResultDesc):
            yield from _leaf_or_recurse_attr(d, "ok", f"{path}.ok")
        elif isinstance(d, ListDesc):
            yield from _leaf_or_recurse_attr(d, "inner", f"{path}.elem")

    def _leaf_or_recurse(seq, k, d, path):
        if _demotable(d):
            yield path, seq, k
        else:
            yield from leaves(d, path)

    def _leaf_or_recurse_attr(obj, attr, path):
        child = getattr(obj, attr)
        if _demotable(child):
            yield path, obj, attr
        else:
            yield from leaves(child, path)

    def _demotable(d: Desc) -> bool:
        return isinstance(d, _Listed) and d.name == "fx_full" and FIXED[d.ty.head][0] == 16

    while product() > budget:
        target = None
        for idx, b in enumerate(binders):
            if _demotable(b):
                target = (str(idx), binders, idx)
                break
            found = next(iter(leaves(b, str(idx))), None)
            if found is not None:
                target = found
                break
        if target is None:
            raise OracleError("generation", "W0706: domain product exceeds the budget")
        path, holder, key = target
        old = holder[key] if not isinstance(key, str) else getattr(holder, key)
        new = leaf_fx_edge(old.ty)
        if isinstance(key, str):
            setattr(holder, key, new)
        else:
            holder[key] = new
        _refresh(binders)
        demoted.append(path)
    return demoted


def _refresh(binders: list[Desc]) -> None:
    """Recompute every cached size after a demotion changed a leaf.

    Strictly bottom-up (post-order): a variant's per-constructor sizes and a list's ``d16``/``d4``
    are cached *from their children's sizes* (and every ``label`` from the children's labels: a demoted
    ``fx_edge`` leaf is ``bounded-set``, so each ancestor must stop saying ``full``), so a child tuple must drop its own cache before the
    parent reads it.  Recomputing a parent first would read the child's stale size (CV2-F17 / O1).
    """

    def walk(d: Desc) -> None:
        if isinstance(d, TupleDesc):
            for p in d.parts:
                walk(p)
            d._size = None
            d.label = _weakest(p.label for p in d.parts)
        elif isinstance(d, VariantDesc):
            for _, fields in d.ctors:
                for f in fields:
                    walk(f)
            d._sizes = []
            for _, fields in d.ctors:
                total = 1
                for f in fields:
                    total *= f.size()
                d._sizes.append(total)
            # the constructor's rule: the weakest field label, or ``full`` with no fields at all
            d.label = (
                _weakest(f.label for _, fs in d.ctors for f in fs)
                if any(fs for _, fs in d.ctors)
                else "full"
            )
        elif isinstance(d, ListDesc):
            walk(d.inner)
            d.d16 = min(16, d.inner.size())
            d.d4 = min(4, d.inner.size())
            d.label = "bounded-set" if d.inner.label != "sample" else "sample"
        elif isinstance(d, OptionDesc):
            walk(d.inner)
            d.label = _weakest([d.inner.label])
        elif isinstance(d, ResultDesc):
            walk(d.ok)
            d.label = d.ok.label

    for b in binders:
        walk(b)


def check_budget(binders: list[Desc], budget: int) -> int:
    """Return the product, raising ``W0706``/``W0707`` before anything is built."""
    total = 1
    for b in binders:
        total = _checked_mul(total, b.size())
    if total > MAX_SUITE_PRODUCT:
        raise OracleError("generation", "W0706: product exceeds MAX_SUITE_PRODUCT (2^28)")
    if total > budget:
        raise OracleError("generation", "W0706: product exceeds the exhaustive budget")
    bytes_total = sum(b.size() * b.bytes_cost() for b in binders)
    if bytes_total > DOMAIN_BYTES_BUDGET:
        raise OracleError("generation", "W0706: domains exceed DOMAIN_BYTES_BUDGET")
    return total


@dataclass
class DomainReport:
    """What a suite's binders enumerate: sizes, labels and demotions."""

    sizes: list[int]
    labels: list[str]
    demoted: list[str]

    @property
    def label(self) -> str:
        return _weakest(self.labels)
