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


def dom(slug, summary, descriptor, ty, window, intent, requires=None):
    fields = {"descriptor": descriptor, "type": ty}
    fields.update(window)
    ROWS.append(row(slug, summary, fields, intent, requires))


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


# The descriptor limit is "at most 16 levels deep" and a leaf alone is one level, so nested(n), n result
# nodes over a leaf, is n+1 levels: nested(15) is exactly 16 levels and accepted; nested(16) (17 levels) and
# nested(17) (18 levels) are refused.  (The type text of nested(16) is itself within the type depth of 16,
# which counts a bare name as 0: it is the descriptor tree that is too deep.)
d15, t15 = nested(15)
dom("result-nested-15-accepted", "a tree of 16 levels is exactly at the limit", d15, t15, {"index": 16},
    {"size": 17, "values_hex": ["51" + u32le(1) + b"X".hex()]})
d16, t16 = nested(16)
dom("result-nested-16-refused", "a tree of 17 levels is one past the limit of 16", d16, t16, {"index": 0},
    {"error": "descriptor"})
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

# -- S5e (VER04r section 16.18 R72): errs is 1 to 64 names of the name grammar, without duplicates -------------------------
# At ir.core a v2-domain case has no types field (R44), so the list is checked by itself.


def _errs(slug, summary, names, intent):
    dom(slug, summary, {"d": "result", "ok": {"d": "bool", "size": 2}, "errs": names, "size": 2 + len(names)},
        "Result<Bool,E>", {"index": 0}, intent)


_errs("descriptor-errs-empty", "an empty errs list (an error type has at least one case)", [], {"error": "descriptor"})
_errs("descriptor-errs-65-names", "65 error cases (an error type has at most 64)", [f"C{i}" for i in range(65)],
      {"error": "descriptor"})
_errs("descriptor-errs-duplicate", "a case name repeated in errs", ["BAD", "BAD"], {"error": "descriptor"})
_errs("descriptor-errs-bad-name", "a case name outside the name grammar (a hyphen)", ["BAD-NAME"], {"error": "descriptor"})
_errs("descriptor-errs-name-starts-with-a-digit", "a case name that starts with a digit", ["1BAD"], {"error": "descriptor"})
_errs("descriptor-errs-empty-name", "the empty string is not a name", [""], {"error": "descriptor"})
_errs("descriptor-errs-64-names-accepted", "64 error cases are allowed: size 2 + 64 = 66", [f"C{i}" for i in range(64)],
      {"size": 66})
_errs("descriptor-errs-underscore-name-accepted", "the name grammar allows a leading underscore and digits after it",
      ["_A1", "b_2"], {"size": 4})


# ===============================================================================================
# S10d (VER04r section 13, R17): the ``tuple`` descriptor (``ir.tuple``) and the ``nat_shift`` leaf
# ===============================================================================================
# A tuple descriptor is the Cartesian product of its components, **last component varying fastest** (section 5.1), and a
# value is the stream bytes of a tuple (``64``, the u32 count, the components).  ``descriptor_levels`` gives a tuple descriptor
# ``ir.tuple``.  ``nat_shift`` is a leaf over ``Nat`` like ``nat_count``, so its ``requires`` is ``["ir.core"]`` (R9 levels a
# descriptor by its kind and by its type, and a ``Nat`` leaf needs no level); what selects it is an ``ir.fixed`` count operand
# in a claim, which a ``v2-domain`` case does not hold.  A lane that implements ``ir.core`` therefore runs the ``nat_shift``
# cases, and every case below with a ``tuple`` skips there.
TU = ["ir.core", "ir.tuple"]
BOOLD = {"d": "bool", "size": 2}
U8D = {"d": "u8_all", "size": 256}


def tup_d(*parts):
    size = 1
    for part in parts:
        size *= part["size"]
    return {"d": "tuple", "of": list(parts), "size": size}


def T(*components):
    """A tuple value's stream bytes from its components' (already hex) stream bytes."""
    return "64" + u32le(len(components)) + "".join(components)


F, TR = "00", "01"


def tdom(slug, summary, descriptor, ty, window, intent):
    dom(slug, summary, descriptor, ty, window, intent, requires=TU)


def cdom(slug, summary, descriptor, ty, window, intent):
    """An S10d case whose computed ``requires`` is ``["ir.core"]`` (it states so: a row that states none is an S5 row)."""
    dom(slug, summary, descriptor, ty, window, intent, requires=["ir.core"])


