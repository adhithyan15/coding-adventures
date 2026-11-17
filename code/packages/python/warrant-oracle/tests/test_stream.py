"""Value streams, context prefixes and the SHA-256 framing of chunks and slim sets."""

from __future__ import annotations

import hashlib

import pytest

from coding_adventures_warrant_oracle import ast as A
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document
from coding_adventures_warrant_oracle.parser import parse_source
from coding_adventures_warrant_oracle.stream import (
    HEADER_V1,
    HEADER_V2,
    chunk_digest_framing,
    context_prefix,
    field,
    fnv1a64,
    in_v1_set,
    make_encoder,
    slim_base_set,
    suite_header,
    u32,
)

T = A.Ty
SPEC = lower_document(
    parse_source(
        b"warrant s v1\n  error E = BAD | WORSE\n  record Point(x: U8, y: U8)\n"
        b"  variant D = Short | Done(n: Nat, data: Bytes)\n  op f(a: Nat) -> Nat\n"
    )
)


def enc(ty, value):
    return make_encoder(SPEC, ty)(value)


def le(n, width=8):
    return n.to_bytes(width, "little")


def test_scalar_rows():
    assert enc(T("Bool"), False) == b"\x00" and enc(T("Bool"), True) == b"\x01"
    for ty in ("U8", "U16", "U32", "U64", "Nat"):
        assert enc(T(ty), 258) == b"\x10" + le(258)
    assert enc(T("U64"), (1 << 64) - 1) == b"\x10" + b"\xff" * 8


def test_signed_rows_are_sign_extended_twos_complement():
    assert enc(T("I8"), -1) == b"\x11" + b"\xff" * 8  # the spec's worked example
    assert enc(T("I8"), -128) == b"\x11" + le((1 << 64) - 128)
    assert enc(T("I64"), -(1 << 63)) == b"\x11" + le(1 << 63)
    assert enc(T("I32"), 5) == b"\x11" + le(5)


def test_bytes_rows():
    assert enc(T("Bytes"), b"\x01\x02") == b"\x20" + u32(2) + b"\x01\x02"
    assert enc(T("Bytes", (2,)), b"\x01\x02") == enc(T("Bytes"), b"\x01\x02")
    assert enc(T("Bytes"), b"") == b"\x20\x00\x00\x00\x00"


def test_composite_rows():
    point = enc(T("Point"), (1, 2))  # the worked example of section 2.4
    assert point == b"\x64" + u32(2) + b"\x10" + le(1) + b"\x10" + le(2)
    assert point.hex() == "6402000000" + "10" + "0100000000000000" + "10" + "0200000000000000"
    assert enc(T("()", (T("Bool"), T("U8"))), (True, 3)) == b"\x64" + u32(
        2
    ) + b"\x01" + b"\x10" + le(3)
    assert enc(T("Option", (T("Bool"),)), ("none",)) == b"\x40"
    assert enc(T("Option", (T("Bool"),)), ("some", True)) == b"\x41\x01"
    assert enc(T("List", (T("Bool"),)), ("list", (True, False))) == b"\x30" + u32(2) + b"\x01\x00"
    res = T("Result", (T("Bool"), T("E")))
    assert enc(res, ("ok", True)) == b"\x50\x01"
    assert enc(res, ("err", "BAD")) == b"\x51" + u32(3) + b"BAD"
    assert enc(T("D"), ("Short", ())) == b"\x65" + u32(5) + b"Short" + u32(0)
    done = enc(T("D"), ("Done", (7, b"\x01")))
    assert (
        done == b"\x65" + u32(4) + b"Done" + u32(2) + b"\x10" + le(7) + b"\x20" + u32(1) + b"\x01"
    )


def test_nested_composites_are_self_delimiting():
    pair = T("()", (T("Option", (T("Bool"),)), T("Option", (T("Bool"),))))
    assert enc(pair, (("none",), ("some", True))) != enc(pair, (("some", True), ("none",)))
    assert enc(T("List", (T("Bytes"),)), ("list", (b"a", b"b"))) != enc(
        T("List", (T("Bytes"),)), ("list", (b"ab",))
    )


