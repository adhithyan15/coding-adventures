from __future__ import annotations

import pytest

from cil_bytecode_builder import (
    CILBranchKind,
    CILBuilderError,
    CILBytecodeBuilder,
    CILOpcode,
    encode_i4,
    encode_ldarg,
    encode_ldc_i4,
    encode_ldloc,
    encode_metadata_token,
    encode_starg,
    encode_stloc,
)


def test_encode_ldc_i4_uses_compact_forms() -> None:
    assert encode_ldc_i4(-1) == bytes([0x15])
    assert encode_ldc_i4(0) == bytes([0x16])
    assert encode_ldc_i4(8) == bytes([0x1E])
    assert encode_ldc_i4(127) == bytes([0x1F, 0x7F])
    assert encode_ldc_i4(-128) == bytes([0x1F, 0x80])
    assert encode_ldc_i4(128) == bytes([0x20, 0x80, 0x00, 0x00, 0x00])
    assert encode_ldc_i4(-129) == bytes([0x20, 0x7F, 0xFF, 0xFF, 0xFF])


def test_encode_ldc_i4_rejects_values_outside_int32() -> None:
    with pytest.raises(CILBuilderError, match="outside int32 range"):
        encode_ldc_i4(2**31)
    with pytest.raises(CILBuilderError, match="outside int32 range"):
        encode_i4(-(2**31) - 1)


def test_encode_local_and_argument_slots() -> None:
    assert encode_ldloc(0) == bytes([0x06])
    assert encode_ldloc(3) == bytes([0x09])
    assert encode_ldloc(4) == bytes([0x11, 0x04])
    assert encode_ldloc(300) == bytes([0xFE, 0x0C, 0x2C, 0x01])

    assert encode_stloc(0) == bytes([0x0A])
    assert encode_stloc(4) == bytes([0x13, 0x04])
    assert encode_stloc(300) == bytes([0xFE, 0x0E, 0x2C, 0x01])

    assert encode_ldarg(2) == bytes([0x04])
    assert encode_ldarg(4) == bytes([0x0E, 0x04])
    assert encode_ldarg(300) == bytes([0xFE, 0x09, 0x2C, 0x01])

    assert encode_starg(4) == bytes([0x10, 0x04])
    assert encode_starg(300) == bytes([0xFE, 0x0B, 0x2C, 0x01])


def test_indexed_helpers_reject_slots_outside_uint16() -> None:
    with pytest.raises(CILBuilderError, match="outside uint16 range"):
        encode_ldloc(-1)
    with pytest.raises(CILBuilderError, match="outside uint16 range"):
        encode_starg(65536)


def test_metadata_token_encoding() -> None:
    assert encode_metadata_token(0x0A000001) == bytes([0x01, 0x00, 0x00, 0x0A])
    with pytest.raises(CILBuilderError, match="outside uint32 range"):
        encode_metadata_token(2**32)


def test_builder_rejects_invalid_opcode_values() -> None:
    builder = CILBytecodeBuilder()

    with pytest.raises(CILBuilderError, match="outside uint8 range"):
        builder.emit_opcode(0x100)
    with pytest.raises(CILBuilderError, match="outside uint8 range"):
        builder.emit_token_instruction(-1, 0x06000001)


def test_builder_emits_arithmetic_method_body() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(1)
    builder.emit_ldc_i4(2)
    builder.emit_add()
    builder.emit_stloc(0)
    builder.emit_ldloc(0)
    builder.emit_ret()

    assert builder.assemble() == bytes([0x17, 0x18, 0x58, 0x0A, 0x06, 0x2A])


