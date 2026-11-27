"""VER-backed bounded assurance for every public historical-web export."""

from __future__ import annotations

import contextlib
import itertools

import pytest
from coding_adventures_test_bootstrap import (
    FuzzOptions,
    PbtOptions,
    check_property,
    fuzz_function,
    gen_bytes,
    gen_choice,
    gen_list,
)

from historical_web import (
    Element,
    decode_http09_response,
    encode_http09_request,
    parse_html_tags_1992,
    parse_http09_request,
)


def _addresses() -> list[str]:
    """Independent finite address model: four symbols, lengths zero through three."""
    alphabet = "a/?0"
    return [
        "".join(chars)
        for length in range(4)
        for chars in itertools.product(alphabet, repeat=length)
    ]


def _tree_shape(nodes: list[object], depth: int = 0) -> None:
    """Check shape without using the parser's tokenizer, stack, or regexes."""
    assert depth <= 12
    for node in nodes:
        if isinstance(node, str):
            continue
        assert isinstance(node, Element)
        assert node.tag.isascii() and node.tag == node.tag.lower()
        assert isinstance(node.attributes, dict)
        _tree_shape(node.children, depth + 1)


def test_exhaustive_small_http_address_domain() -> None:
    valid = 0
    for address in _addresses():
        if not address:
            with pytest.raises(ValueError):
                encode_http09_request(address)
            continue
        valid += 1
        wire = encode_http09_request(address)
        assert wire == b"GET " + address.encode("ascii") + b"\r\n"
        assert parse_http09_request(wire) == address
        assert parse_http09_request(wire[:-2] + b"\n") == address
    assert valid == 84


@pytest.mark.parametrize(
    "wire",
    [b"GET / x\r\n", b"GET /\r\nextra", b"POST /\r\n", b"GET \xff\r\n"],
)
def test_http_negative_controls(wire: bytes) -> None:
    with pytest.raises(ValueError):
        parse_http09_request(wire)


def test_exhaustive_small_response_domain() -> None:
    alphabet = (b"\x00", b"A", b"\x7f")
    cases = [b""]
    cases.extend(
        b"".join(chars)
        for length in (1, 2)
        for chars in itertools.product(alphabet, repeat=length)
    )
    for body in cases:
        assert decode_http09_response(body) == body.decode("ascii")
    with pytest.raises(UnicodeDecodeError):
        decode_http09_response(b"\x80")
    assert len(cases) == 13


def test_pbt_real_wire_round_trip() -> None:
    address_gen = gen_list(gen_choice(list("a/?0")), 1, 12).map("".join)
    verdict = check_property(
        0x48574542,
        address_gen,
        lambda address: parse_http09_request(encode_http09_request(address)) == address,
        PbtOptions(num_tests=256, ci=False),
    )
    assert verdict.passed, verdict.message
    assert verdict.cases_run == 256


def test_pbt_real_tree_invariants() -> None:
    verdict = check_property(
        0x1992,
        gen_bytes(0, 64),
        lambda data: _tree_shape(parse_html_tags_1992(data.decode("latin1"))),
        PbtOptions(num_tests=256, ci=False),
    )
    assert verdict.passed, verdict.message
    assert verdict.cases_run == 256


def _fuzz_exports(data: bytes) -> None:
    """Exercise every public codec/parser while treating documented rejects as data."""
    with contextlib.suppress(ValueError):
        parse_http09_request(data)
    with contextlib.suppress(UnicodeDecodeError):
        decode_http09_response(data)
    with contextlib.suppress(ValueError):
        encode_http09_request(data.decode("latin1"))
    _tree_shape(parse_html_tags_1992(data.decode("latin1")))


def test_fuzz_real_tree_export_is_crash_clean() -> None:
    report = fuzz_function(
        _fuzz_exports,
        [b"<UL><LI>x<LI>y</UL>", b"<PLAINTEXT>", b"&lt;"],
        FuzzOptions(max_iterations=512, seed=0x19921103, max_input_len=128),
    )
    assert report.iterations == 512
    assert report.is_clean
    assert not report.crashes and not report.timeouts and not report.ooms
