"""Slice S10c-1: the evaluation of the 35 ``ir.fixed`` built-ins (VER04r Appendix A.1, D31).

Every built-in is held against an **independent reference** that works on lists of bits instead of
masked integers (so the two share no arithmetic), at every width and signedness, over the edge
values of VER04j 2.8 and the count rows ``{0, 1, w-1, w, w+1, 2w, 255, 65535, 65536}`` of 3.8.
"""

from __future__ import annotations

import itertools

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.ir_eval import Program, Stats

TYPES = {
    "U8": (8, False),
    "U16": (16, False),
    "U32": (32, False),
    "U64": (64, False),
    "I8": (8, True),
    "I16": (16, True),
    "I32": (32, True),
    "I64": (64, True),
}
NAT = A.Ty("Nat")
INT = A.Ty("Int")
V = lambda n: ["var", n]  # noqa: E731


def lit(ty: str, value: int) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


def evaluate(ir, tenv=None, env=None):
    prog = Program({}, {})
    return prog.compile(ir, tenv or {}, allow_subject=False).fn(env or {})


# -- the reference: values are lists of bits, least significant first -----------------------------


def bits_of(value: int, width: int) -> list[int]:
    """The two's-complement pattern of ``value`` as ``width`` bits, LSB first."""
    return [(value % (1 << width)) >> i & 1 for i in range(width)]


def value_of(bits: list[int], signed: bool) -> int:
    n = sum(b << i for i, b in enumerate(bits))
    return n - (1 << len(bits)) if signed and bits[-1] else n