def tleaf(slug, descriptor, ty, size, first, last):
    """A tuple descriptor at index 0, 1, size-2, size-1 and size (the ``index`` error), as ``leaf`` does for a leaf."""
    tdom(f"{slug}-index-0", f"{slug}: the first value", descriptor, ty, {"index": 0}, {"size": size, "values_hex": [first[0]]})
    tdom(f"{slug}-range-first-two", f"{slug}: indices 0 and 1", descriptor, ty, {"range": [0, 2]}, {"size": size, "values_hex": list(first)})
    tdom(f"{slug}-range-last-two", f"{slug}: indices size-2 and size-1", descriptor, ty, {"range": [size - 2, size]},
         {"size": size, "values_hex": list(last)})
    tdom(f"{slug}-index-last", f"{slug}: the last value", descriptor, ty, {"index": size - 1}, {"size": size, "values_hex": [last[-1]]})
    tdom(f"{slug}-index-size", f"{slug}: index == size is out of range", descriptor, ty, {"index": size}, {"error": "index"})


# -- a pair of leaves: the second component varies fastest -------------------------------------------------------------------
tdom("tuple-bool-bool-in-full", "the four values of two Bool components in order: the last component varies fastest",
     tup_d(BOOLD, BOOLD), "(Bool,Bool)", {"range": [0, 4]},
     {"size": 4, "values_hex": [T(F, F), T(F, TR), T(TR, F), T(TR, TR)]})
tleaf("tuple-u8-bool", tup_d(U8D, BOOLD), "(U8,Bool)", 512, [T(U(0), F), T(U(0), TR)], [T(U(255), F), T(U(255), TR)])
tleaf("tuple-bool-u8", tup_d(BOOLD, U8D), "(Bool,U8)", 512, [T(F, U(0)), T(F, U(1))], [T(TR, U(254)), T(TR, U(255))])
tdom("tuple-bool-u8-across-the-first-component", "indices 255 and 256 cross from (false, 255) to (true, 0)", tup_d(BOOLD, U8D), "(Bool,U8)",
     {"range": [255, 257]}, {"size": 512, "values_hex": [T(F, U(255)), T(TR, U(0))]})
tdom("tuple-three-components-in-full", "(Bool,Bool,Bool): eight values, the last component fastest", tup_d(BOOLD, BOOLD, BOOLD),
     "(Bool,Bool,Bool)", {"range": [0, 8]},
     {"size": 8, "values_hex": [T(*(TR if (i >> k) & 1 else F for k in (2, 1, 0))) for i in range(8)]})
EIGHT = tup_d(*[BOOLD] * 8)
ty8 = "(" + ",".join(["Bool"] * 8) + ")"
tdom("tuple-eight-components-first", "the first of 256 values of an eight-component tuple", EIGHT, ty8, {"index": 0},
     {"size": 256, "values_hex": [T(*[F] * 8)]})
tdom("tuple-eight-components-second", "index 1 sets the last component", EIGHT, ty8, {"index": 1},
     {"size": 256, "values_hex": [T(*[F] * 7, TR)]})
tdom("tuple-eight-components-index-128", "index 128 sets the first component", EIGHT, ty8, {"index": 128},
     {"size": 256, "values_hex": [T(TR, *[F] * 7)]})
tdom("tuple-eight-components-last", "the last value has every component true", EIGHT, ty8, {"index": 255},
     {"size": 256, "values_hex": [T(*[TR] * 8)]})
tdom("tuple-eight-components-index-size", "index 256 is out of range", EIGHT, ty8, {"index": 256}, {"error": "index"})

# -- composite components ---------------------------------------------------------------------------------------------------------------
NEST_FIRST = tup_d(tup_d(BOOLD, BOOLD), BOOLD)
tdom("tuple-nested-first-in-full", "((Bool,Bool),Bool): the inner pair is the slow component", NEST_FIRST, "((Bool,Bool),Bool)",
     {"range": [0, 8]}, {"size": 8, "values_hex": [T(T(*(TR if (i >> k) & 1 else F for k in (2, 1)) ), TR if i & 1 else F) for i in range(8)]})
NEST_LAST = tup_d(BOOLD, tup_d(BOOLD, BOOLD))
tdom("tuple-nested-last-in-full", "(Bool,(Bool,Bool)): the inner pair is the fast component", NEST_LAST, "(Bool,(Bool,Bool))",
     {"range": [0, 8]}, {"size": 8, "values_hex": [T(TR if i >> 2 & 1 else F, T(*(TR if (i >> k) & 1 else F for k in (1, 0)))) for i in range(8)]})
RESD = {"d": "result", "ok": BOOLD, "errs": ERR, "size": 4}
tdom("tuple-with-a-result-component-in-full", "(Result<Bool,E>, Bool): eight values; the Result varies slowly",
     tup_d(RESD, BOOLD), "(Result<Bool,E>,Bool)", {"range": [0, 8]},
     {"size": 8, "values_hex": [T(r, b) for r in ("5000", "5001", "51" + u32le(3) + b"BAD".hex(), "51" + u32le(5) + b"WORSE".hex())
                                for b in (F, TR)]})
