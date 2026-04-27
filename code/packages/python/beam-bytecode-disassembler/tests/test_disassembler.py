import pytest

from beam_bytecode_disassembler import disassemble_bytes
from beam_bytecode_disassembler.disassembler import _decode_u, _decode_value

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


def test_disassemble_simple_module() -> None:
    module = disassemble_bytes(_build_test_beam())
    assert module.module_name == "demo"
    assert [instruction.opcode.name for instruction in module.instructions] == [
        "func_info",
        "label",
        "move",
        "move",
        "call_ext",
        "return",
    ]
    assert module.instructions[2].args[0].kind == "integer"
    assert module.instructions[2].args[0].value == 1
    assert module.find_export("main", 0) == 1


def test_decode_value_variants() -> None:
    atoms = (None, "demo", "main")
    assert _decode_u(bytes([0x50]), 0) == (5, 1)
    assert _decode_u(bytes([0x68, 0x2A]), 0) == (810, 2)
    assert _decode_u(bytes([0x18, 0x08, 0x00]), 0) == (2048, 3)
    integer_operand, _ = _decode_value(bytes([0x31]), 0, atoms)
    atom_operand, _ = _decode_value(bytes([0x22]), 0, atoms)
    x_operand, _ = _decode_value(bytes([0x13]), 0, atoms)
    nil_operand, _ = _decode_value(bytes([0x02]), 0, atoms)
    y_operand, _ = _decode_value(bytes([0x24]), 0, atoms)
    label_operand, _ = _decode_value(bytes([0x15]), 0, atoms)
    unsigned_operand, _ = _decode_value(bytes([0x60]), 0, atoms)
    h_operand, _ = _decode_value(bytes([0x26]), 0, atoms)
    assert integer_operand.kind == "integer"
    assert integer_operand.value == 3
    assert atom_operand.kind == "atom"
    assert atom_operand.value == "main"
    assert x_operand.kind == "x"
    assert x_operand.value == 1
    assert nil_operand.kind == "nil"
    assert y_operand.kind == "y"
    assert label_operand.kind == "label"
    assert unsigned_operand.kind == "u"
    assert h_operand.kind == "h"


def test_extended_operands_are_not_supported_yet() -> None:
    with pytest.raises(NotImplementedError, match="z-tag"):
        _decode_value(bytes([0x07]), 0, (None,))
