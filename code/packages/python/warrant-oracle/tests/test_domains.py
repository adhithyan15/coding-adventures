"""Domain descriptors: sizes, orders, ``nth`` at the boundaries, demotion, budgets."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.domains import (
    DOMAIN_BYTES_BUDGET,
    MAX_SUITE_PRODUCT,
    DomainReport,
    ListDesc,
    OptionDesc,
    ResultDesc,
    TupleDesc,
    VariantDesc,
    check_budget,
    demote,
    descriptor_for,
    leaf_bool,
    leaf_bytes_d1,
    leaf_bytes_n,
    leaf_fx_edge,
    leaf_fx_full,
    leaf_nat_count,
    leaf_nat_shift,
    leaf_u8_all,
    leaf_u64_edge,
)
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source

T = A.Ty
MAX64 = (1 << 64) - 1

SPEC = lower_document(
    parse_source(
        b"warrant d v1\n"
        b"  error E = A | B | C\n"
        b"  record P(x: U8, y: Bool)\n"
        b"  variant V = Q | R(v: U8) | S(p: Bool, q: Bool)\n"
        b"  op f(a: Nat) -> Nat\n"
    )
)


def boundaries(d):
    n = d.size()
    return [d.nth(0), d.nth(1), d.nth(n - 1)] if n > 1 else [d.nth(0)]


def test_bool_and_u8_all():
    assert [leaf_bool().nth(i) for i in range(2)] == [False, True]
    u8 = leaf_u8_all()
    assert (u8.size(), u8.nth(0), u8.nth(1), u8.nth(255)) == (256, 0, 1, 255)
    assert u8.label == "full" and u8.to_json() == {"d": "u8_all", "size": 256}


def test_u64_edge_order_and_size():
    d = leaf_u64_edge()
    assert d.size() == 377
    assert d.nth(255) == 255 and d.nth(256) == 1 << 8  # 0..255 then 1<<8
    assert d.nth(256 + 55) == 1 << 63  # 1<<i for i = 8..63 is 56 values
    assert d.nth(312) == MAX64  # then MAX
    assert d.nth(313) == MAX64 ^ 1  # then MAX ^ (1 << i), i = 0..
    assert d.nth(376) == MAX64 ^ (1 << 63)
    assert d.label == "bounded-set"
    with pytest.raises(IndexError):
        d.nth(377)
    with pytest.raises(IndexError):
        d.nth(-1)


@pytest.mark.parametrize(
    "ty,size",
    [("U32", 313), ("U16", 281), ("U64", 377), ("I32", 559), ("I64", 623), ("I16", 527)],
)
def test_fx_edge_sizes(ty, size):
    assert leaf_fx_edge(T(ty)).size() == size


def test_fx_edge_signed_is_the_wrapped_edge_set_plus_the_small_negatives_sorted():
    """[D21]: ``wrap_S(E_U(w)) | {-255..-1}``, de-duplicated, ascending (the generator's set)."""
    for name, size in (("I16", 527), ("I32", 559), ("I64", 623)):
        s = leaf_fx_edge(T(name))
        bits = int(name[1:])
        unsigned = leaf_fx_edge(T("U" + name[1:])).values
        wrapped = {v - (1 << bits) if v >= 1 << (bits - 1) else v for v in unsigned}
        assert s.values == sorted(wrapped | set(range(-255, 0)))
        assert len(s.values) == len(set(s.values)) == size
        assert s.values[0] == -(1 << (bits - 1)) and s.values[-1] == (1 << (bits - 1)) - 1
    # the head, the negatives and the zero of the 64-bit set, by value
    s = leaf_fx_edge(T("I64")).values
    assert s[:3] == [-(1 << 63), -(1 << 62) - 1, -(1 << 61) - 1]
    assert s.index(-255) < s.index(-1) < s.index(0)
    # unsigned edge sets are unchanged and in pattern order (not sorted)
    u = leaf_fx_edge(T("U32")).values
    assert u[:3] == [0, 1, 2] and u[256] == 1 << 8 and u[280] == (1 << 32) - 1 and u != sorted(u)


def test_fx_edge_at_8_bits_is_illegal_and_16_bits_exists_for_demotion_d25():
    for name in ("U8", "I8"):
        with pytest.raises(OracleError) as err:
            leaf_fx_edge(T(name))
        assert err.value.code == "generation" and "illegal" in err.value.message
    assert leaf_fx_edge(T("U16")).size() == 281 and leaf_fx_edge(T("I16")).size() == 527
    assert leaf_fx_edge(T("I16")).to_json() == {"d": "fx_edge", "t": "I16", "size": 527}


def test_nat_shift_is_the_73_value_leaf_of_d26():
    d = leaf_nat_shift()
    assert d.to_json() == {"d": "nat_shift", "size": 73}
    assert d.values == [*range(65), 65, 127, 128, 129, 255, 256, 65535, 65536]
    assert d.label == "bounded-set" and d.ty == T("Nat")
    # nat_count stays 0..K (D26): it is not the shift domain
    assert leaf_nat_count(64).size() == 65 and leaf_nat_count(100).to_json() == {
        "d": "nat_count",
        "k": 100,
        "size": 101,
    }


def test_fx_full_unsigned_and_signed():
    u16 = leaf_fx_full(T("U16"))
    assert (u16.size(), u16.nth(0), u16.nth(65535)) == (65536, 0, 65535)
    i8 = leaf_fx_full(T("I8"))
    assert (i8.size(), i8.nth(0), i8.nth(1), i8.nth(255)) == (256, -128, -127, 127)
    assert leaf_fx_full(T("I16")).nth(0) == -32768
    with pytest.raises(OracleError):
        leaf_fx_full(T("U32"))


def test_nat_count():
    d = leaf_nat_count(64)
    assert d.size() == 65 and d.to_json()["d"] == "nat_0_64" and d.nth(64) == 64
    d = leaf_nat_count(5)
    assert d.to_json() == {"d": "nat_count", "k": 5, "size": 6}
    with pytest.raises(OracleError):
        leaf_nat_count(5000)


def test_bytes_d1_is_the_347_string_set():
    d = leaf_bytes_d1()
    assert d.size() == 347
    assert d.nth(0) == b"" and d.nth(1) == b"\x00" and d.nth(256) == b"\xff"
    assert d.nth(257) == b"\x00\x00" and d.nth(258) == b"\x00\x01"  # alphabet 00 01 80 ff
    assert d.nth(346) == b"\xa5" * 8
    lengths = [len(v) for v in d.values]
    assert lengths.count(0) == 1 and lengths.count(1) == 256
    assert lengths.count(2) == 16 and lengths.count(3) == 64
    assert all(lengths.count(n) == 2 for n in range(4, 9))
    assert d.bytes_cost() == 32 + 8


@pytest.mark.parametrize("n,size", [(0, 1), (1, 256), (2, 16), (3, 64), (4, 16), (16, 52)])
def test_bytes_n_sizes(n, size):
    d = leaf_bytes_n(n)
    assert d.size() == size and all(len(v) == n for v in d.values)


def test_bytes_n_large_widths_and_labels():
    d = leaf_bytes_n(4)
    assert d.nth(0) == b"\x00" * 4 and d.nth(3) == b"\xff" * 4
    assert d.nth(4) == b"\x01\x00\x00\x00"  # position 0 set to the first symbol
    assert d.nth(15) == b"\x00\x00\x00\xff"
    assert leaf_bytes_n(4096).size() == 4 + 3 * 4096
    assert [leaf_bytes_n(n).label for n in (0, 1, 2, 3, 4, 16)] == ["full", "full"] + ["sample"] * 4
    with pytest.raises(OracleError):
        leaf_bytes_n(4097)


def test_tuple_is_last_component_fastest():
    d = TupleDesc(T("()", (T("Bool"), T("U8"))), [leaf_bool(), leaf_nat_count(2)])
    assert d.size() == 6
    assert [d.nth(i) for i in range(6)] == [
        (False, 0),
        (False, 1),
        (False, 2),
        (True, 0),
        (True, 1),
        (True, 2),
    ]
    assert d.depth() == 2 and d.to_json()["d"] == "tuple"
    with pytest.raises(IndexError):
        d.nth(6)


def test_record_variant_option_result_list_from_types():
    rec = descriptor_for(SPEC, T("P"))
    assert rec.size() == 512 and rec.nth(0) == (0, False) and rec.nth(511) == (255, True)
    var = descriptor_for(SPEC, T("V"))
    assert var.size() == 1 + 256 + 4
    assert var.nth(0) == ("Q", ()) and var.nth(1) == ("R", (0,)) and var.nth(256) == ("R", (255,))
    assert var.nth(257) == ("S", (False, False)) and var.nth(260) == ("S", (True, True))
    with pytest.raises(IndexError):
        var.nth(261)
    opt = descriptor_for(SPEC, T("Option", (T("Bool"),)))
    assert [opt.nth(i) for i in range(3)] == [("none",), ("some", False), ("some", True)]
    res = descriptor_for(SPEC, T("Result", (T("Bool"), T("E"))))
    assert [res.nth(i) for i in range(5)] == [
        ("ok", False),
        ("ok", True),
        ("err", "A"),
        ("err", "B"),
        ("err", "C"),
    ]
    assert res.size() == 5
    for d in (rec, var, opt, res):
        with pytest.raises(IndexError):
            d.nth(d.size())


def test_list_domain_boundary_between_length_two_and_three():
    d = descriptor_for(SPEC, T("List", (T("U8"),)))
    assert isinstance(d, ListDesc)
    assert d.size() == 1 + 16 + 256 + 64 == 337
    assert d.nth(0) == ("list", ())
    assert d.nth(1) == ("list", (0,)) and d.nth(16) == ("list", (15,))
    assert d.nth(17) == ("list", (0, 0)) and d.nth(18) == ("list", (0, 1))
    assert d.nth(272) == ("list", (15, 15))  # the last length-2 list
    assert d.nth(273) == ("list", (0, 0, 0))  # the first length-3 list, over the first 4 values
    assert d.nth(274) == ("list", (0, 0, 1))
    assert d.nth(336) == ("list", (3, 3, 3))
    with pytest.raises(IndexError):
        d.nth(337)


def test_list_of_a_small_domain_uses_all_of_it():
    d = ListDesc(T("List", (T("Bool"),)), leaf_bool())
    assert d.size() == 1 + 2 + 4 + 8
    assert d.nth(14) == ("list", (True, True, True))


def test_list_of_lists_is_337_regardless_of_the_element():
    inner = descriptor_for(SPEC, T("List", (T("U8"),)))
    outer = ListDesc(T("List", (T("List", (T("U8"),)),)), inner)
    assert outer.size() == 337


def test_labels_propagate_to_the_weakest_component():
    assert descriptor_for(SPEC, T("()", (T("Bool"), T("U64")))).label == "bounded-set"
    assert descriptor_for(SPEC, T("()", (T("Bool"), T("Bool")))).label == "full"
    assert descriptor_for(SPEC, T("Option", (T("Bytes", (4,)),))).label == "sample"
    assert descriptor_for(SPEC, T("List", (T("Bool"),))).label == "bounded-set"
    assert descriptor_for(SPEC, T("List", (T("Bytes", (4,)),))).label == "sample"
    assert descriptor_for(SPEC, T("V")).label == "full"
    assert descriptor_for(SPEC, T("Result", (T("Bool"), T("E")))).label == "full"
    assert DomainReport([2], ["full", "sample"], []).label == "sample"


def test_descriptor_depth_and_unknown_types():
    deep = T("Nat")
    for _ in range(20):
        deep = T("Option", (deep,))
    with pytest.raises(OracleError) as err:
        descriptor_for(SPEC, deep)
    assert err.value.code == "limit"
    with pytest.raises(OracleError):
        descriptor_for(SPEC, T("Int"))
    with pytest.raises(OracleError):
        descriptor_for(SPEC, T("Bytes", ("N",)))
    assert descriptor_for(SPEC, T("I8")).size() == 256
    assert descriptor_for(SPEC, T("U32")).size() == 313
    assert descriptor_for(SPEC, T("U16")).size() == 65536


# -- budget demotion (VER04j section 2.6) ------------------------------------------------


def test_u16_u16_demotes_both_at_the_reference_budget():
    binders = [descriptor_for(SPEC, T("U16")), descriptor_for(SPEC, T("U16"))]
    demoted = demote(binders, 400_000)  # the list is updated in place
    assert demoted == ["0", "1"] and [b.size() for b in binders] == [281, 281]  # 78,961


def test_u16_u16_demotes_only_the_leftmost_at_two_to_the_26():
    binders = [descriptor_for(SPEC, T("U16")), descriptor_for(SPEC, T("U16"))]
    assert demote(binders, 1 << 26) == ["0"]
    assert [b.size() for b in binders] == [281, 65536]


def test_u8_is_never_demoted_so_u16_u8_u8_fails_at_two_to_the_24():
    binders = [descriptor_for(SPEC, T(t)) for t in ("U16", "U8", "U8")]
    with pytest.raises(OracleError) as err:
        demote(binders, 1 << 24)
    assert "W0706" in err.value.message
    ok = [descriptor_for(SPEC, T(t)) for t in ("U16", "U8")]
    assert demote(ok, 1 << 24) == []


def test_no_demotion_when_within_budget():
    binders = [descriptor_for(SPEC, T("U8")), descriptor_for(SPEC, T("U8"))]
    assert demote(binders, 1 << 24) == []


def test_composite_leaves_demote_depth_first_left_to_right():
    tup = descriptor_for(SPEC, T("()", (T("U16"), T("U16"))))
    # VER04j 14.1 item 20 demotes *both* components at 2^26, because the Rust
    # generator stores a composite domain as one vector under a 64 MiB byte budget.
    # VER04r 14 enumerates chunk-wise and never materialises a domain, so the
    # oracle demotes by *product size only*: one demotion suffices here.  If the
    # generator keeps the VER04j rule, the descriptor comparison catches the gap.
    binders = [tup]
    assert demote(binders, 1 << 26) == ["0.0"]
    assert binders[0].size() == 281 * 65536
    assert demote(binders, 100_000) == ["0.1"] and binders[0].size() == 281 * 281
    rec_like = [descriptor_for(SPEC, T("()", (T("Bool"), T("I16"))))]
    assert demote(rec_like, 100_000) == ["0.1"] and rec_like[0].size() == 2 * 527
    opt = [descriptor_for(SPEC, T("Option", (T("U16"),)))]
    assert demote(opt, 1000) == ["0.some"] and opt[0].size() == 282
    lst = [descriptor_for(SPEC, T("List", (T("U16"),)))]
    assert demote(lst, 10**9) == []  # within budget: untouched
    with pytest.raises(OracleError):  # a list over >= 16 values is 337 whatever the element
        demote(lst, 300)
    res = [descriptor_for(SPEC, T("Result", (T("U16"), T("E"))))]
    assert demote(res, 1000) == ["0.ok"] and res[0].size() == 281 + 3


def test_variant_leaf_demotion_names_the_constructor():
    spec = lower_document(
        parse_source(b"warrant d v1\n  variant W = Q | R(v: U16)\n  op f(a: Nat) -> Nat\n")
    )
    v = [descriptor_for(spec, T("W"))]
    assert demote(v, 1000) == ["0.R.0"] and v[0].size() == 1 + 281


def test_check_budget_errors_before_anything_is_built():
    assert check_budget([leaf_u8_all(), leaf_u8_all()], 1 << 24) == 65536
    with pytest.raises(OracleError) as err:
        check_budget([leaf_u8_all()] * 3, 1 << 23)
    assert "W0706" in err.value.message
    with pytest.raises(OracleError):  # eight U8 binders are 2**64 cases, never zero
        check_budget([leaf_u8_all()] * 8, 1 << 60)
    with pytest.raises(OracleError):
        check_budget([leaf_u8_all()] * 4, 1 << 40)  # 2**32 > MAX_SUITE_PRODUCT
    assert MAX_SUITE_PRODUCT == 1 << 28
    big = leaf_bytes_n(4096)
    with pytest.raises(OracleError) as err:
        check_budget([big, big], 1 << 28)  # product fits, the byte budget does not
    assert "DOMAIN_BYTES_BUDGET" in err.value.message
    assert DOMAIN_BYTES_BUDGET == 64 << 20


def test_composite_class_exports():
    assert OptionDesc and ResultDesc and VariantDesc


# -- CV2-F17 O1: demotion refreshes cached sizes bottom-up -----------------------------------
#
# A variant caches one size per constructor, computed from its fields' sizes.  A 16-bit leaf
# under a tuple or record *inside* a variant used to leave the variant holding the size from
# before the demotion, so the budget loop saw no progress and refused with W0706.

_STALE = lower_document(
    parse_source(
        b"warrant d v1\n"
        b"  record R(a: U16, b: Bool)\n"
        b"  variant M = X | Y(t: (U16, Bool))\n"
        b"  variant MR = X2 | Y2(r: R)\n"
        b"  variant In = P | Q(v: U16)\n"
        b"  variant Out = X3 | Y3(i: In)\n"
        b"  record Wrap(m: M, b: Bool)\n"
        b"  op f(a: Nat) -> Nat\n"
    )
)

# U16 demotes from 65536 values to the 281-value edge set; (U16, Bool) is then 281 * 2.
_EDGE = 281


def _demoted(ty: A.Ty, budget: int = 1000):
    binders = [descriptor_for(_STALE, ty)]
    paths = demote(binders, budget)
    return paths, binders[0]


def test_variant_over_tuple_demotes_with_the_right_path_and_size():
    paths, d = _demoted(T("M"))
    assert paths == ["0.Y.0.0"] and d.size() == 1 + _EDGE * 2
    assert d.label == "bounded-set"


def test_variant_over_record_demotes_with_the_right_path_and_size():
    paths, d = _demoted(T("MR"))
    assert paths == ["0.Y2.0.0"] and d.size() == 1 + _EDGE * 2
    assert d.label == "bounded-set"


def test_nested_variant_demotes_with_the_right_path_and_size():
    paths, d = _demoted(T("Out"))
    assert paths == ["0.Y3.0.Q.0"] and d.size() == 1 + 1 + _EDGE
    assert d.label == "bounded-set"


def test_record_over_variant_demotes_with_the_right_path_and_size():
    paths, d = _demoted(T("Wrap"), 2000)  # (1 + 562) * 2 = 1126 fits
    assert paths == ["0.0.Y.0.0"] and d.size() == (1 + _EDGE * 2) * 2
    assert d.label == "bounded-set"


def test_option_over_variant_demotes_with_the_right_path_and_size():
    paths, d = _demoted(T("Option", (T("M"),)))
    assert paths == ["0.some.Y.0.0"] and d.size() == 1 + 1 + _EDGE * 2
    assert d.label == "bounded-set"


@pytest.mark.parametrize(
    "ty,path",
    [
        (T("()", (T("U16"), T("Bool"))), "0.0"),
        (T("Option", (T("U16"),)), "0.some"),
        (T("Result", (T("U16"), T("E"))), "0.ok"),
        (T("Result", (T("()", (T("U16"), T("Bool"))), T("E"))), "0.ok.0"),
    ],
)
def test_a_demoted_composite_is_labelled_bounded_set(ty, path):
    # VER04r 9.3: a composite that holds a demoted (fx_edge) leaf is no longer ``full``.
    d = descriptor_for(SPEC, ty)
    assert d.label == "full"
    binders = [d]
    assert demote(binders, 1000) == [path] and binders[0].label == "bounded-set"


def test_list_over_variant_refreshes_the_list_caches():
    # The list's d16/d4 come from its element's size: bottom-up refresh keeps them honest.
    binders = [descriptor_for(_STALE, T("List", (T("M"),)))]
    demote(binders, 10**9)  # within budget: nothing to refresh, nothing demoted
    assert binders[0].size() == 1 + 16 + 16**2 + 4**3


# -- CV2-F17 O2: a descriptor path has at most 16 nodes (VER04r 14) -------------------------


def _nested_records(n: int) -> A.Ty:
    """A binder type whose descriptor path has ``n`` nodes: ``n - 1`` records over a Bool."""
    src = [b"warrant d v1\n  record R0(x: Bool)\n"]
    for i in range(1, n - 1):
        src.append(f"  record R{i}(x: R{i - 1})\n".encode())
    src.append(b"  op f(a: Nat) -> Nat\n")
    return lower_document(parse_source(b"".join(src))), T(f"R{n - 2}")


def _nested_options(n: int) -> A.Ty:
    ty = T("Bool")
    for _ in range(n - 1):
        ty = T("Option", (ty,))
    return ty


@pytest.mark.parametrize("nodes,ok", [(16, True), (17, False)])
def test_descriptor_depth_is_counted_in_nodes_for_records(nodes, ok):
    spec, ty = _nested_records(nodes)
    if ok:
        assert descriptor_for(spec, ty).depth() == nodes
    else:
        with pytest.raises(OracleError) as e:
            descriptor_for(spec, ty)
        assert e.value.code == "limit"


@pytest.mark.parametrize("nodes,ok", [(16, True), (17, False)])
def test_descriptor_depth_is_counted_in_nodes_for_options(nodes, ok):
    ty = _nested_options(nodes)
    if ok:
        assert descriptor_for(SPEC, ty).depth() == nodes
    else:
        with pytest.raises(OracleError) as e:
            descriptor_for(SPEC, ty)
        assert e.value.code == "limit"


# -- CV2-F17: derive -> verify-v2 end to end over demoted composites ---------------------------

_E2E = b"""warrant demoted v1
  record R(a: U16, b: Bool)
  variant M = X | Y(t: (U16, Bool))
  op ft(t: (U16, Bool)) -> Bool
  op fr(r: R) -> Bool
  op fm(m: M) -> Bool
  op fo(o: Option<U16>) -> Bool
  model kt(t: (U16, Bool)) -> Bool = true
  model kr(r: R) -> Bool = true
  model km(m: M) -> Bool = true
  model ko(o: Option<U16>) -> Bool = true
  claim c_tuple:
    forall t: (U16, Bool). ft(t) == kt(t)
  claim c_record:
    forall r: R. fr(r) == kr(r)
  claim c_variant:
    forall m: M. fm(m) == km(m)
  claim c_option:
    forall o: Option<U16>. fo(o) == ko(o)
"""


def test_derive_then_verify_v2_accepts_demoted_composites(tmp_path):
    from coding_adventures_warrant_oracle.corpus_v2 import canonical, derive
    from coding_adventures_warrant_oracle.verify_v2 import verify_v2

    profile = b"budget 1000\n"
    derived = derive(_E2E, profile, spec_ref="code/demoted.warrant", generator={"name": "t", "version": "0"})
    rows = {r["claim"]: r for r in derived.corpus["suites"]}
    for claim in ("c_tuple", "c_record", "c_variant", "c_option"):
        assert rows[claim]["demoted"] and rows[claim]["domain_label"] == "bounded-set"
    root = tmp_path / "corpus"
    (root / "chunks").mkdir(parents=True)
    (root / "corpus.json").write_bytes(canonical(derived.corpus))
    (root / "cases.tfv").write_bytes(derived.tfv)
    for name, text in derived.chunk_files.items():
        (root / "chunks" / name).write_bytes(text)
    (tmp_path / "code").mkdir()
    (tmp_path / "code" / "demoted.warrant").write_bytes(_E2E)
    (tmp_path / "corpus.profile").write_bytes(profile)
    report, _ = verify_v2(
        root, tmp_path / "code" / "demoted.warrant", tmp_path / "corpus.profile", None, cache={}, repo_root=tmp_path
    )
    assert report.ok, report.mismatches
