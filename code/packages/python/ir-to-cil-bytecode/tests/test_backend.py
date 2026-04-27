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
