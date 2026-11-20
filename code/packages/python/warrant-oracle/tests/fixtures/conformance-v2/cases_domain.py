"""``v2-domain``: ``size()`` and ``nth()`` of a resolved descriptor tree (slice S5a table).

A case is a section 5.1 descriptor, the binder ``type`` (needed to encode the values) and one
``index`` or one ``range`` ``[lo, hi)`` (read half-open).  The expectation is ``size`` and each value's
bare section 2.4 bytes, in index order.  The intents below are written by hand from section 5.1:

* ``u64_edge`` is ``0..255``, then ``1<<i`` for ``i = 8..63``, then ``MAX``, then ``MAX ^ (1<<i)`` for
  ``i = 0..63`` (377 values), so its last two values are ``MAX ^ (1<<62)`` and ``MAX ^ (1<<63) = 2^63-1``;
* an unsigned ``fx_edge`` of width ``w`` ends the same way and a **signed** one is the same bit patterns
  read in two's complement plus ``-255..-1``, in ascending order (D21): it starts at ``-2^(w-1)`` then
  ``-2^(w-2)-1`` and ends at ``2^(w-2)`` then ``2^(w-1)-1``;
* ``bytes_d1`` is the empty string, the 256 one-byte strings, the 16 + 64 strings of length 2 and 3 over
  ``{00, 01, 80, ff}`` and the ten repeated fills of length 4 to 8 (``00`` and ``a5``): 347 values;
* ``bytes_n`` for ``N >= 4`` is the four fills ``00 01 80 ff`` and then one-position edits
  (``4 + 3N`` values); for ``N <= 3`` it is the members of ``bytes_d1`` of length ``N``.

Only ``ir.core`` descriptors appear (tuple and ``nat_shift`` are S10d's, variant and option S11's).
"""

from __future__ import annotations

from kit_common import row, u32le, word

ROWS: list = []
ERR = ["BAD", "WORSE"]


def U(v):  # an unsigned stream value
    return "10" + word(v)


def Sg(v):  # a signed stream value
    return "11" + word(v)


