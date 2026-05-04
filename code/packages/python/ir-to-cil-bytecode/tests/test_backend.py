from __future__ import annotations

import pytest
from compiler_ir import (
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILBackendError,
    CILHelper,
    CILLoweringPipeline,
    CILMethodArtifact,
    CILProgramArtifact,
    CILTokenProvider,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
    validate_for_clr,
)


class FixedTokenProvider:
    def method_token(self, method_name: str) -> int:
        return {
            "_start": 0x06000001,
            "callee": 0x06000002,
        }[method_name]

    def helper_token(self, helper: CILHelper) -> int:
        return {
            CILHelper.MEM_LOAD_BYTE: 0x0A000011,
            CILHelper.MEM_STORE_BYTE: 0x0A000012,
            CILHelper.LOAD_WORD: 0x0A000013,
            CILHelper.STORE_WORD: 0x0A000014,
            CILHelper.SYSCALL: 0x0A000015,
        }[helper]


def test_lower_arithmetic_return_method() -> None:
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(40)]),
        IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(0), IrImmediate(2)]),
        IrInstruction(IrOp.RET),
    ))

    assert artifact.callable_labels == ("_start",)
    assert artifact.entry_method.local_count == 5
    assert artifact.entry_method.body == bytes(
        [0x1F, 0x28, 0x0A, 0x06, 0x18, 0x58, 0x0B, 0x07, 0x2A]
    )


def test_lower_comparisons_and_bitwise_and() -> None:
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(7)]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(9)]),
        IrInstruction(IrOp.CMP_LT, [IrRegister(2), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.CMP_NE, [IrRegister(3), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.AND_IMM, [IrRegister(1), IrRegister(3), IrImmediate(1)]),
        IrInstruction(IrOp.RET),
    ))

    assert artifact.entry_method.body == bytes(
        [
            0x1D,
            0x0A,
            0x1F,
            0x09,
            0x0B,
            0x06,
            0x07,
            0xFE,
            0x04,
            0x0C,
            0x06,
            0x07,
            0xFE,
            0x01,
            0x16,
            0xFE,
            0x01,
            0x0D,
            0x09,
            0x17,
            0x5F,
            0x0B,
            0x07,
            0x2A,
        ]
    )


def test_lower_branches_within_callable_region() -> None:
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(3)]),
        IrInstruction(IrOp.LABEL, [IrLabel("loop")]),
        IrInstruction(IrOp.ADD_IMM, [IrRegister(0), IrRegister(0), IrImmediate(-1)]),
        IrInstruction(IrOp.BRANCH_NZ, [IrRegister(0), IrLabel("loop")]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0)]),
        IrInstruction(IrOp.RET),
    ))

    assert artifact.entry_method.body == bytes(
        [0x19, 0x0A, 0x06, 0x15, 0x58, 0x0A, 0x06, 0x2D, 0xF9, 0x16, 0x0B, 0x07, 0x2A]
    )


def test_lower_calls_use_injected_method_tokens() -> None:
    artifact = lower_ir_to_cil_bytecode(
        _program(
            IrInstruction(IrOp.CALL, [IrLabel("callee")]),
            IrInstruction(IrOp.RET),
            IrInstruction(IrOp.LABEL, [IrLabel("callee")]),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(7)]),
            IrInstruction(IrOp.RET),
        ),
        token_provider=FixedTokenProvider(),
    )

    assert artifact.callable_labels == ("_start", "callee")
    assert artifact.entry_method.body == bytes(
        [0x28, 0x02, 0x00, 0x00, 0x06, 0x0B, 0x07, 0x2A]
    )


def test_lower_calls_can_pass_virtual_register_window() -> None:
    artifact = lower_ir_to_cil_bytecode(
        _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)]),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(7)]),
            IrInstruction(IrOp.CALL, [IrLabel("callee")]),
            IrInstruction(IrOp.RET),
            IrInstruction(IrOp.LABEL, [IrLabel("callee")]),
            IrInstruction(IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(3)]),
            IrInstruction(IrOp.RET),
        ),
        CILBackendConfig(call_register_count=4),
        token_provider=FixedTokenProvider(),
    )

    entry, callee = artifact.methods

    assert entry.parameter_types == ()
    assert callee.parameter_types == ("int32", "int32", "int32", "int32")
    assert entry.body == bytes(
        [
            0x1B,
            0x0C,
            0x1D,
            0x0D,
            0x06,
            0x07,
            0x08,
            0x09,
            0x28,
            0x02,
            0x00,
            0x00,
            0x06,
            0x0B,
            0x07,
            0x2A,
        ]
    )
    assert callee.body[:8] == bytes([0x02, 0x0A, 0x03, 0x0B, 0x04, 0x0C, 0x05, 0x0D])


