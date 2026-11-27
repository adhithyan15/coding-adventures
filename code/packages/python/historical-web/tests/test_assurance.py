"""VER-backed bounded assurance for every public historical-web export."""

from __future__ import annotations

import contextlib
import itertools
from collections.abc import Callable

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


def _tree_shape(nodes: list[object]) -> None:
    """Iteratively check shape, ownership, and acyclicity independently."""
    pending = list(nodes)
    seen: set[int] = set()
    while pending:
        node = pending.pop()
        if isinstance(node, str):
            continue
        assert isinstance(node, Element)
        identity = id(node)
        assert identity not in seen
        seen.add(identity)
        assert node.tag.isascii() and node.tag == node.tag.lower()
        assert isinstance(node.attributes, dict)
        pending.extend(node.children)


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


def _expected_request(wire: bytes) -> tuple[bool, str | None]:
    """Independent request model; deliberately avoids production regexes."""
    suffix = b"\r\n" if wire.endswith(b"\r\n") else b"\n"
    if not wire.startswith(b"GET ") or not wire.endswith(suffix):
        return False, None
    address = wire[4 : -len(suffix)]
    if not address or any(byte < 33 or byte > 126 for byte in address):
        return False, None
    return True, address.decode("ascii")


def _check_http_wire_oracle(
    parse_fn: Callable[[bytes], str],
    encode_fn: Callable[[str], bytes],
    decode_fn: Callable[[bytes], str],
    wires: list[bytes],
) -> None:
    """Check acceptance and exact output, including documented refusals."""
    for wire in wires:
        accepted, address = _expected_request(wire)
        try:
            observed = parse_fn(wire)
        except ValueError:
            assert not accepted
        else:
            assert accepted and observed == address
    for address in ("", "a", "/x?y", "a b", "é"):
        accepted = bool(address) and all(33 <= ord(char) <= 126 for char in address)
        try:
            observed = encode_fn(address)
        except ValueError:
            assert not accepted
        else:
            assert accepted and observed == b"GET " + address.encode("ascii") + b"\r\n"
    for body in wires:
        if all(byte < 128 for byte in body):
            assert decode_fn(body) == body.decode("ascii")
        else:
            with pytest.raises(UnicodeDecodeError):
                decode_fn(body)


def test_http_checker_rejects_wrong_implementation() -> None:
    wires = [b"GET /\r\n", b"GET /x\n", b"POST /\r\n", b"\xff"]

    def wrong_parse(_: bytes) -> str:
        return "/"

    with pytest.raises(AssertionError):
        _check_http_wire_oracle(
            wrong_parse, encode_http09_request, decode_http09_response, wires
        )


def test_pbt_arbitrary_wire_acceptance_and_outputs() -> None:
    def property_for_wire(wire: bytes) -> None:
        _check_http_wire_oracle(
            parse_http09_request,
            encode_http09_request,
            decode_http09_response,
            [wire],
        )

    verdict = check_property(
        0xBADC0DE,
        gen_bytes(0, 32),
        property_for_wire,
        PbtOptions(num_tests=256, ci=False),
    )
    assert verdict.passed, verdict.message
    assert verdict.cases_run == 256


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


def _list_semantic_oracle(
    parse_fn: Callable[[str], list[object]], left: str, right: str
) -> None:
    """Independent semantic oracle for omitted list-item end tags."""
    tree = parse_fn(f"<UL><LI>{left}<LI>{right}</UL>")
    explicit = parse_fn(f"<UL><LI>{left}</LI><LI>{right}</LI></UL>")
    assert len(tree) == len(explicit) == 1
    assert isinstance(tree[0], Element) and isinstance(explicit[0], Element)
    assert tree[0].tag == explicit[0].tag == "ul"
    assert [child.children for child in tree[0].children] == [[left], [right]]
    assert [child.children for child in explicit[0].children] == [[left], [right]]


def test_generated_html_omitted_li_semantics_and_wrong_control() -> None:
    text_gen = gen_list(gen_choice(list("ab01")), 1, 8).map("".join)
    pair_gen = text_gen.flat_map(
        lambda left: text_gen.map(lambda right: (left, right))
    )
    verdict = check_property(
        0x1A2B,
        pair_gen,
        lambda pair: _list_semantic_oracle(parse_html_tags_1992, *pair),
        PbtOptions(num_tests=256, ci=False),
    )
    assert verdict.passed, verdict.message
    assert verdict.cases_run == 256

    with pytest.raises(AssertionError):
        _list_semantic_oracle(lambda _: [], "a", "b")


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
        FuzzOptions(max_iterations=2048, seed=0x19921103, max_input_len=128),
    )
    assert report.iterations == 2048
    assert report.is_clean
    assert not report.crashes and not report.timeouts and not report.ooms
