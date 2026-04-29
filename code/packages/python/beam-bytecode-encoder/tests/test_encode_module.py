"""Module-level encoding tests.

Validates that ``encode_beam`` produces a byte structure with:

- The standard ``FOR1<u32>BEAM`` IFF preamble.
- The five required chunks in the right order with correct sizes
  and 4-byte padding.
- Atom-table contents matching what the caller passed in.

Round-trip parity against ``beam-bytes-decoder`` lives in
``test_round_trip.py``; this file focuses on byte-level shape so
that decoder bugs don't mask encoder bugs.
"""

from __future__ import annotations

import struct

import pytest
from beam_bytecode_encoder import (
    BEAMEncodeError,
    BEAMExport,
    BEAMImport,
    BEAMInstruction,
    BEAMModule,
    encode_beam,
)


def _smallest_module() -> BEAMModule:
    """The smallest non-degenerate module: one atom (the module
    name), an empty code body, one export with a synthesised label."""
    return BEAMModule(
        name="empty",
        atoms=("empty",),
        instructions=(),
        exports=(BEAMExport(function_atom_index=1, arity=0, label=1),),
        label_count=1,
    )


def _parse_chunks(data: bytes) -> dict[str, bytes]:
    """Tiny inline IFF parser — duplicates beam-bytes-decoder logic
    on purpose, so this test file doesn't depend on the decoder.
    Round-trip parity vs. the real decoder is exercised by
    ``test_round_trip.py``."""
    assert data[:4] == b"FOR1"
    declared_size = struct.unpack(">I", data[4:8])[0]
    assert declared_size + 8 == len(data), (
        f"FOR1 size mismatch: declared={declared_size}, "
        f"actual={len(data) - 8}"
    )
    assert data[8:12] == b"BEAM"

    chunks: dict[str, bytes] = {}
    offset = 12
    while offset < len(data):
        chunk_id = data[offset : offset + 4].decode("ascii")
        size = struct.unpack(">I", data[offset + 4 : offset + 8])[0]
        payload = data[offset + 8 : offset + 8 + size]
        chunks[chunk_id] = payload
        # Skip past the payload + alignment padding to the next chunk.
        offset += 8 + size + ((4 - (size % 4)) % 4)
    return chunks


class TestPreamble:
    def test_starts_with_for1_beam(self) -> None:
        data = encode_beam(_smallest_module())
        assert data[:4] == b"FOR1"
        assert data[8:12] == b"BEAM"

    def test_for1_size_equals_total_minus_eight(self) -> None:
        data = encode_beam(_smallest_module())
        declared = struct.unpack(">I", data[4:8])[0]
        assert declared + 8 == len(data)


class TestRequiredChunksPresent:
    def test_smallest_module_has_required_chunks(self) -> None:
        data = encode_beam(_smallest_module())
        chunks = _parse_chunks(data)
        # The decoder's required chunk set, plus StrT (which we
        # always emit even though decoder doesn't require it).
        for required in ("AtU8", "Code", "StrT", "ImpT", "ExpT"):
            assert required in chunks, f"missing chunk {required!r}"

    def test_locals_chunk_omitted_when_empty(self) -> None:
        data = encode_beam(_smallest_module())
        chunks = _parse_chunks(data)
        assert "LocT" not in chunks

    def test_locals_chunk_present_when_nonempty(self) -> None:
        module = BEAMModule(
            name="x",
            atoms=("x", "helper"),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            locals_=(BEAMExport(2, 0, 2),),
            label_count=2,
        )
        chunks = _parse_chunks(encode_beam(module))
        assert "LocT" in chunks