def test_lower_memory_and_syscall_helpers_use_injected_tokens() -> None:
    program = _program(
        IrInstruction(IrOp.LOAD_ADDR, [IrRegister(0), IrLabel("tape")]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(65)]),
        IrInstruction(IrOp.STORE_BYTE, [IrRegister(2), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.LOAD_BYTE, [IrRegister(4), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.SYSCALL, [IrImmediate(1)]),
        IrInstruction(IrOp.RET),
    )
    program.add_data(IrDataDecl("header", 4, 0))
    program.add_data(IrDataDecl("tape", 8, 0))

    artifact = lower_ir_to_cil_bytecode(program, token_provider=FixedTokenProvider())

    assert artifact.data_offsets == {"header": 0, "tape": 4}
    assert artifact.data_size == 12
    assert artifact.entry_method.body == bytes(
        [
            0x1A,
            0x0A,
            0x1F,
            0x41,
            0x0C,
            0x06,
            0x07,
            0x58,
            0x08,
            0x28,
            0x12,
            0x00,
            0x00,
            0x0A,
            0x06,
            0x07,
            0x58,
            0x28,
            0x11,
            0x00,
            0x00,
            0x0A,
            0x13,
            0x04,
            0x17,
            0x11,
            0x04,
            0x28,
            0x15,
            0x00,
            0x00,
            0x0A,
            0x13,
            0x04,
            0x07,
            0x2A,
        ]
    )


def test_default_token_provider_is_deterministic() -> None:
    provider = SequentialCILTokenProvider(("_start", "callee"))

    assert provider.method_token("_start") == 0x06000001
    assert provider.method_token("callee") == 0x06000002
    assert provider.helper_token(CILHelper.MEM_LOAD_BYTE) == 0x0A000001
    with pytest.raises(CILBackendError, match="Unknown CIL method"):
        provider.method_token("missing")


def test_pipeline_accepts_replacement_region_stage() -> None:
    def lower_fake_region(
        _program: IrProgram,
        _config: CILBackendConfig,
        _plan: object,
        _region: object,
    ) -> CILMethodArtifact:
        return CILMethodArtifact("_start", b"\x2a", 1, ())

    artifact = CILLoweringPipeline(lower_region=lower_fake_region).lower(_program())

    assert artifact.entry_method.body == b"\x2a"


def test_validation_rejects_bad_program_shapes() -> None:
    with pytest.raises(CILBackendError, match="Entry label not found"):
        lower_ir_to_cil_bytecode(IrProgram(entry_label="missing"))

    with pytest.raises(CILBackendError, match="Missing callable labels"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.CALL, [IrLabel("missing")]),
        ))

    with pytest.raises(CILBackendError, match="Duplicate IR label"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LABEL, [IrLabel("_start")]),
        ))

    with pytest.raises(CILBackendError, match="does not exist"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.JUMP, [IrLabel("missing")]),
        ))

    with pytest.raises(CILBackendError, match="escapes callable"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.CALL, [IrLabel("callee")]),
            IrInstruction(IrOp.BRANCH_Z, [IrRegister(0), IrLabel("callee")]),
            IrInstruction(IrOp.LABEL, [IrLabel("callee")]),
            IrInstruction(IrOp.RET),
        ))

    with pytest.raises(CILBackendError, match="Unknown data label"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_ADDR, [IrRegister(0), IrLabel("missing")]),
            IrInstruction(IrOp.RET),
        ))


def test_validation_rejects_bad_operands_and_limits() -> None:
    with pytest.raises(CILBackendError, match="must be an IrRegister"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_IMM, [IrImmediate(0), IrImmediate(1)]),
        ))

    with pytest.raises(CILBackendError, match="must be an IrImmediate"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrRegister(1)]),
        ))

    with pytest.raises(CILBackendError, match="must be an IrLabel"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.JUMP, [IrRegister(0)]),
        ))

    with pytest.raises(CILBackendError, match="must not be empty"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.JUMP, [IrLabel("")]),
        ))

    with pytest.raises(CILBackendError, match="non-negative"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(-1), IrImmediate(1)]),
        ))

    with pytest.raises(CILBackendError, match="outside CLR local slot range"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(65536), IrImmediate(1)]),
        ))

    with pytest.raises(CILBackendError, match="int32 range"):
        lower_ir_to_cil_bytecode(_program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(2**31)]),
        ))

    program = _program(IrInstruction(IrOp.RET))
    program.add_data(IrDataDecl("too_big", 2, 0))
    with pytest.raises(CILBackendError, match="Total static data exceeds"):
        lower_ir_to_cil_bytecode(program, CILBackendConfig(max_static_data_bytes=1))

    program = _program(IrInstruction(IrOp.RET))
    program.add_data(IrDataDecl("negative", -1, 0))
    with pytest.raises(CILBackendError, match="Negative data size"):
        lower_ir_to_cil_bytecode(program)

    program = _program(IrInstruction(IrOp.RET))
    program.add_data(IrDataDecl("same", 1, 0))
    program.add_data(IrDataDecl("same", 1, 0))
    with pytest.raises(CILBackendError, match="Duplicate data label"):
        lower_ir_to_cil_bytecode(program)

    program = _program(IrInstruction(IrOp.RET))
    program.add_data(IrDataDecl("bad_init", 1, 256))
    with pytest.raises(CILBackendError, match="outside uint8"):
        lower_ir_to_cil_bytecode(program)


def test_config_and_artifact_validation_paths() -> None:
    with pytest.raises(CILBackendError, match="syscall_arg_reg"):
        lower_ir_to_cil_bytecode(_program(), CILBackendConfig(syscall_arg_reg=-1))
    with pytest.raises(CILBackendError, match="max_static_data_bytes"):
        lower_ir_to_cil_bytecode(
            _program(),
            CILBackendConfig(max_static_data_bytes=-1),
        )
    with pytest.raises(CILBackendError, match="method_max_stack"):
        lower_ir_to_cil_bytecode(_program(), CILBackendConfig(method_max_stack=0))

    artifact = CILProgramArtifact(
        entry_label="missing",
        methods=(),
        data_offsets={},
        data_size=0,
        helper_specs=(),
        token_provider=SequentialCILTokenProvider(()),
    )
    with pytest.raises(CILBackendError, match="Entry method not found"):
        _ = artifact.entry_method


