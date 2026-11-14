"""The finite domains, enumerated in this lane (VER04i 2.6)."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_corpus.domains import (
    binder_domain,
    domain_values,
    known_domain_names,
)
from coding_adventures_warrant_corpus.errors import CorpusError


def test_sizes_match_the_spec() -> None:
    assert domain_values("bool")[0] == [False, True]
    assert len(domain_values("u8_all")[0]) == 256
    assert len(domain_values("u64_edge")[0]) == 377
    assert domain_values("nat_0_64")[0][-1] == 64
    assert len(domain_values("bytes_d1")[0]) == 347


def test_bytes_d1_order() -> None:
    values, ty = domain_values("bytes_d1")
    assert ty == "Bytes"
    assert values[0] == b""
    assert values[1:257] == [bytes((i,)) for i in range(256)]
    # then every length-2 string over {00,01,80,ff}, lexicographically
    assert values[257] == b"\x00\x00" and values[258] == b"\x00\x01"
    assert values[272] == b"\xff\xff"
    assert values[273] == b"\x00\x00\x00"  # first length-3
    assert values[-2:] == [b"\x00" * 8, b"\xa5" * 8]
    assert len(set(values)) == len(values)


def test_u64_edge_order() -> None:
    values, ty = domain_values("u64_edge")
    assert ty == "U64"
    assert values[:256] == list(range(256))
    assert values[256] == 1 << 8 and values[311] == 1 << 63
    assert values[312] == (1 << 64) - 1
    assert values[313] == ((1 << 64) - 1) ^ 1
    assert values[-1] == ((1 << 64) - 1) ^ (1 << 63)


def test_bytes_n_small_and_large_widths() -> None:
    assert domain_values("bytes_n(0)") == ([b""], "Bytes<0>")
    assert len(domain_values("bytes_n(1)")[0]) == 256
    assert len(domain_values("bytes_n(2)")[0]) == 16
    four, ty = domain_values("bytes_n(4)")
    assert ty == "Bytes<4>" and len(four) == 16
    assert four[:4] == [b"\x00" * 4, b"\x01" * 4, b"\x80" * 4, b"\xff" * 4]
    assert four[4:7] == [b"\x01\x00\x00\x00", b"\x80\x00\x00\x00", b"\xff\x00\x00\x00"]
    assert len(domain_values("bytes_n(16)")[0]) == 52


def test_unknown_or_oversized_domains_are_refused() -> None:
    for name in ("nope", "bytes_n(x)", "bytes_n(99999)", ""):
        with pytest.raises(CorpusError):
            domain_values(name)


def test_binder_domain_checks_the_declared_type() -> None:
    assert binder_domain({"domain": "bool", "type": "Bool"})[1] == "Bool"
    assert binder_domain({"domain": "bool"})[1] == "Bool"  # type is optional
    for bad in (
        {"domain": "bool", "type": "U8"},
        {"domain": 3},
        {"type": "Bool"},
        {"domain": "bool", "extra": 1},
        "bool",
    ):
        with pytest.raises(CorpusError):
            binder_domain(bad)


def test_known_names() -> None:
    assert "bytes_d1" in known_domain_names()
