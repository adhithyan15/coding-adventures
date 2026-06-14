"""Round-trip tests: encode → decode → assert structural equality.

These tests use the in-house ``beam-bytes-decoder`` (which has no
runtime dep on real ``erl``) as the ground-truth oracle.  The
hypothesis: anything we encode must decode cleanly and produce
the same atom list / instruction stream / export rows we put in.

If a future BEAM02 spec ships richer chunks (``LitT``, ``FunT``),
add round-trip cases here BEFORE wiring them into the encoder.
"""

from __future__ import annotations

from beam_bytecode_encoder import (
    BEAMExport,
    BEAMImport,
    BEAMInstruction,
    BEAMModule,
    BEAMOperand,
    BEAMTag,
    encode_beam,
)
from beam_bytes_decoder import decode_beam_module


class TestSmallestModule:
    def test_decoder_accepts_smallest_module(self) -> None:
        module = BEAMModule(
            name="empty",
            atoms=("empty",),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert decoded.module_name == "empty"
        # The decoder prepends ``None`` at index 0 to its atom tuple.
        assert decoded.atoms == (None, "empty")


class TestAtomTablePreserved:
    def test_three_atoms_round_trip(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m", "foo", "bar"),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert decoded.atoms == (None, "m", "foo", "bar")


class TestImportTableRoundTrip:
    def test_one_import(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m", "erlang", "halt"),
            instructions=(),
            imports=(BEAMImport(2, 3, 1),),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert len(decoded.imports) == 1
        imp = decoded.imports[0]
        assert imp.module == "erlang"
        assert imp.function == "halt"
        assert imp.arity == 1


class TestExportTableRoundTrip:
    def test_two_exports(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m", "main", "module_info"),
            instructions=(),
            exports=(
                BEAMExport(2, 0, 1),
                BEAMExport(3, 0, 2),
            ),
            label_count=2,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert len(decoded.exports) == 2
        assert decoded.exports[0].function == "main"
        assert decoded.exports[0].arity == 0
        assert decoded.exports[0].label == 1
        assert decoded.exports[1].function == "module_info"
        assert decoded.exports[1].label == 2


class TestCodeChunkHeaderRoundTrip:
    def test_max_opcode_derived_from_instructions(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m",),
            instructions=(
                BEAMInstruction(opcode=1),
                BEAMInstruction(opcode=42),
                BEAMInstruction(opcode=7),
            ),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert decoded.code_header.max_opcode == 42

    def test_label_count_preserved(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m",),
            instructions=(),
            exports=(BEAMExport(1, 0, 3),),
            label_count=3,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert decoded.code_header.label_count == 3


class TestInstructionStreamPreserved:
    def test_empty_instructions(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m",),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        assert decoded.code_header.code == b""

    def test_instruction_with_one_operand(self) -> None:
        # opcode=64 (move) operand = ``{integer, 42}``, ``{x, 0}``
        # Encoding: opcode byte, then encode_compact_term(I, 42),
        # then encode_compact_term(X, 0).
        module = BEAMModule(
            name="m",
            atoms=("m",),
            instructions=(
                BEAMInstruction(
                    opcode=64,
                    operands=(
                        BEAMOperand(BEAMTag.I, 42),
                        BEAMOperand(BEAMTag.X, 0),
                    ),
                ),
            ),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        decoded = decode_beam_module(encode_beam(module))
        # value=42 fits in medium form: high 3 bits = (42 >> 8) = 0,
        # second byte = 42; first byte = 0b00001001 (tag I=1, bit 3=1).
        # value=0 fits in small form: 0b00000011 = 0x03.
        # Stream: opcode 64, then 0x09, 0x2a, 0x03.
        assert decoded.code_header.code == bytes([64, 0x09, 0x2A, 0x03])