class TestChunkPayloads:
    def test_atu8_round_trips(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m", "foo", "bar"),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        chunks = _parse_chunks(encode_beam(module))
        atu8 = chunks["AtU8"]
        # First 4 bytes: count
        assert struct.unpack(">I", atu8[:4])[0] == 3
        # Atoms inline as <u8 length><utf8 bytes>
        offset = 4
        for expected in ("m", "foo", "bar"):
            length = atu8[offset]
            text = atu8[offset + 1 : offset + 1 + length].decode("utf-8")
            assert text == expected
            offset += 1 + length

    def test_code_chunk_header_layout(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m",),
            instructions=(BEAMInstruction(opcode=153),),  # arbitrary opcode
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
            instruction_set_version=0,
        )
        chunks = _parse_chunks(encode_beam(module))
        code = chunks["Code"]
        sub_size, fmt, max_op, label_count, fn_count = struct.unpack(
            ">IIIII", code[:20]
        )
        assert sub_size == 16
        assert fmt == 0
        assert max_op == 153
        assert label_count == 1
        assert fn_count == 1
        # Body starts immediately after the header (sub_size+4 = 20 bytes).
        assert code[20:] == bytes([153])

    def test_impt_and_expt_layouts(self) -> None:
        module = BEAMModule(
            name="m",
            atoms=("m", "erlang", "halt", "main"),
            instructions=(),
            imports=(BEAMImport(2, 3, 1),),  # erlang:halt/1
            exports=(BEAMExport(4, 0, 1),),
            label_count=1,
        )
        chunks = _parse_chunks(encode_beam(module))
        impt = chunks["ImpT"]
        assert struct.unpack(">I", impt[:4])[0] == 1
        assert struct.unpack(">III", impt[4:16]) == (2, 3, 1)
        expt = chunks["ExpT"]
        assert struct.unpack(">I", expt[:4])[0] == 1
        assert struct.unpack(">III", expt[4:16]) == (4, 0, 1)


class TestPadding:
    def test_chunks_are_four_byte_aligned(self) -> None:
        # Use atom names that produce a non-aligned AtU8 payload.
        module = BEAMModule(
            name="aa",  # 2-byte atom + 1-byte length = 3 bytes plus 4-byte count = 7 bytes
            atoms=("aa",),
            instructions=(),
            exports=(BEAMExport(1, 0, 1),),
            label_count=1,
        )
        data = encode_beam(module)
        # Walk the IFF stream and assert each chunk-end lands on a
        # 4-byte boundary relative to the start of the file.
        offset = 12  # past "FOR1<u32>BEAM"
        while offset < len(data):
            size = struct.unpack(">I", data[offset + 4 : offset + 8])[0]
            offset += 8 + size + ((4 - (size % 4)) % 4)
            assert offset % 4 == 0, f"chunk boundary at {offset} is unaligned"


class TestValidation:
    def test_empty_atoms_rejected(self) -> None:
        with pytest.raises(BEAMEncodeError, match="atoms must not be empty"):
            encode_beam(
                BEAMModule(
                    name="x",
                    atoms=(),
                    instructions=(),
                    exports=(),
                    label_count=0,
                )
            )

    def test_name_must_match_first_atom(self) -> None:
        with pytest.raises(BEAMEncodeError, match="must equal atoms"):
            encode_beam(
                BEAMModule(
                    name="x",
                    atoms=("y",),
                    instructions=(),
                    exports=(),
                    label_count=0,
                )
            )

    def test_export_with_oob_atom_rejected(self) -> None:
        with pytest.raises(BEAMEncodeError, match="function_atom_index"):
            encode_beam(
                BEAMModule(
                    name="m",
                    atoms=("m",),
                    instructions=(),
                    exports=(BEAMExport(99, 0, 1),),
                    label_count=1,
                )
            )

    def test_export_label_zero_rejected(self) -> None:
        with pytest.raises(BEAMEncodeError, match="label must be >= 1"):
            encode_beam(
                BEAMModule(
                    name="m",
                    atoms=("m",),
                    instructions=(),
                    exports=(BEAMExport(1, 0, 0),),
                    label_count=1,
                )
            )

    def test_export_label_above_count_rejected(self) -> None:
        with pytest.raises(BEAMEncodeError, match="label_count"):
            encode_beam(
                BEAMModule(
                    name="m",
                    atoms=("m",),
                    instructions=(),
                    exports=(BEAMExport(1, 0, 5),),
                    label_count=2,
                )
            )

    def test_oversize_atom_rejected(self) -> None:
        big = "x" * 256
        with pytest.raises(BEAMEncodeError, match="< 256 bytes"):
            encode_beam(
                BEAMModule(
                    name=big,
                    atoms=(big,),
                    instructions=(),
                    exports=(BEAMExport(1, 0, 1),),
                    label_count=1,
                )
            )
