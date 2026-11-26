"""Resolved domain descriptors: ``size()`` and ``nth()`` (VER04r section 5.1).

The corpus never ships its exhaustive inputs (gf256 alone has 51 million of
them).  A binder carries instead a **fully resolved descriptor tree**, emitted
*after* the budget demotion, so that a lane never re-runs demotion (the largest
cause of silent drift between a generator and a consumer).  Each node records
its ``size``; every consumer *recomputes* it and refuses a difference.

======================  =======================================================
node                    values, in order
======================  =======================================================
``bool``                ``false``, ``true``
``u8_all``              ``0..=255``
``u64_edge``            ``0..=255``; ``1<<i`` for ``i = 8..=63``; ``MAX``; then
                        ``MAX ^ (1<<i)`` for ``i = 0..=63``          (377 values)
``nat_0_64``            ``0..=64``
``nat_count`` ``k``     ``0..=k``, ``k <= 4096``
``nat_shift``           ``0..=64``, then ``65 127 128 129 255 256 65535 65536``
``bytes_d1``            ``[]``; each single byte; every length-2 and length-3 string
                        over ``A = [00,01,80,ff]``, lexicographic; for ``L = 4..=8``
                        the pair ``00``x``L``, ``a5``x``L``              (347 values)
``bytes_n`` ``n``       ``n <= 3``: the ``bytes_d1`` members of length ``n``; else the
                        four uniform fills over ``A``, then for each position and each
                        of ``01 80 ff`` the zero string with that one byte set
``fx_full`` ``t``       every value of an 8- or 16-bit type other than ``U8``,
                        ascending (signed from ``MIN`` to ``MAX``)
``fx_edge`` ``t``       see :meth:`.Semantics.fx_edge_values`
tuple / record          Cartesian product, **last component fastest**
variant                 constructors in declaration order, each its field product
option                  ``None``, then ``Some(x)`` for ``x`` in the payload domain
result                  ``Ok(x)`` for ``x`` in the ok domain, then each error case
======================  =======================================================

The ``list`` descriptor of section 5.1 is **not** implemented: lists are the
reserved level ``ir.list`` ([D63]), and a ``list`` node is refused as that level
(``level-missing``) however well-formed it is.

Every descriptor defines two **total** functions, ``size()`` and ``nth(i)`` (the
``i``-th value, ``0 <= i < size``).  ``nth`` is what lets a lane jump straight to
a chunk (section 5.4), so it costs ``O(depth + size of one value)``, never
``O(i)``: ``nth`` of a product is mixed-radix division, ``nth`` of a variant a
walk over its constructors, ``nth`` of a leaf an index or a little arithmetic.

A descriptor tree is at most 16 levels deep, counting nodes on a path (a leaf
alone is one level).
"""

from __future__ import annotations

import re
from collections.abc import Callable
from functools import lru_cache

from .errors import Refusal
from .semantics import STANDARD, Semantics
from .wtypes import BYTES, FIXED, NAT, Ty, check_types_table, parse_type

MAX_DESCRIPTOR_DEPTH = 16
MAX_NAT_COUNT = 4096
MAX_BYTES_N = 4096
_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_ALPHABET = (0x00, 0x01, 0x80, 0xFF)
_SHIFT_VALUES = (*range(65), 65, 127, 128, 129, 255, 256, 65535, 65536)


class Desc:
    """A descriptor: ``size`` and the total function ``nth``."""

    def __init__(self, size: int, nth: Callable[[int], object]) -> None:
        self.size = size
        self._nth = nth

    def nth(self, index: int) -> object:
        """The ``index``-th value; ``index`` must be ``0 <= index < size``."""
        return self._nth(index)


# ---------------------------------------------------------------------------
# The two fixed byte domains
# ---------------------------------------------------------------------------
_D1_SIZE = 1 + 256 + 16 + 64 + 10


