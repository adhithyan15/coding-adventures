"""Unit tests for the basic IR-op → BEAM-op lowering.

These tests don't need ``erl`` — they verify the lowering produces
a structurally-correct ``BEAMModule`` and that ``encode_beam`` can
round-trip the result through ``beam-bytes-decoder``.
"""

from __future__ import annotations

import pytest
from beam_bytecode_encoder import BEAMTag, encode_beam
from beam_bytes_decoder import decode_beam_module
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)

from ir_to_beam import (
    BEAMBackendConfig,
    BEAMBackendError,
    lower_ir_to_beam,
)


def _label(name: str) -> IrLabel:
    return IrLabel(name=name)


def _reg(i: int) -> IrRegister:
    return IrRegister(index=i)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _build_identity_program() -> IrProgram:
    """``identity()`` returns the constant 42.

    TW03 Phase 1 convention: the result lands in ``r1``
    (``_REG_HALT_RESULT``) before ``RET`` because ir-to-beam's
    RET lowering reads y1 → x0 → return.
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="identity")
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [_label("identity")], id=-1)
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(42)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    return program


def _build_add_program() -> IrProgram:
    """``add() -> 17 + 25`` (= 42).  Result in r1."""
    gen = IDGenerator()
    program = IrProgram(entry_label="add")
    program.add_instruction(IrInstruction(IrOp.LABEL, [_label("add")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(17)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(25)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD, [_reg(1), _reg(2), _reg(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    return program


# ---------------------------------------------------------------------------
# Module shape
# ---------------------------------------------------------------------------


class TestModuleShape:
    def test_module_name_lands_at_atom_one(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        assert m.name == "answer"
        assert m.atoms[0] == "answer"

    def test_user_function_is_exported(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # We expect three exports: identity/0, module_info/0, module_info/1.
        assert len(m.exports) == 3
        names = {m.atoms[e.function_atom_index - 1] for e in m.exports}
        assert names == {"identity", "module_info"}

    def test_module_info_imports_synthesised(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # erlang:get_module_info/1 and /2 must both be in the import table.
        triples = {
            (
                m.atoms[row.module_atom_index - 1],
                m.atoms[row.function_atom_index - 1],
                row.arity,
            )
            for row in m.imports
        }
        assert ("erlang", "get_module_info", 1) in triples
        assert ("erlang", "get_module_info", 2) in triples


# ---------------------------------------------------------------------------
# Encoder round-trip
# ---------------------------------------------------------------------------


class TestEncoderRoundTrip:
    def test_identity_program_encodes_and_decodes(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        decoded = decode_beam_module(encode_beam(m))
        assert decoded.module_name == "answer"
        # The decoder prepends None at index 0.
        assert decoded.atoms[1] == "answer"
        # All exports landed.
        export_names = {e.function for e in decoded.exports}
        assert "identity" in export_names
        assert "module_info" in export_names

    def test_add_program_uses_arithmetic_bif(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        decoded = decode_beam_module(encode_beam(m))
        # erlang:+/2 must be in the import table.
        triples = {
            (imp.module, imp.function, imp.arity) for imp in decoded.imports
        }
        assert ("erlang", "+", 2) in triples


# ---------------------------------------------------------------------------
# Validation / error cases
# ---------------------------------------------------------------------------


class TestErrors:
    def test_missing_module_name_rejected(self) -> None:
        with pytest.raises(BEAMBackendError, match="module_name"):
            lower_ir_to_beam(
                _build_identity_program(), BEAMBackendConfig(module_name="")
            )

    def test_program_without_label_rejected(self) -> None:
        program = IrProgram(entry_label="x")
        program.add_instruction(IrInstruction(IrOp.RET, []))
        with pytest.raises(BEAMBackendError, match="must begin with at least one LABEL"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="m"))

    def test_unsupported_opcode_rejected_with_clear_message(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="boom")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("boom")], id=-1)
        )
        # SYSCALL isn't supported in TW03 Phase 1.
        program.add_instruction(
            IrInstruction(IrOp.SYSCALL, [_imm(1), _reg(1)], id=gen.next())
        )
        with pytest.raises(BEAMBackendError, match="unsupported IR op SYSCALL"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="boom"))

    def test_load_imm_negative_rejected(self) -> None:
        gen = IDGenerator()
        program = IrProgram(entry_label="neg")
        program.add_instruction(
            IrInstruction(IrOp.LABEL, [_label("neg")], id=-1)
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(0), _imm(-1)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        with pytest.raises(BEAMBackendError, match="negative integer"):
            lower_ir_to_beam(program, BEAMBackendConfig(module_name="neg"))


# ---------------------------------------------------------------------------
# Sanity: opcode bytes look right
# ---------------------------------------------------------------------------


class TestOpcodeChoices:
    def test_load_imm_emits_move_opcode(self) -> None:
        m = lower_ir_to_beam(
            _build_identity_program(),
            BEAMBackendConfig(module_name="answer"),
        )
        # Opcodes used should include 64 (move), 19 (return),
        # 1 (label), 2 (func_info), 3 (int_code_end).
        used = {ins.opcode for ins in m.instructions}
        assert 64 in used   # move
        assert 19 in used   # return
        assert 1 in used    # label
        assert 2 in used    # func_info
        assert 3 in used    # int_code_end

    def test_arithmetic_emits_gc_bif2(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        used = {ins.opcode for ins in m.instructions}
        assert 125 in used  # gc_bif2

    def test_arithmetic_operand_tags(self) -> None:
        m = lower_ir_to_beam(
            _build_add_program(),
            BEAMBackendConfig(module_name="adder"),
        )
        gc_bif2_instructions = [ins for ins in m.instructions if ins.opcode == 125]
        assert len(gc_bif2_instructions) == 1
        ops = gc_bif2_instructions[0].operands
        # gc_bif2 has 6 operands: fail-label, live, bif-idx, src1, src2, dest.
        # TW03 Phase 1 — sources/dest are y-registers (callee-saves
        # state survives recursive calls).
        assert len(ops) == 6
        assert ops[0].tag == BEAMTag.F
        assert ops[1].tag == BEAMTag.U
        assert ops[2].tag == BEAMTag.U
        assert ops[3].tag == BEAMTag.Y
        assert ops[4].tag == BEAMTag.Y
        assert ops[5].tag == BEAMTag.Y

    def test_branch_lowering_uses_is_ne_exact(self) -> None:
        """``BRANCH_Z r, label`` lowers to ``is_ne_exact label, {y,r}, {integer,0}``."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="b")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("b")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(0)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(
                IrOp.BRANCH_Z, [_reg(1), _label("end")], id=gen.next()
            )
        )
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("end")], id=-1))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(program, BEAMBackendConfig(module_name="b"))
        used = {ins.opcode for ins in m.instructions}
        assert 44 in used  # is_ne_exact

    def test_cmp_eq_lowers_to_5_instructions(self) -> None:
        """``CMP_EQ`` becomes is_eq_exact + move 1 + jump + label + move 0 + label."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="c")
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("c")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(2), _imm(1)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [_reg(3), _imm(2)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.CMP_EQ, [_reg(1), _reg(2), _reg(3)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(program, BEAMBackendConfig(module_name="c"))
        used = {ins.opcode for ins in m.instructions}
        assert 43 in used  # is_eq_exact
        assert 61 in used  # jump

    def test_recursive_function_emits_allocate_call_deallocate(self) -> None:
        """Recursive ``fact`` IR uses CALL — backend emits allocate / call / deallocate."""
        from compiler_ir import IDGenerator
        gen = IDGenerator()
        program = IrProgram(entry_label="main")
        # fact(n)
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("fact")], id=-1))
        # Body: just CALL itself (artificial — no termination, but enough
        # to exercise allocate + call + deallocate emission).
        program.add_instruction(IrInstruction(IrOp.CALL, [_label("fact")], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
        # main
        program.add_instruction(IrInstruction(IrOp.LABEL, [_label("main")], id=-1))
        program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

        m = lower_ir_to_beam(
            program,
            BEAMBackendConfig(
                module_name="rec",
                arity_overrides={"fact": 1},
            ),
        )
        used = {ins.opcode for ins in m.instructions}
        assert 12 in used   # allocate
        assert 18 in used   # deallocate
        assert 4 in used    # call
