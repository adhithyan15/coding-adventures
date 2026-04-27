from __future__ import annotations

import pytest

from jvm_bytecode_disassembler import (
    JVMOpcode,
    JVMVersion,
    assemble_jvm,
    disassemble_method_body,
    encode_iconst,
)


def test_disassemble_simple_program() -> None:
    method = disassemble_method_body(
        assemble_jvm(
            (JVMOpcode.ICONST_1,),
            (JVMOpcode.ICONST_2,),
            (JVMOpcode.IADD,),
            (JVMOpcode.IRETURN,),
        ),
        version=JVMVersion(61, 0),
        max_stack=2,
        max_locals=0,
    )

    assert method.version.label == "Java 17"
    assert [instruction.mnemonic for instruction in method.instructions] == [
        "iconst_1",
        "iconst_2",
        "iadd",
        "ireturn",
    ]


def test_ldc_preserves_real_constant_pool_index() -> None:
    method = disassemble_method_body(
        assemble_jvm((JVMOpcode.LDC, 9), (JVMOpcode.IRETURN,)),
        constant_pool={9: 300},
    )

    assert method.instructions[0].constant_pool_index == 9
    assert method.constant_pool_lookup[9] == 300


def test_encode_iconst_uses_sipush_for_signed_short_values() -> None:
    assert encode_iconst(1024) == bytes([0x11, 0x04, 0x00])


def test_disassemble_locals_branches_and_member_refs() -> None:
    method = disassemble_method_body(
        assemble_jvm(
            (JVMOpcode.BIPUSH, -5),
            (JVMOpcode.SIPUSH, 1024),
            (JVMOpcode.ISTORE, 5),
            (JVMOpcode.ILOAD, 5),
            (JVMOpcode.GOTO, 3),
            (JVMOpcode.GETSTATIC, 12),
            (JVMOpcode.INVOKEVIRTUAL, 20),
            (JVMOpcode.RETURN,),
        ),
        constant_pool={12: "field-ref", 20: "method-ref"},
    )

    assert method.instructions[0].literal == -5
    assert method.instructions[1].literal == 1024
    assert method.instructions[2].local_slot == 5
    assert method.instructions[3].local_slot == 5
    assert method.instructions[4].branch_target == 12
    assert method.instructions[5].constant_pool_index == 12
    assert method.instructions[6].constant_pool_index == 20
    assert method.instruction_at(12).mnemonic == "getstatic"


def test_disassemble_invalid_version_and_unknown_opcode_raise() -> None:
    with pytest.raises(ValueError, match="Unsupported JVM class-file version"):
        disassemble_method_body(bytes([0x04]), version=JVMVersion(44, 0))

    with pytest.raises(ValueError, match="Unknown JVM opcode"):
        disassemble_method_body(bytes([0xFF]))


def test_assemble_jvm_missing_offset_raises() -> None:
    with pytest.raises(ValueError, match="requires an offset operand"):
        assemble_jvm((JVMOpcode.GOTO,))