def _d1_nth(index: int) -> bytes:
    """The ``index``-th member of ``bytes_d1`` (347 of them), by arithmetic."""
    if index == 0:
        return b""
    if index <= 256:
        return bytes([index - 1])
    index -= 257
    if index < 16:
        return bytes([_ALPHABET[index // 4], _ALPHABET[index % 4]])
    index -= 16
    if index < 64:
        return bytes(
            [_ALPHABET[index // 16], _ALPHABET[(index // 4) % 4], _ALPHABET[index % 4]]
        )
    index -= 64
    length = 4 + index // 2
    return bytes([0x00 if index % 2 == 0 else 0xA5]) * length


def _bytes_n_size(width: int) -> int:
    return (1, 256, 16, 64)[width] if width <= 3 else 4 + 3 * width


def _bytes_n_nth(width: int) -> Callable[[int], bytes]:
    def nth(index: int) -> bytes:
        if width == 0:
            return b""
        if width == 1:
            return bytes([index])
        if width == 2:
            return bytes([_ALPHABET[index // 4], _ALPHABET[index % 4]])
        if width == 3:
            return bytes(
                [
                    _ALPHABET[index // 16],
                    _ALPHABET[(index // 4) % 4],
                    _ALPHABET[index % 4],
                ]
            )
        if index < 4:
            return bytes([_ALPHABET[index]]) * width
        position, which = divmod(index - 4, 3)
        buffer = bytearray(width)
        buffer[position] = (0x01, 0x80, 0xFF)[which]
        return bytes(buffer)

    return nth


@lru_cache(maxsize=1)
def _u64_edge() -> tuple[int, ...]:
    top = (1 << 64) - 1
    return (
        *range(256),
        *(1 << i for i in range(8, 64)),
        top,
        *(top ^ (1 << i) for i in range(64)),
    )


# ---------------------------------------------------------------------------
# Reading and validating a descriptor tree
# ---------------------------------------------------------------------------
def _refuse(why: str) -> Refusal:
    return Refusal("descriptor", why)


def _is_int(value: object) -> bool:
    """A JSON integer: ``bool`` is an ``int`` in Python and is *not* one here."""
    return type(value) is int


def _need_keys(node: dict, extra: set[str]) -> None:
    if set(node) != {"d", "size", *extra}:
        raise _refuse("a descriptor has a key too many or too few for its kind")


def build(
    node: object,
    sem: Semantics = STANDARD,
    depth: int = 1,
) -> Desc:
    """Validate the descriptor tree ``node`` and build it, or refuse ``descriptor``.

    Every refusal of section 5.1 comes from here: an unknown ``d``, a missing or
    extra key, a ``size`` that is not the recomputed one, an illegal ``t``, a tree
    deeper than 16 levels.  (That the tree is one *for* a given type is a separate
    question, :func:`descriptor_for`.)
    """
    if depth > MAX_DESCRIPTOR_DEPTH:
        raise _refuse("the descriptor tree nests past sixteen levels")
    if type(node) is not dict or type(node.get("d")) is not str:
        raise _refuse("a descriptor must be an object that names its kind")
    kind = node["d"]
    desc = _build_leaf(node, kind, sem)
    if desc is None:
        desc = _build_composite(node, kind, sem, depth)
    if not _is_int(node["size"]) or node["size"] != desc.size:
        raise _refuse("the stated size differs from the size computed here")
    return desc


_LEAVES = {
    "bool": (2, lambda i: i == 1),
    "u8_all": (256, lambda i: i),
    "nat_0_64": (65, lambda i: i),
    "bytes_d1": (_D1_SIZE, _d1_nth),
    "nat_shift": (73, lambda i: _SHIFT_VALUES[i]),
}


def _build_leaf(node: dict, kind: str, sem: Semantics) -> Desc | None:
    if kind in _LEAVES:
        _need_keys(node, set())
        size, nth = _LEAVES[kind]
        return Desc(size, nth)
    if kind == "u64_edge":
        _need_keys(node, set())
        return Desc(377, lambda i: _u64_edge()[i])
    if kind == "bytes_n":
        _need_keys(node, {"n"})
        width = node["n"]
        if not _is_int(width) or not 0 <= width <= MAX_BYTES_N:
            raise _refuse("bytes_n's length is outside the allowed range")
        return Desc(_bytes_n_size(width), _bytes_n_nth(width))
    if kind == "nat_count":
        _need_keys(node, {"k"})
        count = node["k"]
        if not _is_int(count) or not 0 <= count <= MAX_NAT_COUNT:
            raise _refuse("nat_count's bound is outside the allowed range")
        return Desc(count + 1, lambda i: i)
    if kind in ("fx_full", "fx_edge"):
        _need_keys(node, {"t"})
        return _fixed_leaf(kind, node["t"], sem)
    return None


def _fixed_leaf(kind: str, name: object, sem: Semantics) -> Desc:
    if not isinstance(name, str) or name not in FIXED:
        raise _refuse("t must name one of the fixed-width integer types")
    bits, signed = FIXED[name]
    ty = Ty(name)
    if kind == "fx_full":
        # Every value of an 8- or 16-bit type; a U8 binder is u8_all everywhere.
        if bits > 16 or name == "U8":
            raise _refuse("fx_full covers only the 8- and 16-bit types, U8 excepted")
        low = -(1 << (bits - 1)) if signed else 0
        return Desc(1 << bits, lambda i: low + i)
    # fx_edge: 16-, 32- and 64-bit types, U64 excluded (that is u64_edge).
    if bits < 16 or name == "U64":
        raise _refuse("fx_edge covers only wider integer types, and not U64")
    values = sem.fx_edge_values(ty)
    return Desc(len(values), lambda i: values[i])


def _children(node: dict, key: str) -> list:
    items = node.get(key)
    if type(items) is not list:
        raise _refuse(f"{key} is an array")
    return items


def _build_composite(node: dict, kind: str, sem: Semantics, depth: int) -> Desc:
    if kind == "tuple":
        _need_keys(node, {"of"})
        parts = [build(p, sem, depth + 1) for p in _children(node, "of")]
        if not 2 <= len(parts) <= 8:
            raise _refuse("a tuple descriptor has too few or too many components")
        return _product(parts)
    if kind == "option":
        _need_keys(node, {"of"})
        inner = build(node["of"], sem, depth + 1)
        return Desc(
            1 + inner.size,
            lambda i: ("none",) if i == 0 else ("some", inner.nth(i - 1)),
        )
    if kind == "result":
        _need_keys(node, {"ok", "errs"})
        ok = build(node["ok"], sem, depth + 1)
        errs = _children(node, "errs")
        if (
            not 1 <= len(errs) <= 64
            or not all(isinstance(e, str) and _NAME.fullmatch(e) for e in errs)
            or len(set(errs)) != len(errs)
        ):
            raise _refuse(
                "errs must list between one and sixty-four "
                "distinct, well-formed case names"
            )
        return Desc(
            ok.size + len(errs),
            lambda i: ("ok", ok.nth(i)) if i < ok.size else ("err", errs[i - ok.size]),
        )
    if kind == "variant":
        return _variant(node, sem, depth)
    if kind == "list":
        raise Refusal(
            "level-missing", "the list descriptor belongs to the level ir.list"
        )
    raise _refuse("no descriptor of that kind exists")


def _product(parts: list[Desc]) -> Desc:
    """The Cartesian product of ``parts``, **last component varying fastest**."""
    size = 1
    for part in parts:
        size *= part.size

    def nth(index: int) -> tuple:
        values: list[object] = [None] * len(parts)
        for position in range(len(parts) - 1, -1, -1):
            index, digit = divmod(index, parts[position].size)
            values[position] = parts[position].nth(digit)
        return tuple(values)

    return Desc(size, nth)


def _variant(node: dict, sem: Semantics, depth: int) -> Desc:
    _need_keys(node, {"ctors"})
    rows = []
    for entry in _children(node, "ctors"):
        if type(entry) is not list or len(entry) != 2 or not isinstance(entry[0], str):
            raise _refuse(
                "a constructor entry is a name plus a list of field descriptors"
            )
        fields = [build(f, sem, depth + 1) for f in _children({"x": entry[1]}, "x")]
        rows.append((entry[0], _product(fields) if fields else Desc(1, lambda _i: ())))
    if not 2 <= len(rows) <= 64:
        raise _refuse("a variant descriptor has too few or too many constructors")

    def nth(index: int) -> tuple:
        for name, fields in rows:
            if index < fields.size:
                return (name, fields.nth(index))
            index -= fields.size
        raise IndexError(index)  # pragma: no cover - callers check the index

    return Desc(sum(f.size for _n, f in rows), nth)


# ---------------------------------------------------------------------------
# Is the tree one FOR this type?
# ---------------------------------------------------------------------------
# A descriptor is a *recipe for the values of one type*, and "one for this type"
# is a question about shape: a ``bool`` leaf is the recipe for ``Bool``, a
# ``tuple`` for a tuple *or* a record, a ``variant`` for a declared variant whose
# constructors it lists by name, and so on.  The check walks the descriptor and
# the type together.  It keeps its own list of pairs still to compare instead of
# calling itself, so the depth of a tree never matters to the stack.
_LEAF_RECIPES: dict[str, Ty] = {
    "bool": Ty("Bool"),
    "u8_all": Ty("U8"),
    "u64_edge": Ty("U64"),
    "nat_0_64": NAT,
    "nat_count": NAT,
    "nat_shift": NAT,
    "bytes_d1": BYTES,
}

Pairs = list  # of (descriptor node, type) still to be compared


def _components_of(ty: Ty, types: dict) -> list[Ty] | None:
    """The component types of a tuple or a record type, else ``None``."""
    if ty.head == "()":
        return list(ty.args)
    row = types.get(ty.head)
    if isinstance(row, dict) and row.get("kind") == "record":
        return [parse_type(text, types) for _name, text in row["fields"]]
    return None


def _pair_up(nodes: list, wanted: list[Ty] | None) -> Pairs | None:
    if wanted is None or len(nodes) != len(wanted):
        return None
    return list(zip(nodes, wanted, strict=True))


def _compare_variant(node: dict, ty: Ty, types: dict) -> Pairs | None:
    row = types.get(ty.head)
    if not isinstance(row, dict) or row.get("kind") != "variant":
        return None
    if [name for name, _fields in row["ctors"]] != [n for n, _f in node["ctors"]]:
        return None
    pairs: Pairs = []
    for (_name, texts), (_dname, nodes) in zip(
        row["ctors"], node["ctors"], strict=True
    ):
        more = _pair_up(nodes, [parse_type(text, types) for text in texts])
        if more is None:
            return None
        pairs += more
    return pairs


def _compare_result(node: dict, ty: Ty, types: dict) -> Pairs | None:
    if ty.head != "Result":
        return None
    row = types.get(ty.args[1].head)
    # With a types table the descriptor must list the error type's cases, in the
    # order the type declares them; without one (R44) there is nothing to compare.
    if isinstance(row, dict) and row.get("cases") != node["errs"]:
        return None
    return [(node["ok"], ty.args[0])]


def _compare(node: dict, ty: Ty, types: dict) -> Pairs | None:
    """Compare one node with one type: the pairs to compare next, or ``None``."""
    kind = node["d"]
    if kind in _LEAF_RECIPES:
        return [] if ty == _LEAF_RECIPES[kind] else None
    if kind == "bytes_n":
        return [] if ty == Ty("Bytes", (node["n"],)) else None
    if kind in ("fx_full", "fx_edge"):
        return [] if ty == Ty(node["t"]) else None
    if kind == "option":
        return [(node["of"], ty.args[0])] if ty.head == "Option" else None
    if kind == "result":
        return _compare_result(node, ty, types)
    if kind == "tuple":
        return _pair_up(node["of"], _components_of(ty, types))
    return _compare_variant(node, ty, types)


def is_descriptor_for(node: dict, ty: Ty, types: dict) -> bool:
    """Is the already-validated descriptor ``node`` a recipe for the type ``ty``?"""
    pending: Pairs = [(node, ty)]
    while pending:
        here, wanted = pending.pop()
        more = _compare(here, wanted, types)
        if more is None:
            return False
        pending.extend(more)
    return True


def descriptor_for(
    node: object,
    type_text: object,
    types: dict | None = None,
    sem: Semantics = STANDARD,
) -> tuple[Desc, Ty]:
    """The built descriptor of a binder of type ``type_text``, or ``descriptor``.

    ``types`` is the optional section 2.2 table (absent means ``{}``, R44): with
    none, a ``Result``'s error name is free text and no record or variant can be
    named.  A type text that is not a type, or a tree that is not one for it, is
    ``descriptor`` too: the descriptor is the thing being validated.
    """
    try:
        if types:
            check_types_table(types, "descriptor")  # section 2.2, every consumer
        ty = parse_type(type_text, types or None)
    except Refusal as error:
        raise _refuse(f"not a binder type: {error.message}") from error
    desc = build(node, sem)
    assert type(node) is dict
    try:
        matches = is_descriptor_for(node, ty, types or {})
    except Refusal as error:  # a field type of the types table that is not a type
        raise _refuse(f"the types table does not parse: {error.message}") from error
    if not matches:
        raise _refuse("the descriptor does not describe the binder's type")
    return desc, ty