def B(hexstring):  # a Bytes stream value
    return "20" + u32le(len(hexstring) // 2) + hexstring


def dom(slug, summary, descriptor, ty, window, intent):
    fields = {"descriptor": descriptor, "type": ty}
    fields.update(window)
    ROWS.append(row(slug, summary, fields, intent))


def leaf(slug, descriptor, ty, size, first, last):
    """Every leaf at index 0, 1, size-2, size-1 and size (the ``index`` error)."""
    name = descriptor["d"] if slug is None else slug
    base = f"{name}"
    dom(f"{base}-index-0", f"{base}: the first value", descriptor, ty, {"index": 0}, {"size": size, "values_hex": [first[0]]})
    dom(f"{base}-range-first-two", f"{base}: indices 0 and 1", descriptor, ty, {"range": [0, 2]},
        {"size": size, "values_hex": list(first)})
    dom(f"{base}-range-last-two", f"{base}: indices size-2 and size-1", descriptor, ty, {"range": [size - 2, size]},
        {"size": size, "values_hex": list(last)})
    dom(f"{base}-index-last", f"{base}: the last value", descriptor, ty, {"index": size - 1},
        {"size": size, "values_hex": [last[-1]]})
    dom(f"{base}-index-size", f"{base}: index == size is out of range", descriptor, ty, {"index": size},
        {"error": "index"})


# -- leaves -------------------------------------------------------------------------------------------
leaf("bool", {"d": "bool", "size": 2}, "Bool", 2, ["00", "01"], ["00", "01"])
leaf("u8-all", {"d": "u8_all", "size": 256}, "U8", 256, [U(0), U(1)], [U(254), U(255)])
MAX = 2**64 - 1
leaf("u64-edge", {"d": "u64_edge", "size": 377}, "U64", 377, [U(0), U(1)], [U(MAX ^ (1 << 62)), U(MAX ^ (1 << 63))])
leaf("nat-0-64", {"d": "nat_0_64", "size": 65}, "Nat", 65, [U(0), U(1)], [U(63), U(64)])
leaf("bytes-d1", {"d": "bytes_d1", "size": 347}, "Bytes", 347, [B(""), B("00")], [B("00" * 8), B("a5" * 8)])

# bytes_n: N = 0 (one value), 1, 2, 4 and 16
dom("bytes-n-0-index-0", "bytes_n 0 has exactly one value", {"d": "bytes_n", "n": 0, "size": 1}, "Bytes<0>",
    {"index": 0}, {"size": 1, "values_hex": [B("")]})
dom("bytes-n-0-index-size", "bytes_n 0: index 1 is out of range", {"d": "bytes_n", "n": 0, "size": 1}, "Bytes<0>",
    {"index": 1}, {"error": "index"})
leaf("bytes-n-1", {"d": "bytes_n", "n": 1, "size": 256}, "Bytes<1>", 256, [B("00"), B("01")], [B("fe"), B("ff")])
leaf("bytes-n-2", {"d": "bytes_n", "n": 2, "size": 16}, "Bytes<2>", 16, [B("0000"), B("0001")], [B("ff80"), B("ffff")])
leaf("bytes-n-4", {"d": "bytes_n", "n": 4, "size": 16}, "Bytes<4>", 16, [B("00" * 4), B("01" * 4)],
     [B("00" * 3 + "80"), B("00" * 3 + "ff")])
leaf("bytes-n-16", {"d": "bytes_n", "n": 16, "size": 52}, "Bytes<16>", 52, [B("00" * 16), B("01" * 16)],
     [B("00" * 15 + "80"), B("00" * 15 + "ff")])

# nat_count: K = 0 (one value), 1, 2 and 4096 (K = 64 is the leaf nat_0_64)
dom("nat-count-0-index-0", "nat_count 0 has exactly one value", {"d": "nat_count", "k": 0, "size": 1}, "Nat",
    {"index": 0}, {"size": 1, "values_hex": [U(0)]})
dom("nat-count-0-index-size", "nat_count 0: index 1 is out of range", {"d": "nat_count", "k": 0, "size": 1}, "Nat",
    {"index": 1}, {"error": "index"})
leaf("nat-count-1", {"d": "nat_count", "k": 1, "size": 2}, "Nat", 2, [U(0), U(1)], [U(0), U(1)])
leaf("nat-count-2", {"d": "nat_count", "k": 2, "size": 3}, "Nat", 3, [U(0), U(1)], [U(1), U(2)])
leaf("nat-count-4096", {"d": "nat_count", "k": 4096, "size": 4097}, "Nat", 4097, [U(0), U(1)], [U(4095), U(4096)])

# fx_full: 8- and 16-bit types other than U8
leaf("fx-full-i8", {"d": "fx_full", "t": "I8", "size": 256}, "I8", 256, [Sg(-128), Sg(-127)], [Sg(126), Sg(127)])
leaf("fx-full-u16", {"d": "fx_full", "t": "U16", "size": 65536}, "U16", 65536, [U(0), U(1)], [U(65534), U(65535)])
leaf("fx-full-i16", {"d": "fx_full", "t": "I16", "size": 65536}, "I16", 65536, [Sg(-32768), Sg(-32767)], [Sg(32766), Sg(32767)])

# fx_edge: 16-, 32- and 64-bit types other than U64
leaf("fx-edge-u16", {"d": "fx_edge", "t": "U16", "size": 281}, "U16", 281, [U(0), U(1)],
     [U(0xFFFF ^ (1 << 14)), U(0xFFFF ^ (1 << 15))])
leaf("fx-edge-u32", {"d": "fx_edge", "t": "U32", "size": 313}, "U32", 313, [U(0), U(1)],
     [U(0xFFFFFFFF ^ (1 << 30)), U(0xFFFFFFFF ^ (1 << 31))])
for t, size in (("I16", 527), ("I32", 559), ("I64", 623)):
    w = int(t[1:])
    leaf(f"fx-edge-{t.lower()}", {"d": "fx_edge", "t": t, "size": size}, t, size,
         [Sg(-(1 << (w - 1))), Sg(-(1 << (w - 2)) - 1)], [Sg(1 << (w - 2)), Sg((1 << (w - 1)) - 1)])

# -- the four fx_edge types in full, as a range (section 11: a mandatory vector) ---------------------------
for slug, d, ty, size in (
    ("fx-edge-u32-in-full", {"d": "fx_edge", "t": "U32", "size": 313}, "U32", 313),
    ("fx-edge-i32-in-full", {"d": "fx_edge", "t": "I32", "size": 559}, "I32", 559),
    ("fx-edge-i64-in-full", {"d": "fx_edge", "t": "I64", "size": 623}, "I64", 623),
    ("fx-edge-u64-as-u64-edge-in-full", {"d": "u64_edge", "size": 377}, "U64", 377),
):
    dom(slug, f"every value of the descriptor, indices 0..{size - 1}", d, ty, {"range": [0, size]}, {"size": size})

# -- the result composite: Ok(x) for x in the ok descriptor, then each error case in order ---------------------
RES_BOOL = {"d": "result", "ok": {"d": "bool", "size": 2}, "errs": ERR, "size": 4}
dom("result-bool-in-full", "Ok(false), Ok(true), then the error cases in declaration order", RES_BOOL, "Result<Bool,E>",
    {"range": [0, 4]}, {"size": 4, "values_hex": ["5000", "5001", "51" + u32le(3) + b"BAD".hex(), "51" + u32le(5) + b"WORSE".hex()]})
dom("result-bool-index-size", "result: index == size is out of range", RES_BOOL, "Result<Bool,E>", {"index": 4},
    {"error": "index"})
RES_U8 = {"d": "result", "ok": {"d": "u8_all", "size": 256}, "errs": ["X"], "size": 257}
dom("result-u8-last-two", "result over u8_all: Ok(255) then the one error case", RES_U8, "Result<U8,E>",
    {"range": [255, 257]}, {"size": 257, "values_hex": ["50" + U(255), "51" + u32le(1) + b"X".hex()]})
dom("result-u8-first", "result over u8_all: Ok(0) first", RES_U8, "Result<U8,E>", {"index": 0},
    {"size": 257, "values_hex": ["50" + U(0)]})
dom("result-signed-ok", "result over a signed leaf", {"d": "result", "ok": {"d": "fx_full", "t": "I8", "size": 256},
    "errs": ["X"], "size": 257}, "Result<I8,E>", {"range": [0, 1]}, {"size": 257, "values_hex": ["50" + Sg(-128)]})


def nested(n):
    d = {"d": "bool", "size": 2}
    ty = "Bool"
    for _ in range(n):
        d = {"d": "result", "ok": d, "errs": ["X"], "size": d["size"] + 1}
        ty = f"Result<{ty},E>"
    return d, ty


d14, t14 = nested(14)
dom("result-nested-14-accepted", "a tree of 15 levels is within the limit of 16", d14, t14, {"index": 15},
    {"size": 16, "values_hex": ["51" + u32le(1) + b"X".hex()]})
d17, t17 = nested(17)
dom("result-nested-17-refused", "a tree of 18 levels is past the limit of 16", d17, t17, {"index": 0},
    {"error": "descriptor"})

# -- ranges and the index error -------------------------------------------------------------------------------
BOOL = {"d": "bool", "size": 2}
dom("range-empty-at-zero", "an empty range [0, 0) is valid", BOOL, "Bool", {"range": [0, 0]}, {"size": 2, "values_hex": []})
dom("range-empty-at-size", "an empty range [size, size) is valid", BOOL, "Bool", {"range": [2, 2]}, {"size": 2, "values_hex": []})
dom("range-whole", "the whole domain as a range", BOOL, "Bool", {"range": [0, 2]}, {"size": 2, "values_hex": ["00", "01"]})
dom("range-lo-above-hi", "a range with lo > hi", BOOL, "Bool", {"range": [2, 1]}, {"error": "index"})
dom("range-hi-above-size", "a range whose hi is past the size", BOOL, "Bool", {"range": [0, 3]}, {"error": "index"})
dom("range-starts-at-size", "a range that starts at size and is not empty", BOOL, "Bool", {"range": [2, 3]},
    {"error": "index"})
dom("index-size-plus-one", "an index past size", BOOL, "Bool", {"index": 3}, {"error": "index"})

# -- rejected descriptors: every refusal of section 5.1 -------------------------------------------------------------
dom("descriptor-unknown-kind", "an unknown d", {"d": "frobnicate", "size": 2}, "Bool", {"index": 0}, {"error": "descriptor"})
dom("descriptor-missing-size", "a missing size", {"d": "bool"}, "Bool", {"index": 0}, {"error": "descriptor"})
dom("descriptor-size-is-text", "size is a JSON integer, not text", {"d": "bool", "size": "2"}, "Bool", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-size-is-bool", "size is not a JSON boolean", {"d": "bool", "size": True}, "Bool", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-wrong-size", "a size that is not the recomputed one", {"d": "bool", "size": 3}, "Bool", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-extra-key", "an extra key", {"d": "bool", "size": 2, "k": 1}, "Bool", {"index": 0}, {"error": "descriptor"})
dom("descriptor-missing-key", "a missing key: bytes_n needs n", {"d": "bytes_n", "size": 16}, "Bytes<2>", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-not-an-object", "a descriptor that is not an object", ["bool", 2], "Bool", {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-edge-u8", "fx_edge for an 8-bit type is illegal (D25)", {"d": "fx_edge", "t": "U8", "size": 256}, "U8",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-edge-i8", "fx_edge for I8 is illegal (D25)", {"d": "fx_edge", "t": "I8", "size": 256}, "I8",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-edge-u64", "a U64 binder is u64_edge, never fx_edge", {"d": "fx_edge", "t": "U64", "size": 377}, "U64",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-edge-i64-withdrawn-377", "the withdrawn 377-value signed set of the first draft (D21)",
    {"d": "fx_edge", "t": "I64", "size": 377}, "I64", {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-edge-no-type", "fx_edge without t", {"d": "fx_edge", "size": 313}, "U32", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-fx-full-u8", "a U8 binder is u8_all in every document version", {"d": "fx_full", "t": "U8", "size": 256},
    "U8", {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-full-32-bit", "fx_full is for 8- and 16-bit types", {"d": "fx_full", "t": "U32", "size": 4294967296},
    "U32", {"index": 0}, {"error": "descriptor"})
dom("descriptor-fx-full-not-fixed", "fx_full names a fixed-width type", {"d": "fx_full", "t": "Nat", "size": 256}, "Nat",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-bytes-n-too-wide", "bytes_n past 4,096", {"d": "bytes_n", "n": 4097, "size": 12295}, "Bytes", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-bytes-n-negative", "a negative bytes_n width", {"d": "bytes_n", "n": -1, "size": 1}, "Bytes", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-nat-count-too-large", "nat_count K past 4,096", {"d": "nat_count", "k": 4097, "size": 4098}, "Nat",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-result-errs-not-names", "a result's errs are names", {"d": "result", "ok": {"d": "bool", "size": 2},
    "errs": [1], "size": 3}, "Result<Bool,E>", {"index": 0}, {"error": "descriptor"})
dom("descriptor-result-wrong-size", "a result size that is not ok + errs", {"d": "result", "ok": {"d": "bool", "size": 2},
    "errs": ERR, "size": 5}, "Result<Bool,E>", {"index": 0}, {"error": "descriptor"})
dom("descriptor-result-inner-wrong-size", "a wrong size inside a result", {"d": "result", "ok": {"d": "bool", "size": 3},
    "errs": ERR, "size": 5}, "Result<Bool,E>", {"index": 0}, {"error": "descriptor"})
# not a descriptor for the type
dom("descriptor-for-another-type-bool-for-u8", "a bool descriptor for a U8 binder", {"d": "bool", "size": 2}, "U8",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-u8-for-i8", "a u8_all descriptor for an I8 binder", {"d": "u8_all", "size": 256}, "I8",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-nat-for-u8", "a Nat descriptor for a U8 binder", {"d": "nat_0_64", "size": 65}, "U8",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-bytes-width", "bytes_n 2 for a Bytes<3> binder", {"d": "bytes_n", "n": 2, "size": 16},
    "Bytes<3>", {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-bytes-unsized", "bytes_d1 for a Bytes<1> binder", {"d": "bytes_d1", "size": 347},
    "Bytes<1>", {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-fx-edge-width", "fx_edge U16 for an I16 binder", {"d": "fx_edge", "t": "U16", "size": 281},
    "I16", {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-result-payload", "a result over a bool for a Result<U8,E> binder", RES_BOOL, "Result<U8,E>",
    {"index": 0}, {"error": "descriptor"})
dom("descriptor-for-another-type-result-for-plain", "a result descriptor for a Bool binder", RES_BOOL, "Bool", {"index": 0},
    {"error": "descriptor"})
dom("descriptor-for-an-unknown-type", "a binder type that is not a type at all", BOOL, "Frob", {"index": 0},
    {"error": "descriptor"})
