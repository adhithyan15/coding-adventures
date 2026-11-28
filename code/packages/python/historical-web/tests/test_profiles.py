import json
from dataclasses import asdict
from pathlib import Path

import pytest

from historical_web import (
    decode_http09_response,
    encode_http09_request,
    parse_html_tags_1992,
    parse_http09_request,
)

VECTORS = json.loads(
    (
        Path(__file__).resolve().parents[4]
        / "specs/fixtures/historical-web-v1/vectors.json"
    ).read_text(encoding="utf-8")
)
HTTP = VECTORS["http-0.9"]


def portable_tree(source: str) -> list:
    return [
        node if isinstance(node, str) else asdict(node)
        for node in parse_html_tags_1992(source)
    ]


@pytest.mark.parametrize("vector", HTTP["requests"])
def test_request(vector: dict) -> None:
    wire = bytes.fromhex(vector["wire_hex"])
    address = vector["address"]
    assert parse_http09_request(wire) == address
    assert (
        encode_http09_request(address)
        == wire.removesuffix(b"\n").rstrip(b"\r") + b"\r\n"
    )


@pytest.mark.parametrize("address", HTTP["invalid_addresses"])
def test_invalid_address(address: str) -> None:
    with pytest.raises(ValueError, match="document address"):
        encode_http09_request(address)


@pytest.mark.parametrize("wire_hex", HTTP["invalid_request_hex"])
def test_invalid_request(wire_hex: str) -> None:
    with pytest.raises(ValueError, match="request line"):
        parse_http09_request(bytes.fromhex(wire_hex))


@pytest.mark.parametrize("vector", HTTP["responses"])
def test_response(vector: dict) -> None:
    assert decode_http09_response(bytes.fromhex(vector["wire_hex"])) == vector["body"]


@pytest.mark.parametrize("wire_hex", HTTP["invalid_response_hex"])
def test_invalid_response(wire_hex: str) -> None:
    with pytest.raises(UnicodeDecodeError):
        decode_http09_response(bytes.fromhex(wire_hex))


@pytest.mark.parametrize(
    "vector", VECTORS["html-tags-1992"], ids=lambda case: case["id"]
)
def test_document(vector: dict) -> None:
    assert portable_tree(vector["source"]) == vector["tree"]


@pytest.mark.parametrize("exchange", VECTORS["exchanges"])
def test_early_web_exchange(exchange: dict) -> None:
    request = encode_http09_request(exchange["address"])
    assert request == bytes.fromhex(exchange["request_hex"])
    assert parse_http09_request(request) == exchange["address"]
    body = decode_http09_response(bytes.fromhex(exchange["response_hex"]))
    assert portable_tree(body) == exchange["tree"]


def test_deep_unmatched_end_tags_do_not_change_scope() -> None:
    """Unmatched tags after deep nesting remain harmless and linear-time."""
    source = "<A>" * 2_000 + "</TITLE>" * 2_000 + "tail"
    tree = parse_html_tags_1992(source)
    current = tree[0]
    for _ in range(1_999):
        assert current.tag == "a"
        current = current.children[0]
    assert current.tag == "a"
    assert current.children == ["tail"]