tdom("tuple-with-a-bytes-n-component", "(Bytes<1>, Bool): 512 values, the Bytes<1> component slow", tup_d({"d": "bytes_n", "n": 1, "size": 256}, BOOLD),
     "(Bytes<1>,Bool)", {"range": [2, 4]}, {"size": 512, "values_hex": [T(B("01"), F), T(B("01"), TR)]})
tdom("tuple-with-a-bytes-d1-component", "(Bytes, Bool) over bytes_d1: 347 * 2 values; index 2 is (00, false)",
     tup_d({"d": "bytes_d1", "size": 347}, BOOLD), "(Bytes,Bool)", {"range": [0, 4]},
     {"size": 694, "values_hex": [T(B(""), F), T(B(""), TR), T(B("00"), F), T(B("00"), TR)]})
tdom("tuple-with-a-nat-count-component", "(Nat, Bool) over nat_count 2: six values", tup_d({"d": "nat_count", "k": 2, "size": 3}, BOOLD),
     "(Nat,Bool)", {"range": [0, 6]}, {"size": 6, "values_hex": [T(U(n), b) for n in range(3) for b in (F, TR)]})
tdom("tuple-with-a-signed-component", "(I8, Bool) over fx_full: the first values are the minimum", tup_d({"d": "fx_full", "t": "I8", "size": 256}, BOOLD),
     "(I8,Bool)", {"range": [0, 3]}, {"size": 512, "values_hex": [T(Sg(-128), F), T(Sg(-128), TR), T(Sg(-127), F)]})
tdom("tuple-with-a-u64-edge-component", "(U64, U8) over u64_edge: the last value", tup_d({"d": "u64_edge", "size": 377}, U8D), "(U64,U8)",
     {"index": 377 * 256 - 1}, {"size": 377 * 256, "values_hex": [T(U(MAX ^ (1 << 63)), U(255))]})
tdom("tuple-with-an-fx-edge-component", "(I32, Bool) over fx_edge: the first value", tup_d({"d": "fx_edge", "t": "I32", "size": 559}, BOOLD),
     "(I32,Bool)", {"index": 0}, {"size": 559 * 2, "values_hex": [T(Sg(-(1 << 31)), F)]})

# -- nat_shift: 0..=64, then 65 127 128 129 255 256 65535 65536 (73 values, [D26]) ---------------------------------------------------
SHIFT = {"d": "nat_shift", "size": 73}
cdom("nat-shift-index-0", "nat_shift: the first value", SHIFT, "Nat", {"index": 0}, {"size": 73, "values_hex": [U(0)]})
cdom("nat-shift-range-first-two", "nat_shift: indices 0 and 1", SHIFT, "Nat", {"range": [0, 2]}, {"size": 73, "values_hex": [U(0), U(1)]})
cdom("nat-shift-range-last-two", "nat_shift: indices size-2 and size-1", SHIFT, "Nat", {"range": [71, 73]},
    {"size": 73, "values_hex": [U(65535), U(65536)]})
cdom("nat-shift-index-last", "nat_shift: the last value is 65536", SHIFT, "Nat", {"index": 72}, {"size": 73, "values_hex": [U(65536)]})
cdom("nat-shift-index-size", "nat_shift: index == size is out of range", SHIFT, "Nat", {"index": 73}, {"error": "index"})
cdom("nat-shift-at-the-width-boundaries", "the run 0..=64 ends at index 64 and the bounded set follows: 64, 65, 127, 128, 129, 255",
    SHIFT, "Nat", {"range": [64, 70]}, {"size": 73, "values_hex": [U(v) for v in (64, 65, 127, 128, 129, 255)]})
cdom("nat-shift-in-full", "every one of the 73 values", SHIFT, "Nat", {"range": [0, 73]}, {"size": 73})
cdom("nat-nat-count-64-is-not-nat-shift", "nat_0_64 stops at 64 where nat_shift goes on: index 65 is out of range for nat_0_64",
    {"d": "nat_0_64", "size": 65}, "Nat", {"index": 65}, {"error": "index"})
tdom("tuple-with-a-nat-shift-component", "(Nat, Bool) over nat_shift: 146 values, the last component fastest",
     tup_d(SHIFT, BOOLD), "(Nat,Bool)", {"range": [144, 146]}, {"size": 146, "values_hex": [T(U(65536), F), T(U(65536), TR)]})
tdom("tuple-of-two-nat-shifts", "(Nat, Nat) over two nat_shift leaves: 5,329 values; index 73 starts the second row",
     tup_d(SHIFT, SHIFT), "(Nat,Nat)", {"range": [72, 75]},
     {"size": 73 * 73, "values_hex": [T(U(0), U(65536)), T(U(1), U(0)), T(U(1), U(1))]})