def test_a_type_without_an_encoding_is_refused():
    with pytest.raises(OracleError):
        make_encoder(SPEC, T("Int"))


def test_u32_and_field_bounds():
    assert u32(0) == b"\x00" * 4 and u32((1 << 32) - 1) == b"\xff" * 4
    with pytest.raises(OracleError):
        u32(1 << 32)
    with pytest.raises(OracleError):
        u32(-1)
    assert field("é") == u32(2) + "é".encode()


def test_the_v1_type_set_and_header_choice():
    assert in_v1_set(T("Bytes", (4,))) and in_v1_set(T("Result", (T("Bytes"), T("E"))))
    assert in_v1_set(T("List", (T("Option", (T("U64"),)),)))
    assert (
        not in_v1_set(T("U16"))
        and not in_v1_set(T("I8"))
        and not in_v1_set(T("()", (T("Bool"), T("Bool"))))
    )
    assert not in_v1_set(T("Point")) and not in_v1_set(T("Option", (T("U32"),)))
    assert suite_header([T("U8"), T("Bytes", (2,))], T("Bool")) == HEADER_V1
    assert suite_header([T("U8")], T("Result", (T("Bytes"), T("E")))) == HEADER_V1
    assert suite_header([T("U16")], T("Bool")) == HEADER_V2
    assert suite_header([T("U8")], T("()", (T("Bool"), T("Bool")))) == HEADER_V2


def test_context_prefix_layout():
    prefix = context_prefix("s/id", [T("U8"), T("Bytes", (2,))], T("Bool"), HEADER_V1)
    assert prefix == (
        b"warrant-stream-v1\x00"
        + u32(4)
        + b"s/id"
        + u32(2)
        + u32(2)
        + b"U8"
        + u32(8)
        + b"Bytes<2>"
        + u32(4)
        + b"Bool"
    )


def test_chunk_digest_framing_is_a_listing_not_a_merge():
    d = [hashlib.sha256(bytes([i])).digest() for i in range(3)]
    expected = hashlib.sha256(b"warrant-chunks-v1\x00" + u32(3) + b"".join(d)).hexdigest()
    assert chunk_digest_framing(d) == expected
    assert chunk_digest_framing(d[:2]) != expected
    assert chunk_digest_framing(list(reversed(d))) != expected
    assert chunk_digest_framing([]) == hashlib.sha256(b"warrant-chunks-v1\x00" + u32(0)).hexdigest()


def test_fnv1a_64_reference_values():
    assert fnv1a64("") == 0xCBF29CE484222325
    assert fnv1a64("a") == 0xAF63DC4C8601EC8C  # the published FNV-1a 64 test vector
    assert fnv1a64("foobar") == 0x85944171F73967E8
    assert fnv1a64("é") == fnv1a64("é")  # over the UTF-8 bytes


def test_slim_selection_small_suites_are_full():
    assert slim_base_set("x", 1) == [0]
    assert slim_base_set("x", 16) == list(range(16))


def test_slim_selection_for_a_2_24_suite():
    m = 256  # k = ceil(256 / 14) = 19
    chosen = slim_base_set("warrant/gf256/exhaustive/add_associative", m)
    o = fnv1a64("warrant/gf256/exhaustive/add_associative") % 19
    assert chosen == sorted({0, 255, *range(o, 256, 19)})
    assert chosen[0] == 0 and chosen[-1] == 255 and len(chosen) <= 16
    assert chosen == sorted(set(chosen))


@pytest.mark.parametrize("m", [17, 18, 28, 29, 100, 255, 256, 1000, 4096])
def test_slim_selection_invariants(m):
    for name in ("a", "warrant/x/exhaustive/y", "zzzz"):
        chosen = slim_base_set(name, m)
        assert chosen[0] == 0 and chosen[-1] == m - 1
        assert chosen == sorted(set(chosen)) and 2 <= len(chosen) <= 16 + 1
        assert all(0 <= c < m for c in chosen)
        k = -(-m // 14)
        assert {c for c in chosen if c not in (0, m - 1)} == {
            c for c in range(fnv1a64(name) % k, m, k) if c not in (0, m - 1)
        }