def test_lower_remaining_operations() -> None:
    artifact = lower_ir_to_cil_bytecode(
        _program(
            IrInstruction(IrOp.COMMENT, []),
            IrInstruction(IrOp.NOP),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(10)]),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(2)]),
            IrInstruction(IrOp.SUB, [IrRegister(2), IrRegister(0), IrRegister(1)]),
            IrInstruction(IrOp.MUL, [IrRegister(3), IrRegister(2), IrRegister(1)]),
            IrInstruction(IrOp.DIV, [IrRegister(4), IrRegister(3), IrRegister(1)]),
            IrInstruction(IrOp.CMP_EQ, [IrRegister(5), IrRegister(4), IrRegister(0)]),
            IrInstruction(IrOp.CMP_GT, [IrRegister(6), IrRegister(4), IrRegister(1)]),
            IrInstruction(IrOp.BRANCH_Z, [IrRegister(6), IrLabel("done")]),
            IrInstruction(IrOp.JUMP, [IrLabel("done")]),
            IrInstruction(IrOp.LABEL, [IrLabel("done")]),
            IrInstruction(IrOp.HALT),
        )
    )

    assert artifact.entry_method.body == bytes(
        [
            0x00,
            0x1F,
            0x0A,
            0x0A,
            0x18,
            0x0B,
            0x06,
            0x07,
            0x59,
            0x0C,
            0x08,
            0x07,
            0x5A,
            0x0D,
            0x09,
            0x07,
            0x5B,
            0x13,
            0x04,
            0x11,
            0x04,
            0x06,
            0xFE,
            0x01,
            0x13,
            0x05,
            0x11,
            0x04,
            0x07,
            0xFE,
            0x02,
            0x13,
            0x06,
            0x11,
            0x06,
            0x2C,
            0x02,
            0x2B,
            0x00,
            0x07,
            0x2A,
        ]
    )


def test_lower_word_helpers() -> None:
    artifact = lower_ir_to_cil_bytecode(
        _program(
            IrInstruction(
                IrOp.STORE_WORD,
                [IrRegister(2), IrRegister(0), IrRegister(1)],
            ),
            IrInstruction(
                IrOp.LOAD_WORD,
                [IrRegister(3), IrRegister(0), IrRegister(1)],
            ),
            IrInstruction(IrOp.RET),
        ),
        token_provider=FixedTokenProvider(),
    )

    assert bytes([0x14, 0x00, 0x00, 0x0A]) in artifact.entry_method.body
    assert bytes([0x13, 0x00, 0x00, 0x0A]) in artifact.entry_method.body


def test_protocol_import_is_runtime_safe() -> None:
    provider: CILTokenProvider = FixedTokenProvider()

    assert provider.method_token("_start") == 0x06000001
    assert provider.helper_token(CILHelper.SYSCALL) == 0x0A000015


def test_lower_or_register() -> None:
    """IrOp.OR emits the CIL ``or`` byte (0x60) between two register loads.

    The sequence for ``OR v2, v0, v1`` is:
      ldloc.0  (0x06)  — push v0
      ldloc.1  (0x07)  — push v1
      or       (0x60)  — bitwise OR
      stloc.2  (0x0C)  — pop into v2
    """
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0b1010)]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0b0101)]),
        IrInstruction(IrOp.OR, [IrRegister(2), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    # ``or`` opcode must appear in the output
    assert 0x60 in body
    # The OR pattern: ldloc.0, ldloc.1, or, stloc.2
    assert bytes([0x06, 0x07, 0x60, 0x0C]) in body


def test_lower_or_imm() -> None:
    """IrOp.OR_IMM emits ``or`` with the immediate pushed by ldc.i4."""
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0b1010)]),
        IrInstruction(IrOp.OR_IMM, [IrRegister(1), IrRegister(0), IrImmediate(0b0101)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    assert 0x60 in body


def test_lower_xor_register() -> None:
    """IrOp.XOR emits the CIL ``xor`` byte (0x61).

    The sequence for ``XOR v2, v0, v1``:
      ldloc.0  (0x06)
      ldloc.1  (0x07)
      xor      (0x61)
      stloc.2  (0x0C)
    """
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0b1111)]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0b1010)]),
        IrInstruction(IrOp.XOR, [IrRegister(2), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    assert 0x61 in body
    assert bytes([0x06, 0x07, 0x61, 0x0C]) in body


def test_lower_xor_imm() -> None:
    """IrOp.XOR_IMM emits ``xor`` with an immediate operand."""
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0xFF)]),
        IrInstruction(IrOp.XOR_IMM, [IrRegister(1), IrRegister(0), IrImmediate(0x0F)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    assert 0x61 in body


def test_lower_not() -> None:
    """IrOp.NOT lowers to ``ldc.i4.m1`` + ``xor`` (flip all 32 bits).

    NOT x = x XOR 0xFFFF_FFFF = x XOR (-1 as signed int32).

    CIL has no single NOT instruction.  The canonical idiom is:
      ldloc.N          — push x
      ldc.i4.m1  (0x15) — push -1 (= 0xFFFF_FFFF bit-pattern)
      xor        (0x61) — bitwise XOR → all bits flipped
      stloc.M          — store result

    For the entry program with v0 as src and v1 as dst, after LOAD_IMM
    (into loc 0), the NOT body is:
      ldloc.0  (0x06)
      ldc.i4.m1 (0x15)
      xor      (0x61)
      stloc.1  (0x0B)
    """
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0x00FF_00FF)]),
        IrInstruction(IrOp.NOT, [IrRegister(1), IrRegister(0)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    # ldc.i4.m1 (0x15) must precede xor (0x61) to form the NOT idiom
    assert bytes([0x06, 0x15, 0x61, 0x0B]) in body


def test_lower_not_double_inverts() -> None:
    """Applying NOT twice returns the original value (double-complement law).

    This is a semantic round-trip test: load 42, NOT it into v1, NOT v1 into
    v2.  We verify the body compiles without error and contains two xor (0x61)
    opcodes — one for each NOT — plus two ldc.i4.m1 (0x15) pushes.
    """
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(42)]),
        IrInstruction(IrOp.NOT, [IrRegister(1), IrRegister(0)]),
        IrInstruction(IrOp.NOT, [IrRegister(2), IrRegister(1)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    xor_count = body.count(0x61)
    m1_count = body.count(0x15)
    assert xor_count == 2
    assert m1_count == 2


def test_lower_bitwise_ops_mixed() -> None:
    """AND, OR, XOR, NOT can coexist in a single method body without collision.

    Encodes the truth-table identity:  (a & b) | (a ^ b) == a | b.

    Registers:
      v0 = 0b1100  (12)
      v1 = 0b1010  (10)
      v2 = v0 & v1  →  0b1000  (8)
      v3 = v0 ^ v1  →  0b0110  (6)
      v4 = v2 | v3  →  0b1110  (14)  expected == v0 | v1
    """
    artifact = lower_ir_to_cil_bytecode(_program(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0b1100)]),
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0b1010)]),
        IrInstruction(IrOp.AND, [IrRegister(2), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.XOR, [IrRegister(3), IrRegister(0), IrRegister(1)]),
        IrInstruction(IrOp.OR, [IrRegister(4), IrRegister(2), IrRegister(3)]),
        IrInstruction(IrOp.RET),
    ))

    body = artifact.entry_method.body
    # All three bitwise opcodes must appear in the body
    assert 0x5F in body  # and
    assert 0x60 in body  # or
    assert 0x61 in body  # xor