def test_builder_emits_tokens_and_two_byte_comparisons() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_call(0x06000001)
    builder.emit_callvirt(0x0A000002)
    builder.emit_ldsfld(0x04000003)
    builder.emit_stsfld(0x04000004)
    builder.emit_newarr(0x01000005)
    builder.emit_ceq()
    builder.emit_cgt()
    builder.emit_clt()

    assert builder.assemble() == (
        bytes([0x28, 0x01, 0x00, 0x00, 0x06])
        + bytes([0x6F, 0x02, 0x00, 0x00, 0x0A])
        + bytes([0x7E, 0x03, 0x00, 0x00, 0x04])
        + bytes([0x80, 0x04, 0x00, 0x00, 0x04])
        + bytes([0x8D, 0x05, 0x00, 0x00, 0x01])
        + bytes([0xFE, 0x01, 0xFE, 0x02, 0xFE, 0x04])
    )


def test_short_forward_and_backward_branches() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_branch(CILBranchKind.ALWAYS, "end")
    builder.mark("loop")
    builder.emit_ldc_i4(1)
    builder.emit_branch(CILBranchKind.TRUE, "loop")
    builder.mark("end")
    builder.emit_ret()

    assert builder.assemble() == bytes([0x2B, 0x03, 0x17, 0x2D, 0xFD, 0x2A])


def test_all_branch_kinds_have_short_encodings() -> None:
    builder = CILBytecodeBuilder()
    for kind in CILBranchKind:
        builder.emit_branch(kind, "target")
    builder.mark("target")
    bytecode = builder.assemble()

    assert bytecode == bytes(
        [
            CILOpcode.BR_S,
            16,
            CILOpcode.BRFALSE_S,
            14,
            CILOpcode.BRTRUE_S,
            12,
            CILOpcode.BEQ_S,
            10,
            CILOpcode.BGE_S,
            8,
            CILOpcode.BGT_S,
            6,
            CILOpcode.BLE_S,
            4,
            CILOpcode.BLT_S,
            2,
            CILOpcode.BNE_UN_S,
            0,
        ]
    )


def test_branch_auto_promotes_to_long_form() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_branch(CILBranchKind.ALWAYS, "far")
    for _ in range(130):
        builder.emit_opcode(CILOpcode.NOP)
    builder.mark("far")
    builder.emit_ret()

    bytecode = builder.assemble()

    assert bytecode[:5] == bytes([0x38, 0x82, 0x00, 0x00, 0x00])
    assert bytecode[-1:] == bytes([0x2A])


def test_branch_can_be_forced_long() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_branch(CILBranchKind.FALSE, "target", force_long=True)
    builder.mark("target")

    assert builder.assemble() == bytes([0x39, 0x00, 0x00, 0x00, 0x00])


def test_builder_rejects_label_errors() -> None:
    with pytest.raises(CILBuilderError, match="label must not be empty"):
        CILBytecodeBuilder().mark("")
    with pytest.raises(CILBuilderError, match="branch label must not be empty"):
        CILBytecodeBuilder().emit_branch(CILBranchKind.ALWAYS, "")
    with pytest.raises(CILBuilderError, match="unknown label"):
        CILBytecodeBuilder().emit_branch(CILBranchKind.ALWAYS, "missing").assemble()
    with pytest.raises(CILBuilderError, match="unknown branch kind"):
        CILBytecodeBuilder().emit_branch("maybe", "target")  # type: ignore[arg-type]

    builder = CILBytecodeBuilder()
    builder.mark("same")
    builder.mark("same")
    with pytest.raises(CILBuilderError, match="duplicate label"):
        builder.assemble()


def test_fluent_helpers_return_builder() -> None:
    builder = CILBytecodeBuilder()

    returned = (
        builder.emit_raw(b"\x00")
        .emit_opcode(CILOpcode.DUP)
        .emit_ldarg(0)
        .emit_starg(0)
        .emit_sub()
        .emit_mul()
        .emit_div()
        .emit_ret()
    )

    assert returned is builder
    assert builder.assemble() == bytes(
        [0x00, 0x25, 0x02, 0x10, 0x00, 0x59, 0x5A, 0x5B, 0x2A]
    )