cdom("descriptor-nat-shift-wrong-size", "nat_shift always has size 73", {"d": "nat_shift", "size": 74}, "Nat", {"index": 0}, {"error": "descriptor"})
cdom("descriptor-nat-shift-with-a-parameter", "nat_shift is a leaf with no parameter: an extra key is refused",
    {"d": "nat_shift", "size": 73, "k": 64}, "Nat", {"index": 0}, {"error": "descriptor"})
cdom("descriptor-nat-shift-missing-size", "nat_shift without its size", {"d": "nat_shift"}, "Nat", {"index": 0}, {"error": "descriptor"})
cdom("descriptor-for-another-type-nat-shift-for-u8", "a nat_shift descriptor for a U8 binder", SHIFT, "U8", {"index": 0},
    {"error": "descriptor"})
cdom("descriptor-for-another-type-nat-shift-for-bytes", "a nat_shift descriptor for a Bytes binder", SHIFT, "Bytes", {"index": 0},
    {"error": "descriptor"})

# -- tuple: every refusal of section 5.1 and the depth of a tree -----------------------------------------------------------------------------
tdom("descriptor-tuple-wrong-size", "a tuple size that is not the product of its components", {"d": "tuple", "of": [BOOLD, BOOLD], "size": 5},
     "(Bool,Bool)", {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-component-wrong-size", "a wrong size inside a component", {"d": "tuple", "of": [{"d": "bool", "size": 3}, BOOLD], "size": 6},
     "(Bool,Bool)", {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-missing-of", "a tuple without of", {"d": "tuple", "size": 4}, "(Bool,Bool)", {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-extra-key", "an extra key on a tuple", {"d": "tuple", "of": [BOOLD, BOOLD], "size": 4, "k": 2}, "(Bool,Bool)",
     {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-of-is-not-a-list", "of is a list of descriptors", {"d": "tuple", "of": BOOLD, "size": 2}, "(Bool,Bool)", {"index": 0},
     {"error": "descriptor"})
tdom("descriptor-tuple-component-is-not-an-object", "a component that is not a descriptor", {"d": "tuple", "of": ["bool", BOOLD], "size": 4},
     "(Bool,Bool)", {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-three-components-for-a-pair", "of has three entries for a pair type", tup_d(BOOLD, BOOLD, BOOLD), "(Bool,Bool)",
     {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-two-components-for-a-triple", "of has two entries for a triple type", tup_d(BOOLD, BOOLD), "(Bool,Bool,Bool)",
     {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-component-for-another-type", "a bool descriptor for the U8 component", tup_d(BOOLD, BOOLD), "(Bool,U8)", {"index": 0},
     {"error": "descriptor"})
tdom("descriptor-tuple-component-nested-for-another-type", "the inner pair's descriptor does not fit the inner type",
     tup_d(tup_d(BOOLD, BOOLD), BOOLD), "((Bool,U8),Bool)", {"index": 0}, {"error": "descriptor"})
tdom("descriptor-tuple-for-a-scalar-type", "a tuple descriptor for a Bool binder", tup_d(BOOLD, BOOLD), "Bool", {"index": 0},
     {"error": "descriptor"})
tdom("descriptor-tuple-for-a-result-type", "a tuple descriptor for a Result binder", tup_d(BOOLD, BOOLD), "Result<Bool,E>", {"index": 0},
     {"error": "descriptor"})
cdom("descriptor-scalar-for-a-tuple-type", "a bool descriptor for a pair binder: the descriptor needs no level, so a lane that implements "
    "ir.core runs this case and must refuse it", BOOLD, "(Bool,Bool)", {"index": 0}, {"error": "descriptor"})


def tnest(n):
    """``n`` nested pairs over a ``bool`` leaf: ``n + 1`` levels, the type ``(((Bool,Bool),Bool),..,Bool)`` of depth ``n``."""
    d, ty = BOOLD, "Bool"
    for _ in range(n):
        d, ty = tup_d(d, BOOLD), f"({ty},Bool)"
    return d, ty


d15t, t15t = tnest(15)
_last = TR
for _ in range(15):
    _last = T(_last, TR)
tdom("tuple-nested-15-accepted", "a tree of 16 levels is exactly at the limit: the last of 2^16 values has every leaf true", d15t, t15t,
     {"index": 2**16 - 1}, {"size": 2**16, "values_hex": [_last]})
d16t, t16t = tnest(16)
tdom("tuple-nested-16-refused", "a tree of 17 levels is one past the limit of 16, although the type text is within its depth",
     d16t, t16t, {"index": 0}, {"error": "descriptor"})