class TestValidateForClr:
    """Unit tests for the validate_for_clr() pre-flight validator.

    The validator catches CLR-incompatible programs *before* any bytecode is
    generated.  Three categories are checked:

    1. Opcode support — every IrOp must appear in ``_CLR_SUPPORTED_OPCODES``.
    2. Constant range — LOAD_IMM / ADD_IMM immediates must fit in int32.
    3. SYSCALL number — only 1 (write byte), 2 (read byte), 10 (exit) are wired.

    Oct's I/O intrinsics map to SYSCALL 40+PORT (output) and SYSCALL 20+PORT
    (input) — both ranges are absent from the CLR host and are caught here.
    """

    # ── Passing cases ──────────────────────────────────────────────────────

    def test_pure_arithmetic_program_passes(self) -> None:
        """A program using only arithmetic opcodes and SYSCALL 1 passes."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(3)]),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(7)]),
            IrInstruction(IrOp.ADD, [IrRegister(4), IrRegister(0), IrRegister(1)]),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(4)]),
            IrInstruction(IrOp.HALT, []),
        )
        assert validate_for_clr(program) == []

    def test_syscall_1_passes(self) -> None:
        """SYSCALL 1 (write byte) is wired in the CLR host — accepted."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(65)]),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(0)]),
            IrInstruction(IrOp.HALT, []),
        )
        assert validate_for_clr(program) == []

    def test_syscall_2_passes(self) -> None:
        """SYSCALL 2 (read byte) is wired in the CLR host — accepted."""
        program = _program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(2), IrRegister(0)]),
            IrInstruction(IrOp.HALT, []),
        )
        assert validate_for_clr(program) == []

    def test_syscall_10_passes(self) -> None:
        """SYSCALL 10 (process exit) is wired in the CLR host — accepted."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(0)]),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(10), IrRegister(0)]),
            IrInstruction(IrOp.HALT, []),
        )
        assert validate_for_clr(program) == []

    def test_int32_boundary_immediates_pass(self) -> None:
        """LOAD_IMM immediates at the int32 boundary are accepted."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(-(2**31))]),
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(2**31 - 1)]),
            IrInstruction(IrOp.RET, []),
        )
        assert validate_for_clr(program) == []

    # ── SYSCALL rejection ──────────────────────────────────────────────────

    def test_rejects_oct_out_syscall_57(self) -> None:
        """Oct's out(17, val) → SYSCALL 57 is rejected.

        out(PORT, val) in Oct lowers to SYSCALL 40+PORT.  Port 17 gives
        SYSCALL 57.  The CLR host only knows about SYSCALLs 1, 2, and 10.
        """
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(10)]),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(57), IrRegister(4)]),
            IrInstruction(IrOp.HALT, []),
        )
        errors = validate_for_clr(program)
        assert errors, "Expected validation to reject SYSCALL 57"
        assert any("57" in e or "unsupported" in e.lower() for e in errors)

    def test_rejects_oct_in_syscall_23(self) -> None:
        """Oct's in(3) → SYSCALL 23 is rejected.

        in(PORT) in Oct lowers to SYSCALL 20+PORT.  Port 3 gives SYSCALL 23.
        """
        program = _program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(23), IrRegister(4)]),
            IrInstruction(IrOp.HALT, []),
        )
        errors = validate_for_clr(program)
        assert errors, "Expected validation to reject SYSCALL 23"
        assert any("23" in e or "unsupported" in e.lower() for e in errors)

    def test_rejects_syscall_0(self) -> None:
        """SYSCALL 0 is not wired in the CLR host."""
        program = _program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(0), IrRegister(0)]),
            IrInstruction(IrOp.HALT, []),
        )
        errors = validate_for_clr(program)
        assert errors
        assert any("0" in e for e in errors)

    # ── Constant range rejection ───────────────────────────────────────────

    def test_rejects_load_imm_above_int32_max(self) -> None:
        """LOAD_IMM with a value above int32 max is rejected."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(2**31)]),
            IrInstruction(IrOp.RET, []),
        )
        errors = validate_for_clr(program)
        assert errors
        assert any("int32" in e.lower() or "range" in e.lower() for e in errors)

    def test_rejects_load_imm_below_int32_min(self) -> None:
        """LOAD_IMM with a value below int32 min is rejected."""
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(-(2**31) - 1)]),
            IrInstruction(IrOp.RET, []),
        )
        errors = validate_for_clr(program)
        assert errors
        assert any("int32" in e.lower() or "range" in e.lower() for e in errors)

    # ── Integration with lower_ir_to_cil_bytecode ─────────────────────────

    def test_lower_raises_on_oct_syscall(self) -> None:
        """lower_ir_to_cil_bytecode raises CILBackendError on SYSCALL 57.

        The pre-flight validator is called inside lower_ir_to_cil_bytecode().
        Callers that use the public API get a compile-time CILBackendError
        instead of a runtime CLRVMError buried inside the CLR VM.
        """
        program = _program(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(10)]),
            IrInstruction(IrOp.SYSCALL, [IrImmediate(57), IrRegister(4)]),
            IrInstruction(IrOp.HALT, []),
        )
        with pytest.raises(CILBackendError, match=r"57|pre-flight"):
            lower_ir_to_cil_bytecode(program)

    def test_lower_raises_with_multiple_errors(self) -> None:
        """lower_ir_to_cil_bytecode error message includes count when > 1 error."""
        program = _program(
            IrInstruction(IrOp.SYSCALL, [IrImmediate(57), IrRegister(4)]),  # bad
            IrInstruction(IrOp.SYSCALL, [IrImmediate(23), IrRegister(4)]),  # bad
            IrInstruction(IrOp.HALT, []),
        )
        with pytest.raises(CILBackendError, match=r"2 errors"):
            lower_ir_to_cil_bytecode(program)


def _program(*instructions: IrInstruction) -> IrProgram:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    for instruction in instructions:
        program.add_instruction(instruction)
    return program


# ─────────────────────────────────────────────────────────────────────────
# CLR02 Phase 2c — closure lowering (structural)
# ─────────────────────────────────────────────────────────────────────────


class TestClosureLoweringStructure:
    """Tests for MAKE_CLOSURE / APPLY_CLOSURE lowering shape.

    These verify the auto-generated TypeArtifacts and bytecode
    layout — *not* runtime semantics, which require typed-register
    work landing in a follow-up phase (managed-pointer locals can't
    fit in the current int32-uniform register convention; see the
    xfail real-dotnet test in cli-assembly-writer).
    """

    def _build_make_adder_program(self) -> IrProgram:
        program = IrProgram(entry_label="Main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.ADD,
                [IrRegister(1), IrRegister(2), IrRegister(3)],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))

        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(1),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))

        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)])
        )
        program.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [
                    IrRegister(1),
                    IrRegister(1),
                    IrImmediate(1),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        return program

    def _config(self) -> CILBackendConfig:
        return CILBackendConfig(
            call_register_count=None,
            closure_free_var_counts={"_lambda_0": 1},
        )

    def test_iclosure_interface_emitted_when_any_closure_present(self) -> None:
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        names = {(t.namespace, t.name) for t in artifact.extra_types}
        assert ("CodingAdventures", "IClosure") in names
        assert ("CodingAdventures", "Closure__lambda_0") in names

    def test_no_extra_types_when_no_closures(self) -> None:
        program = IrProgram(entry_label="Main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)])
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        artifact = lower_ir_to_cil_bytecode(program)
        assert artifact.extra_types == ()

    def test_iclosure_apply_is_abstract(self) -> None:
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        iclosure = next(
            t for t in artifact.extra_types if t.name == "IClosure"
        )
        assert iclosure.is_interface is True
        assert iclosure.extends is None
        assert len(iclosure.methods) == 1
        apply = iclosure.methods[0]
        assert apply.name == "Apply"
        assert apply.is_abstract is True
        assert apply.is_instance is True
        # TW03 Phase 3 follow-up: Apply now returns ``object`` (was
        # ``int32``) so closure-returning closures can carry their
        # inner closure ref through the call boundary.
        # Multi-arity follow-up: Apply takes ``int32[]`` (was
        # ``int32``) so closures of any arity share a single uniform
        # call site.  The body's prologue extracts each arg via
        # ``ldelem.i4``.
        assert apply.return_type == "object"
        assert apply.parameter_types == ("int32[]",)

    def test_closure_class_has_field_per_capture(self) -> None:
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        closure = next(
            t for t in artifact.extra_types if t.name == "Closure__lambda_0"
        )
        assert len(closure.fields) == 1
        assert closure.fields[0].name == "capt0"
        assert closure.fields[0].type == "int32"
        assert closure.implements == ("CodingAdventures.IClosure",)

    def test_closure_class_has_ctor_and_apply(self) -> None:
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        closure = next(
            t for t in artifact.extra_types if t.name == "Closure__lambda_0"
        )
        method_names = {m.name for m in closure.methods}
        assert method_names == {".ctor", "Apply"}
        ctor = next(m for m in closure.methods if m.name == ".ctor")
        assert ctor.is_special_name is True
        assert ctor.is_instance is True
        assert ctor.return_type == "void"
        assert ctor.parameter_types == ("int32",)
        apply = next(m for m in closure.methods if m.name == "Apply")
        assert apply.is_instance is True
        assert apply.is_abstract is False
        # TW03 Phase 3 follow-up: Apply now returns ``object`` (was
        # ``int32``) so closure-returning closures can carry their
        # inner closure ref through the call boundary.
        # Multi-arity follow-up: Apply takes ``int32[]`` (was
        # ``int32``).
        assert apply.return_type == "object"
        assert apply.parameter_types == ("int32[]",)

    def test_lambda_region_omitted_from_main_methods(self) -> None:
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        names = {m.name for m in artifact.methods}
        assert "_lambda_0" not in names
        assert "Main" in names
        assert "make_adder" in names

    def test_make_closure_emits_newobj_token(self) -> None:
        from cil_bytecode_builder import CILOpcode
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        make_adder = next(m for m in artifact.methods if m.name == "make_adder")
        # newobj opcode = 0x73; appears at least once.
        assert 0x73 in make_adder.body, "MAKE_CLOSURE should emit newobj"
        # No callvirt — that's APPLY_CLOSURE's opcode.
        assert int(CILOpcode.CALLVIRT) not in make_adder.body

    def test_apply_closure_emits_callvirt(self) -> None:
        from cil_bytecode_builder import CILOpcode
        artifact = lower_ir_to_cil_bytecode(
            self._build_make_adder_program(), self._config()
        )
        main = next(m for m in artifact.methods if m.name == "Main")
        assert int(CILOpcode.CALLVIRT) in main.body, (
            "APPLY_CLOSURE should emit callvirt"
        )

    def test_apply_closure_multi_arity_lowers_without_error(self) -> None:
        """Multi-arity follow-up: APPLY_CLOSURE with arity > 1 now
        lowers successfully (the arity-1 hard limit was removed when
        IClosure.Apply was widened to take ``int32[]``).  The call
        site builds an int32[] via ``newarr [System.Int32]`` and
        populates each slot with ``dup; ldc.i4 i; ldloc; stelem.i4``.
        """
        program = IrProgram(entry_label="Main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")])
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(0),
                ],
            )
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [
                    IrRegister(1),
                    IrRegister(1),
                    IrImmediate(2),  # arity 2 — previously rejected
                    IrRegister(2),
                    IrRegister(3),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        artifact = lower_ir_to_cil_bytecode(
            program,
            CILBackendConfig(
                closure_free_var_counts={"_lambda_0": 0},
                closure_explicit_arities={"_lambda_0": 2},
            ),
        )
        main = next(m for m in artifact.methods if m.name == "Main")
        # newarr opcode = 0x8D; stelem.i4 = 0x9E.  Both must appear
        # in the APPLY_CLOSURE call-site sequence.
        assert 0x8D in main.body, (
            "APPLY_CLOSURE should emit newarr to build int32[]"
        )
        assert 0x9E in main.body, (
            "APPLY_CLOSURE should emit stelem.i4 to populate args"
        )

    def test_closure_free_var_counts_unknown_region_rejected(self) -> None:
        program = IrProgram(entry_label="Main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(IrInstruction(IrOp.RET, []))
        with pytest.raises(CILBackendError, match=r"don't exist"):
            lower_ir_to_cil_bytecode(
                program,
                CILBackendConfig(closure_free_var_counts={"_ghost": 1}),
            )

    def test_make_closure_capture_count_mismatch_rejected(self) -> None:
        program = IrProgram(entry_label="Main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")])
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(2),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        with pytest.raises(CILBackendError, match=r"num_captured=2"):
            lower_ir_to_cil_bytecode(
                program,
                CILBackendConfig(closure_free_var_counts={"_lambda_0": 2}),
            )


# ─────────────────────────────────────────────────────────────────────────
# CLR02 Phase 2c.5 — typed register pool analysis
# ─────────────────────────────────────────────────────────────────────────


class TestTypedRegisterPool:
    """Tests for the per-region register-typing pass that
    Phase 2c.5 adds.  These verify the analysis classifies
    registers correctly so the lowerer emits the right
    int32-vs-object slot at each program point.
    """

    def _make_adder_program(self) -> IrProgram:
        """The headline closure fixture — make-adder + Main applying
        the resulting closure."""
        program = IrProgram(entry_label="Main")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("_lambda_0")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(3)],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.MAKE_CLOSURE,
                [
                    IrRegister(1),
                    IrLabel("_lambda_0"),
                    IrImmediate(1),
                    IrRegister(2),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)])
        )
        program.add_instruction(
            IrInstruction(IrOp.CALL, [IrLabel("make_adder")])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.ADD_IMM, [IrRegister(10), IrRegister(1), IrImmediate(0)],
            )
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(11), IrImmediate(35)])
        )
        program.add_instruction(
            IrInstruction(
                IrOp.APPLY_CLOSURE,
                [
                    IrRegister(1),
                    IrRegister(10),
                    IrImmediate(1),
                    IrRegister(11),
                ],
            )
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        return program

    def _config(self) -> CILBackendConfig:
        return CILBackendConfig(
            call_register_count=None,
            closure_free_var_counts={"_lambda_0": 1},
        )

    def test_make_adder_returns_object(self) -> None:
        """A function whose r1 is the dst of MAKE_CLOSURE returns
        ``object``."""
        artifact = lower_ir_to_cil_bytecode(
            self._make_adder_program(), self._config()
        )
        make_adder = next(m for m in artifact.methods if m.name == "make_adder")
        assert make_adder.return_type == "object"

    def test_apply_returning_function_stays_int32(self) -> None:
        """A function whose r1 receives only an APPLY_CLOSURE result
        stays int32 — apply returns int per the IClosure contract."""
        artifact = lower_ir_to_cil_bytecode(
            self._make_adder_program(), self._config()
        )
        main = next(m for m in artifact.methods if m.name == "Main")
        # Main's r1 ends up holding APPLY_CLOSURE's int return,
        # so Main returns int32.
        assert main.return_type == "int32"

    def test_main_method_has_object_locals_for_closure_flow(self) -> None:
        """Main's local table has parallel object locals for the
        registers that ever hold a closure ref (r1 and r10)."""
        artifact = lower_ir_to_cil_bytecode(
            self._make_adder_program(), self._config()
        )
        main = next(m for m in artifact.methods if m.name == "Main")
        # Should have at least 2 object slots appended after the
        # int32 slots — for r1 and r10.
        object_local_count = sum(1 for t in main.local_types if t == "object")
        assert object_local_count >= 2

    def test_make_adder_has_object_local_for_r1(self) -> None:
        """make_adder's r1 holds the closure ref produced by
        MAKE_CLOSURE — it needs an object local."""
        artifact = lower_ir_to_cil_bytecode(
            self._make_adder_program(), self._config()
        )
        make_adder = next(m for m in artifact.methods if m.name == "make_adder")
        object_local_count = sum(
            1 for t in make_adder.local_types if t == "object"
        )
        assert object_local_count >= 1

    def test_pure_int_function_has_no_object_locals(self) -> None:
        """A function with no MAKE_CLOSURE / closure-returning CALLs
        / object-typed MOVs has zero object locals (backward-compat
        with the pre-Phase-2c.5 emission shape)."""
        program = IrProgram(entry_label="Main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)])
        )
        program.add_instruction(IrInstruction(IrOp.RET, []))
        artifact = lower_ir_to_cil_bytecode(program)
        main = next(m for m in artifact.methods if m.name == "Main")
        assert all(t == "int32" for t in main.local_types)
        assert main.return_type == "int32"

    def test_function_return_types_map_populated(self) -> None:
        """The lowering plan exposes a function-return-types map so
        callers can introspect the analysis."""
        # Use the public lowering pipeline to get at the plan.
        from ir_to_cil_bytecode.backend import (
            CILLoweringPipeline,
            _analyze_program,
        )
        plan = _analyze_program(self._make_adder_program(), self._config())
        assert plan.function_return_types["make_adder"] == "object"
        assert plan.function_return_types["Main"] == "int32"
        # Closure regions always return int32 per the IClosure contract.
        assert plan.function_return_types["_lambda_0"] == "int32"


# ─────────────────────────────────────────────────────────────────────────
# TW03 Phase 3c — heap-primitive lowering (cons / symbol / nil)
# ─────────────────────────────────────────────────────────────────────────
#
# Phase 3c v1 ships **structural-only** lowering: bytecode shape is correct
# (right opcode bytes, right token-provider interaction), the three new
# extra TypeArtifacts (Cons / Symbol / Nil) get auto-included, but a
# follow-up (Phase 3c.5) wires in the cli-assembly-writer-side intern
# tables for symbol names and the singleton Nil instance.  Until then,
# IS_NULL still works correctly via ``isinst Nil`` (every Nil instance
# qualifies as null) but two MAKE_SYMBOL calls with the same name yield
# different Symbol instances (semantically wrong; bytecode-shape correct).


class TestHeapExtraTypes:
    """Tests for the auto-included Cons / Symbol / Nil TypeDefs."""

    def _heap_program(self, instructions: list[IrInstruction]) -> IrProgram:
        program = IrProgram(entry_label="Main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        for ins in instructions:
            program.add_instruction(ins)
        program.add_instruction(IrInstruction(IrOp.RET, []))
        return program

    def test_extra_types_include_cons_symbol_nil_when_heap_op_present(
        self,
    ) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        names = {(t.namespace, t.name) for t in artifact.extra_types}
        assert ("CodingAdventures", "Cons") in names
        assert ("CodingAdventures", "Symbol") in names
        assert ("CodingAdventures", "Nil") in names

    def test_no_heap_extra_types_when_no_heap_op(self) -> None:
        program = self._heap_program([])
        artifact = lower_ir_to_cil_bytecode(program)
        names = {(t.namespace, t.name) for t in artifact.extra_types}
        assert ("CodingAdventures", "Cons") not in names
        assert ("CodingAdventures", "Symbol") not in names
        assert ("CodingAdventures", "Nil") not in names

    def test_cons_typedef_has_object_head_and_tail(self) -> None:
        # Heterogeneous-cons follow-up: head is now object-typed
        # (was int32) so cons cells can hold any Twig value (boxed
        # Int32, Symbol, Nil, Cons, closure ref).
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        cons = next(t for t in artifact.extra_types if t.name == "Cons")
        field_types = {f.name: f.type for f in cons.fields}
        assert field_types == {"head": "object", "tail": "object"}

    def test_symbol_typedef_has_string_name(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        sym = next(t for t in artifact.extra_types if t.name == "Symbol")
        field_types = {f.name: f.type for f in sym.fields}
        assert field_types == {"name": "string"}

    def test_nil_typedef_has_no_fields(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        nil = next(t for t in artifact.extra_types if t.name == "Nil")
        assert nil.fields == ()
        # Single ctor.
        ctor_names = [m.name for m in nil.methods]
        assert ctor_names == [".ctor"]


class TestHeapOpLowering:
    """Each heap op should emit its identifying CIL opcode bytes."""

    def _heap_program(self, instructions: list[IrInstruction]) -> IrProgram:
        program = IrProgram(entry_label="Main")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("Main")]))
        for ins in instructions:
            program.add_instruction(ins)
        program.add_instruction(IrInstruction(IrOp.RET, []))
        return program

    def _main_body(self, artifact: CILProgramArtifact) -> bytes:
        return next(m.body for m in artifact.methods if m.name == "Main")

    def test_make_cons_emits_newobj(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)]),
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(3)]),
            IrInstruction(
                IrOp.MAKE_CONS,
                [IrRegister(4), IrRegister(2), IrRegister(3)],
            ),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        body = self._main_body(artifact)
        # 0x73 = newobj.
        assert b"\x73" in body

    def test_car_emits_castclass_and_ldfld(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.CAR, [IrRegister(3), IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        body = self._main_body(artifact)
        # 0x74 = castclass; 0x7B = ldfld.
        assert b"\x74" in body
        assert b"\x7B" in body

    def test_cdr_emits_castclass_and_ldfld(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.CDR, [IrRegister(3), IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        body = self._main_body(artifact)
        assert b"\x74" in body
        assert b"\x7B" in body

    def test_is_null_emits_isinst_ldnull_cgt_un(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.IS_NULL, [IrRegister(3), IrRegister(2)]),
        ])
        artifact = lower_ir_to_cil_bytecode(program)
        body = self._main_body(artifact)
        # 0x75 = isinst; 0x14 = ldnull; 0xFE 0x03 = cgt.un (two bytes).
        assert b"\x75" in body
        assert b"\x14" in body
        assert b"\xfe\x03" in body

    def test_is_pair_emits_isinst(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.IS_PAIR, [IrRegister(3), IrRegister(2)]),
        ])
        body = self._main_body(lower_ir_to_cil_bytecode(program))
        assert b"\x75" in body

    def test_is_symbol_emits_isinst(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.IS_SYMBOL, [IrRegister(3), IrRegister(2)]),
        ])
        body = self._main_body(lower_ir_to_cil_bytecode(program))
        assert b"\x75" in body

    def test_make_symbol_emits_newobj_and_ldnull_placeholder(self) -> None:
        """Phase 3c v1: MAKE_SYMBOL emits ``ldnull`` for the name
        argument (proper ldstr UserString wiring lands in 3c.5)."""
        program = self._heap_program([
            IrInstruction(
                IrOp.MAKE_SYMBOL, [IrRegister(2), IrLabel("foo")],
            ),
        ])
        body = self._main_body(lower_ir_to_cil_bytecode(program))
        # 0x14 ldnull, 0x73 newobj.
        assert b"\x14" in body
        assert b"\x73" in body

    def test_load_nil_emits_newobj(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
        ])
        body = self._main_body(lower_ir_to_cil_bytecode(program))
        # 0x73 = newobj (Nil.ctor()).
        assert b"\x73" in body

    def test_make_cons_arity_validation(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.MAKE_CONS, [IrRegister(2)]),  # missing 2 ops
        ])
        with pytest.raises(CILBackendError, match="MAKE_CONS expects"):
            lower_ir_to_cil_bytecode(program)

    def test_load_nil_arity_validation(self) -> None:
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2), IrRegister(3)]),
        ])
        with pytest.raises(CILBackendError, match="LOAD_NIL expects"):
            lower_ir_to_cil_bytecode(program)

    def test_validate_for_clr_accepts_heap_opcodes(self) -> None:
        """validate_for_clr no longer rejects the 8 heap opcodes."""
        program = self._heap_program([
            IrInstruction(IrOp.LOAD_NIL, [IrRegister(2)]),
            IrInstruction(IrOp.MAKE_CONS, [
                IrRegister(3), IrRegister(2), IrRegister(2),
            ]),
            IrInstruction(IrOp.CAR, [IrRegister(4), IrRegister(3)]),
        ])
        # Empty errors list → no rejections.
        assert validate_for_clr(program) == []


class TestHeapTokenLayout:
    """Lock down the deterministic heap-token layout so the writer
    can mirror it.  Tokens are computed relative to the closure
    counts so future Phase 3c.5 work doesn't shift them."""

    def test_heap_tokens_unavailable_without_include_heap_types(self) -> None:
        provider = SequentialCILTokenProvider(
            ("Main",),
        )
        with pytest.raises(CILBackendError, match="cons_ctor"):
            provider.heap_cons_ctor_token()

    def test_heap_tokens_layout_no_closures(self) -> None:
        """With no closures and one main method, the heap method
        tokens start at 0x06000002."""
        provider = SequentialCILTokenProvider(
            ("Main",), include_heap_types=True,
        )
        # M=1 main methods, no closures.  Heap method base = 0x06000002.
        assert provider.heap_cons_ctor_token() == 0x06000002
        assert provider.heap_symbol_ctor_token() == 0x06000003
        assert provider.heap_nil_ctor_token() == 0x06000004
        # Field tokens start at 0x04000001.
        assert provider.heap_cons_head_token() == 0x04000001
        assert provider.heap_cons_tail_token() == 0x04000002
        assert provider.heap_symbol_name_token() == 0x04000003
        # TypeDef tokens: row 1=<Module>, row 2=Main, row 3=Cons.
        assert provider.heap_cons_typedef_token() == 0x02000003
        assert provider.heap_symbol_typedef_token() == 0x02000004
        assert provider.heap_nil_typedef_token() == 0x02000005

    def test_heap_tokens_layout_with_closures(self) -> None:
        """With 1 main method + 2 closures, the heap method tokens
        follow the closure rows (1 + 2*2 = 5 closure method rows)."""
        provider = SequentialCILTokenProvider(
            ("Main",),
            closure_names=("_l0", "_l1"),
            closure_free_var_counts={"_l0": 1, "_l1": 2},
            include_heap_types=True,
        )
        # M=1, closure_method_rows = 1 + 2*2 = 5.  Heap base = 0x06000002 + 5.
        assert provider.heap_cons_ctor_token() == 0x06000007
        # field_row after closures: 1 + 2 = 3 capture fields.  Heap fields
        # start at 0x04000001 + 3 = 0x04000004.
        assert provider.heap_cons_head_token() == 0x04000004
        # TypeDef rows: <Module>=1, Main=2, IClosure=3, Closure__l0=4,
        # Closure__l1=5, Cons=6.
        assert provider.heap_cons_typedef_token() == 0x02000006