def edge_values(name: str) -> list[int]:
    """Boundary values of a type: 0, 1, 2, small, the extremes and their neighbours, mid bits."""
    width, signed = TYPES[name]
    lo, hi = (-(1 << (width - 1)), (1 << (width - 1)) - 1) if signed else (0, (1 << width) - 1)
    values = {lo, lo + 1, hi, hi - 1, 0, 1, 2, 3, 127, 128, 255, 1 << (width // 2)}
    values |= {1 << i for i in range(width - (1 if signed else 0))}
    values |= {-1, -2, -3, -127, -128}
    return sorted(v for v in values if lo <= v <= hi)


def counts(width: int) -> list[int]:
    return sorted({0, 1, 2, width - 1, width, width + 1, 2 * width, 255, 256, 65535, 65536})


def ref_shl(x, n, w, s):
    b = bits_of(x, w)
    return value_of([b[i - n] if i >= n else 0 for i in range(w)], s)


def ref_shr(x, n, w, s):
    b = bits_of(x, w)
    return value_of([b[i + n] if i + n < w else 0 for i in range(w)], s)


def ref_sar(x, n, w, s):
    b = bits_of(x, w)
    return value_of([b[i + n] if i + n < w else b[-1] for i in range(w)], s)


def ref_rotl(x, n, w, s):
    b = bits_of(x, w)
    return value_of([b[(i - n) % w] for i in range(w)], s)


def ref_rotr(x, n, w, s):
    b = bits_of(x, w)
    return value_of([b[(i + n) % w] for i in range(w)], s)


# -- bit operations --------------------------------------------------------------------------------


@pytest.mark.parametrize("name", sorted(TYPES))
def test_bitwise_operations_match_the_bit_list_reference(name):
    w, s = TYPES[name]
    vals = edge_values(name)
    tenv = {"a": A.Ty(name), "b": A.Ty(name)}
    prog = Program({}, {})
    fns = {
        tag: prog.compile([tag, V("a"), V("b")], tenv, allow_subject=False).fn
        for tag in ("bit_and", "bit_or", "bit_xor")
    }
    inv = prog.compile(["bit_not", V("a")], tenv, allow_subject=False).fn
    for x, y in itertools.product(vals, repeat=2):
        bx, by = bits_of(x, w), bits_of(y, w)
        env = {"a": x, "b": y}
        assert fns["bit_and"](env) == value_of([p & q for p, q in zip(bx, by)], s)
        assert fns["bit_or"](env) == value_of([p | q for p, q in zip(bx, by)], s)
        assert fns["bit_xor"](env) == value_of([p ^ q for p, q in zip(bx, by)], s)
    for x in vals:
        assert inv({"a": x}) == value_of([1 - p for p in bits_of(x, w)], s)


def test_bit_not_pins_from_the_spec():
    assert evaluate(["bit_not", lit("I8", 0)]) == -1
    assert evaluate(["bit_not", lit("U8", 0)]) == 255
    assert evaluate(["bit_not", lit("I8", -128)]) == 127
    assert evaluate(["bit_not", lit("U64", 0)]) == (1 << 64) - 1


@pytest.mark.parametrize("name", sorted(TYPES))
@pytest.mark.parametrize("tag", ["shl", "shr", "rotl", "rotr", "sar"])
def test_shifts_and_rotations_match_the_reference(name, tag):
    w, s = TYPES[name]
    if tag == "sar" and not s:
        pytest.skip("sar is for signed operands")
    ref = {"shl": ref_shl, "shr": ref_shr, "rotl": ref_rotl, "rotr": ref_rotr, "sar": ref_sar}[tag]
    tenv = {"a": A.Ty(name), "n": NAT}
    fn = Program({}, {}).compile([tag, V("a"), V("n")], tenv, allow_subject=False).fn
    for x in edge_values(name):
        for n in counts(w):
            want = ref(x, n % w if tag in ("rotl", "rotr") else n, w, s)
            assert fn({"a": x, "n": n}) == want, (tag, name, x, n)


@pytest.mark.parametrize(
    "ir,expected",
    [
        (["shl", lit("U8", 1), lit("Nat", 7)], 128),
        (["shl", lit("U8", 1), lit("Nat", 8)], 0),  # n >= w gives 0, not the host's mask
        (["shl", lit("U8", 0x81), lit("Nat", 1)], 0x02),
        (["shr", lit("I8", -1), lit("Nat", 1)], 127),  # logical, then re-read
        (["shr", lit("I8", -1), lit("Nat", 8)], 0),
        (["sar", lit("I8", -128), lit("Nat", 1)], -64),
        (["sar", lit("I8", -128), lit("Nat", 8)], -1),
        (["sar", lit("I8", 100), lit("Nat", 9)], 0),
        (["sar", lit("I64", -1), lit("Nat", 65536)], -1),
        (["rotl", lit("U8", 0x81), lit("Nat", 1)], 0x03),
        (["rotl", lit("U8", 0x81), lit("Nat", 9)], 0x03),  # n mod w first
        (["rotl", lit("U8", 0x81), lit("Nat", 8)], 0x81),
        (["rotr", lit("U8", 0x03), lit("Nat", 1)], 0x81),
        (["rotr", lit("I8", -128), lit("Nat", 1)], 64),
    ],
)
def test_the_spec_rows_of_shifts_and_rotations(ir, expected):
    assert evaluate(ir) == expected


def test_counts_are_exact_nats_far_beyond_the_host_word():
    huge = (1 << 100) + 1
    tenv = {"x": A.Ty("U32"), "n": NAT}
    assert evaluate(["shl", V("x"), V("n")], tenv, {"x": 1, "n": 1 << 100}) == 0
    assert evaluate(["shr", V("x"), V("n")], tenv, {"x": 1, "n": huge}) == 0
    assert evaluate(["rotl", V("x"), V("n")], tenv, {"x": 1, "n": huge}) == 2 ** (huge % 32)
    assert evaluate(["rotr", V("x"), V("n")], tenv, {"x": 1, "n": huge}) == 2 ** ((-huge) % 32)
    assert evaluate(["bit", V("x"), V("n")], tenv, {"x": 1, "n": huge}) is False
    tenv = {"x": A.Ty("I32"), "n": NAT}
    assert evaluate(["sar", V("x"), V("n")], tenv, {"x": -5, "n": huge}) == -1
    assert evaluate(["sar", V("x"), V("n")], tenv, {"x": 5, "n": huge}) == 0


@pytest.mark.parametrize("name", sorted(TYPES))
def test_population_and_zero_counts_read_the_pattern(name):
    w, s = TYPES[name]
    tenv = {"a": A.Ty(name)}
    prog = Program({}, {})
    pop, clz, ctz = (prog.compile([t, V("a")], tenv, allow_subject=False).fn for t in ("popcount", "clz", "ctz"))
    for x in edge_values(name):
        b = bits_of(x, w)
        env = {"a": x}
        assert pop(env) == sum(b)
        assert clz(env) == next((i for i, bit in enumerate(reversed(b)) if bit), w)
        assert ctz(env) == next((i for i, bit in enumerate(b) if bit), w)
    assert pop({"a": 0}) == 0 and clz({"a": 0}) == w and ctz({"a": 0}) == w


def test_popcount_of_a_negative_is_the_width_not_the_host_word():
    assert evaluate(["popcount", lit("I8", -1)]) == 8
    assert evaluate(["clz", lit("I8", -1)]) == 0
    assert evaluate(["ctz", lit("I8", -128)]) == 7
    assert evaluate(["clz", lit("U32", 1)]) == 31


@pytest.mark.parametrize("name", sorted(TYPES))
def test_bit_reads_lsb_zero_and_is_false_past_the_width(name):
    w, s = TYPES[name]
    fn = Program({}, {}).compile(["bit", V("a"), V("i")], {"a": A.Ty(name), "i": NAT}, False).fn
    for x in edge_values(name):
        b = bits_of(x, w)
        for i in counts(w):
            assert fn({"a": x, "i": i}) is (i < w and b[i] == 1)


# -- conversions -----------------------------------------------------------------------------------


def span(name):
    w, s = TYPES[name]
    return (-(1 << (w - 1)), (1 << (w - 1)) - 1) if s else (0, (1 << w) - 1)


@pytest.mark.parametrize("src", sorted(TYPES))
@pytest.mark.parametrize("dst", sorted(TYPES))
def test_wrap_try_and_widen(src, dst):
    wd, sd = TYPES[dst]
    prog = Program({}, {})
    wrap = prog.compile(["wrap", dst, V("a")], {"a": A.Ty(src)}, False).fn
    try_ = prog.compile(["unwrap_or", ["try", dst, V("a")], lit(dst, 0)], {"a": A.Ty(src)}, False).fn
    lo, hi = span(dst)
    for x in edge_values(src):
        assert wrap({"a": x}) == value_of(bits_of(x, wd), sd)
        assert try_({"a": x}) == (x if lo <= x <= hi else 0)
    if span(dst)[0] <= span(src)[0] and span(src)[1] <= span(dst)[1]:
        widen = prog.compile(["widen", dst, V("a")], {"a": A.Ty(src)}, False).fn
        for x in edge_values(src):
            assert widen({"a": x}) == x


@pytest.mark.parametrize("src", ["Nat", "Int"])
def test_wrap_and_try_of_exact_integers(src):
    prog = Program({}, {})
    for dst in sorted(TYPES):
        wd, sd = TYPES[dst]
        lo, hi = span(dst)
        wrap = prog.compile(["wrap", dst, V("a")], {"a": A.Ty(src)}, False).fn
        try_ = prog.compile(["unwrap_or", ["try", dst, V("a")], lit(dst, 0)], {"a": A.Ty(src)}, False).fn
        probes = [0, 1, 255, 256, 65536, 1 << 40, 1 << 64, (1 << 64) + 5]
        if src == "Int":
            probes += [-1, -128, -129, -(1 << 63), -(1 << 63) - 1, -(1 << 100)]
        for x in probes:
            assert wrap({"a": x}) == value_of(bits_of(x, wd), sd)
            assert try_({"a": x}) == (x if lo <= x <= hi else 0)


def test_try_is_none_outside_the_range_and_some_inside():
    prog = Program({}, {})
    # An Option is an IR-only intermediate: look at it through the node's closure directly.
    fn = prog._fixed(["try", "U8", V("a")], {"a": INT}, False, 0).fn
    assert fn({"a": 255}) == ("some", 255)
    assert fn({"a": 256}) == ("none",)
    assert fn({"a": -1}) == ("none",)


def test_to_int_to_nat_and_the_bounds():
    for name, (_w, s) in TYPES.items():
        lo, hi = span(name)
        assert evaluate(["max_value", name]) == hi
        assert evaluate(["min_value", name]) == lo
        assert evaluate(["to_int", ["max_value", name]]) == hi
        if not s:
            assert evaluate(["to_nat", ["max_value", name]]) == hi
    assert evaluate(["to_int", lit("I8", -5)]) == -5


# -- checked arithmetic, div and rem -------------------------------------------------------------


@pytest.mark.parametrize("name", sorted(TYPES))
def test_checked_arithmetic_is_none_exactly_when_the_result_leaves_the_type(name):
    lo, hi = span(name)
    prog = Program({}, {})
    tenv = {"a": A.Ty(name), "b": A.Ty(name)}
    for tag, exact in (
        ("checked_add", lambda x, y: x + y),
        ("checked_sub", lambda x, y: x - y),
        ("checked_mul", lambda x, y: x * y),
    ):
        fn = prog._fixed([tag, V("a"), V("b")], tenv, False, 0).fn
        for x, y in itertools.product(edge_values(name), repeat=2):
            r = exact(x, y)
            assert fn({"a": x, "b": y}) == (("some", r) if lo <= r <= hi else ("none",)), (tag, x, y)


@pytest.mark.parametrize("name", ["U8", "U16", "U32", "U64"])
def test_unsigned_div_and_rem(name):
    prog = Program({}, {})
    tenv = {"a": A.Ty(name), "b": A.Ty(name)}
    d = prog._fixed(["div", V("a"), V("b")], tenv, False, 0).fn
    r = prog._fixed(["rem", V("a"), V("b")], tenv, False, 0).fn
    for x, y in itertools.product(edge_values(name), repeat=2):
        env = {"a": x, "b": y}
        if y == 0:
            assert d(env) == r(env) == ("none",)
        else:
            assert d(env) == ("some", x // y) and r(env) == ("some", x % y)


def test_int_and_nat_div_rem_are_floor_with_the_divisor_sign():
    prog = Program({}, {})
    tenv = {"a": INT, "b": INT}
    d = prog._fixed(["div", V("a"), V("b")], tenv, False, 0).fn
    r = prog._fixed(["rem", V("a"), V("b")], tenv, False, 0).fn
    rows = [(7, 2, 3, 1), (-7, 2, -4, 1), (7, -2, -4, -1), (-7, -2, 3, -1), (-6, 3, -2, 0), (5, -1, -5, 0)]
    for a, b, q, m in rows:
        assert d({"a": a, "b": b}) == ("some", q) and r({"a": a, "b": b}) == ("some", m)
    assert d({"a": 1, "b": 0}) == r({"a": 1, "b": 0}) == ("none",)
    # the identity a == q*b + r for every pair in a small box
    for a in range(-9, 10):
        for b in range(-9, 10):
            if b:
                q, m = d({"a": a, "b": b})[1], r({"a": a, "b": b})[1]
                assert a == q * b + m and (m == 0 or (m < 0) == (b < 0))


@pytest.mark.parametrize("name", ["I8", "I16", "I32", "I64"])
def test_div_trunc_rem_trunc_truncate_toward_zero_and_refuse_min_over_minus_one(name):
    lo, hi = span(name)
    prog = Program({}, {})
    tenv = {"a": A.Ty(name), "b": A.Ty(name)}
    d = prog._fixed(["div_trunc", V("a"), V("b")], tenv, False, 0).fn
    r = prog._fixed(["rem_trunc", V("a"), V("b")], tenv, False, 0).fn
    for x, y in itertools.product(edge_values(name), repeat=2):
        env = {"a": x, "b": y}
        if y == 0 or (x == lo and y == -1):
            assert d(env) == r(env) == ("none",)
            continue
        q = abs(x) // abs(y) * (1 if (x < 0) == (y < 0) else -1)
        assert d(env) == ("some", q)
        assert r(env) == ("some", x - q * y)
        assert (x - q * y == 0) or ((x - q * y < 0) == (x < 0))  # the remainder has the dividend's sign
    assert d({"a": -7, "b": 2}) == ("some", -3)


def test_imin_imax_are_exact():
    for a, b in [(3, -4), (-4, 3), (1 << 70, 5), (0, 0)]:
        assert evaluate(["imin", lit("Int", a), lit("Int", b)]) == min(a, b)
        assert evaluate(["imax", lit("Int", a), lit("Int", b)]) == max(a, b)


# -- endian conversions and at --------------------------------------------------------------------


@pytest.mark.parametrize("name", sorted(TYPES))
def test_endian_bytes_round_trip_and_match_the_reference(name):
    w, s = TYPES[name]
    prog = Program({}, {})
    be = prog.compile(["be_bytes", V("a")], {"a": A.Ty(name)}, False)
    le = prog.compile(["le_bytes", V("a")], {"a": A.Ty(name)}, False)
    assert be.ty == le.ty == A.Ty("Bytes", (w // 8,))
    fb = prog.compile(["from_be", name, V("b")], {"b": A.Ty("Bytes", (w // 8,))}, False).fn
    fl = prog.compile(["from_le", name, V("b")], {"b": A.Ty("Bytes", (w // 8,))}, False).fn
    for x in edge_values(name):
        pattern = bits_of(x, w)
        octets = bytes(sum(pattern[8 * k + j] << j for j in range(8)) for k in range(w // 8))
        assert le.fn({"a": x}) == octets  # least significant byte first
        assert be.fn({"a": x}) == octets[::-1]
        assert fb({"b": octets[::-1]}) == x and fl({"b": octets}) == x


def test_endian_rows_of_the_spec():
    assert evaluate(["be_bytes", lit("U32", 0x01020304)]) == bytes([1, 2, 3, 4])
    assert evaluate(["le_bytes", lit("I16", -2)]) == bytes([0xFE, 0xFF])
    assert evaluate(["from_be", "I8", ["lit", {"type": "Bytes", "value": "ff"}]]) == -1
    assert evaluate(["from_le", "U16", ["lit", {"type": "Bytes", "value": "0201"}]]) == 0x0102


def test_a_runtime_width_other_than_the_targets_is_an_error_never_a_truncation():
    # statically ``Bytes<2>``, but a hand-built environment holds three bytes
    fn = Program({}, {}).compile(["from_be", "U16", V("b")], {"b": A.Ty("Bytes", (2,))}, False).fn
    for bad in (b"\x01", b"\x01\x02\x03", b""):
        with pytest.raises(OracleError) as err:
            fn({"b": bad})
        assert err.value.code == "ir"


def test_at_is_none_outside_the_bytes_and_for_a_negative_index():
    prog = Program({}, {})
    fn = prog._fixed(["at", V("b"), V("i")], {"b": A.Ty("Bytes"), "i": INT}, False, 0).fn
    data = bytes([10, 20, 30])
    assert [fn({"b": data, "i": i}) for i in (-1, 0, 2, 3, 1 << 70)] == [
        ("none",),
        ("some", 10),
        ("some", 30),
        ("none",),
        ("none",),
    ]
    assert fn({"b": b"", "i": 0}) == ("none",)


def test_unwrap_or_evaluates_both_operands_and_takes_the_default_on_none():
    prog = Program({}, {})
    boom = ["add", lit("Int", (1 << 127) - 1), lit("Int", (1 << 127) - 1)]  # ir-overflow when run
    default = ["wrap", "U8", boom]
    ir = ["unwrap_or", ["try", "U8", lit("Int", 5)], default]
    with pytest.raises(OracleError) as err:  # no short circuit: the default runs although Some
        prog.compile(ir, {}, False).fn({})
    assert err.value.code == "ir-overflow"
    assert evaluate(["unwrap_or", ["try", "U8", lit("Int", 5)], lit("U8", 9)]) == 5
    assert evaluate(["unwrap_or", ["try", "U8", lit("Int", 500)], lit("U8", 9)]) == 9


# -- wraps counts only add sub mul neg [D31] --------------------------------------------------------


def test_explicit_truncations_never_count_a_wrap():
    stats = Stats()
    prog = Program({}, {}, stats=stats)
    stats.counting = True
    truncating = [
        ["wrap", "U8", lit("Int", 300)],
        ["shl", lit("U8", 255), lit("Nat", 4)],
        ["bit_not", lit("U8", 0)],
        ["rotl", lit("U8", 255), lit("Nat", 3)],
        ["shl", lit("I8", 64), lit("Nat", 1)],
        ["unwrap_or", ["checked_add", lit("U8", 255), lit("U8", 1)], lit("U8", 0)],
        ["from_be", "U8", ["lit", {"type": "Bytes", "value": "ff"}]],
    ]
    for ir in truncating:
        prog.compile(ir, {}, False).fn({})
    assert stats.wraps == 0
    for ir in (
        ["add", lit("U8", 255), lit("U8", 1)],
        ["sub", lit("U8", 0), lit("U8", 1)],
        ["mul", lit("U8", 16), lit("U8", 16)],
        ["neg", lit("I8", -128)],
    ):
        prog.compile(ir, {}, False).fn({})
    assert stats.wraps == 4


def test_cost_is_one_plus_the_operands_and_a_static_type_is_not_an_operand():
    prog = Program({}, {})
    assert prog.compile(["max_value", "I64"], {}, False).cost == 1
    assert prog.compile(["shl", lit("U8", 1), lit("Nat", 3)], {}, False).cost == 3
    assert prog.compile(["unwrap_or", ["try", "U8", lit("Int", 1)], lit("U8", 0)], {}, False).cost == 4
