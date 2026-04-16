import pytest
from beam_opcode_metadata import OTP_28_PROFILE

from beam_bytes_decoder import decode_beam_module, parse_beam_container
from beam_bytes_decoder.decoder import _decode_compact_u

TAG_U = 0
TAG_I = 1
TAG_A = 2
TAG_X = 3


def _encode_small(tag: int, value: int) -> bytes:
    return bytes([(value << 4) | tag])


def _chunk(chunk_id: str, payload: bytes) -> bytes:
    padding = b"\x00" * ((4 - (len(payload) % 4)) % 4)
    return (
        chunk_id.encode("ascii")
        + len(payload).to_bytes(4, "big")
        + payload
        + padding
    )


def _build_test_beam() -> bytes:
    atoms = ["demo", "main", "erlang", "+"]
    atom_payload = (-len(atoms)).to_bytes(4, "big", signed=True)
    for atom in atoms:
        atom_bytes = atom.encode("utf-8")
        atom_payload += _encode_small(TAG_U, len(atom_bytes)) + atom_bytes

    code = b"".join(
        [
            bytes([2]),
            _encode_small(TAG_A, 1),
            _encode_small(TAG_A, 2),
            _encode_small(TAG_U, 0),
            bytes([1]),
            _encode_small(TAG_U, 1),
            bytes([64]),
            _encode_small(TAG_I, 1),
            _encode_small(TAG_X, 0),
            bytes([64]),
            _encode_small(TAG_I, 2),
            _encode_small(TAG_X, 1),
            bytes([7]),
            _encode_small(TAG_U, 2),
            _encode_small(TAG_U, 0),
            bytes([19]),
        ]
    )
    code_payload = (
        (16).to_bytes(4, "big")
        + (0).to_bytes(4, "big")
        + (64).to_bytes(4, "big")
        + (1).to_bytes(4, "big")
        + (1).to_bytes(4, "big")
        + code
    )
    imp_payload = (
        (1).to_bytes(4, "big")
        + (3).to_bytes(4, "big")
        + (4).to_bytes(4, "big")
        + (2).to_bytes(4, "big")
    )
    exp_payload = (
        (1).to_bytes(4, "big")
        + (2).to_bytes(4, "big")
        + (0).to_bytes(4, "big")
        + (1).to_bytes(4, "big")
    )
    chunks = b"".join(
        [
            _chunk("AtU8", atom_payload),
            _chunk("Code", code_payload),
            _chunk("StrT", b""),
            _chunk("ImpT", imp_payload),
            _chunk("ExpT", exp_payload),
        ]
    )
    form_payload = b"BEAM" + chunks
    return b"FOR1" + len(form_payload).to_bytes(4, "big") + form_payload


def test_parse_beam_container() -> None:
    container = parse_beam_container(_build_test_beam())
    assert container.form_id == "BEAM"
    assert [chunk.chunk_id for chunk in container.chunks] == [
        "AtU8",
        "Code",
        "StrT",
        "ImpT",
        "ExpT",
    ]


def test_decode_beam_module() -> None:
    module = decode_beam_module(_build_test_beam(), OTP_28_PROFILE)
    assert module.module_name == "demo"
    assert module.atoms[2] == "main"
    assert module.code_header.function_count == 1
    assert module.imports[0].module == "erlang"
    assert module.imports[0].function == "+"
    assert module.exports[0].function == "main"
    assert module.exports[0].label == 1


def test_decode_compact_u_variants() -> None:
    assert _decode_compact_u(bytes([0x50])) == (5, 1)
    assert _decode_compact_u(bytes([0x68, 0x2A])) == (810, 2)
    assert _decode_compact_u(bytes([0x18, 0x08, 0x00])) == (2048, 3)


def test_invalid_beam_header_raises() -> None:
    with pytest.raises(ValueError, match="FOR1"):
        parse_beam_container(b"nope")


def test_missing_required_chunk_raises() -> None:
    empty_beam = b"FOR1" + (4).to_bytes(4, "big") + b"BEAM"
    with pytest.raises(ValueError, match="AtU8"):
        decode_beam_module(empty_beam, OTP_28_PROFILE)
