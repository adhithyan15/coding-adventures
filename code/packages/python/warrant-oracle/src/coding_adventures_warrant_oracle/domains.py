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
``fx_edge``    the same edge pattern at width ``w``, reinterpreted in two's
               complement for signed types                (u32: 313, u16: 281)
``nat_count``  ``0..K``                                     (``nat_0_64``: K=64)
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
from dataclasses import dataclass

from . import ast as A
from .errors import OracleError
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


def leaf_fx_edge(ty: A.Ty) -> Desc:
    bits, signed = FIXED[ty.head]
    values = _edge_unsigned(bits)
    if signed:
        values = [v - (1 << bits) if v >= 1 << (bits - 1) else v for v in values]
    return _Listed("fx_edge", ty, values, "bounded-set", {"t": ty.head})


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
    if depth > MAX_DESC_DEPTH:
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
    """Drop cached sizes after a demotion changed a leaf."""
    stack = list(binders)
    while stack:
        d = stack.pop()
        if isinstance(d, TupleDesc):
            d._size = None
            stack.extend(d.parts)
        elif isinstance(d, VariantDesc):
            d._sizes = []
            for _, fields in d.ctors:
                total = 1
                for f in fields:
                    total *= f.size()
                d._sizes.append(total)
                stack.extend(fields)
        elif isinstance(d, (OptionDesc, ListDesc)):
            if isinstance(d, ListDesc):
                d.d16 = min(16, d.inner.size())
                d.d4 = min(4, d.inner.size())
            stack.append(d.inner)
        elif isinstance(d, ResultDesc):
            stack.append(d.ok)


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
