"""Tests for the first logic loader bytecode layer."""

import pytest
from logic_engine import conj, relation, var
from logic_instructions import (
    InstructionProgram,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
)

from logic_bytecode import (
    LogicBytecodeError,
    LogicBytecodeInstruction,
    LogicBytecodeOp,
    LogicBytecodeProgram,
    __version__,
    compile_program,
    decode_program,
    disassemble,
    disassemble_text,
)


def _ancestor_program() -> InstructionProgram:
    """Build one small recursive logic program for bytecode tests."""

    parent = relation("parent", 2)
    ancestor = relation("ancestor", 2)
    x = var("X")
    y = var("Y")
    z = var("Z")
    who = var("Who")

    return instruction_program(
        defrel(parent),
        defrel(ancestor),
        fact(parent("homer", "bart")),
        fact(parent("homer", "lisa")),
        rule(ancestor(x, y), parent(x, y)),
        rule(ancestor(x, y), conj(parent(x, z), ancestor(z, y))),
        query(ancestor("homer", who), outputs=(who,)),
    )


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestLogicBytecode:
    """Compilation, decoding, and disassembly should all agree."""

    def test_compile_emits_expected_loader_opcodes(self) -> None:
        bytecode = compile_program(_ancestor_program())

        assert [instruction.opcode for instruction in bytecode.instructions] == [
            LogicBytecodeOp.EMIT_RELATION,
            LogicBytecodeOp.EMIT_RELATION,
            LogicBytecodeOp.EMIT_FACT,
            LogicBytecodeOp.EMIT_FACT,
            LogicBytecodeOp.EMIT_RULE,
            LogicBytecodeOp.EMIT_RULE,
            LogicBytecodeOp.EMIT_QUERY,
            LogicBytecodeOp.HALT,
        ]

    def test_compile_preserves_dynamic_relation_declarations(self) -> None:
        memo = relation("memo", 1)
        value = var("Value")
        program_value = instruction_program(
            defdynamic(memo),
            fact(memo("cached")),
            query(memo(value), outputs=(value,)),
        )
        bytecode = compile_program(program_value)

        assert [instruction.opcode for instruction in bytecode.instructions] == [
            LogicBytecodeOp.EMIT_DYNAMIC_RELATION,
            LogicBytecodeOp.EMIT_FACT,
            LogicBytecodeOp.EMIT_QUERY,
            LogicBytecodeOp.HALT,
        ]
        assert decode_program(bytecode) == program_value
        assert "0000: EMIT_DYNAMIC_RELATION 0 ; memo/1" in disassemble_text(bytecode)

    def test_compile_appends_halt(self) -> None:
        bytecode = compile_program(_ancestor_program())

        assert bytecode.instructions[-1] == LogicBytecodeInstruction(
            opcode=LogicBytecodeOp.HALT,
        )

    def test_relation_pool_is_deduplicated_across_all_relation_references(self) -> None:
        bytecode = compile_program(_ancestor_program())

        assert bytecode.relation_pool == (
            relation("parent", 2),
            relation("ancestor", 2),
        )

    def test_compile_decode_round_trips(
        self,
    ) -> None:
        program_value = _ancestor_program()

        assert decode_program(compile_program(program_value)) == program_value

    def test_disassemble_renders_human_readable_text(self) -> None:
        text = disassemble_text(compile_program(_ancestor_program()))

        assert "0000: EMIT_RELATION 0 ; parent/2" in text
        assert "0002: EMIT_FACT 0 ; parent(homer, bart)" in text
        assert "0006: EMIT_QUERY 0 ; ancestor(homer, Who)" in text
        assert text.splitlines()[-1] == "0007: HALT"

    def test_disassemble_returns_structured_lines(self) -> None:
        lines = disassemble(compile_program(_ancestor_program()))

        assert lines[0].opcode == "EMIT_RELATION"
        assert lines[0].operand == 0
        assert lines[-1].opcode == "HALT"
        assert lines[-1].comment is None

    def test_decode_rejects_unknown_opcodes(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(opcode=0x99),
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        with pytest.raises(LogicBytecodeError, match="unknown logic bytecode opcode"):
            decode_program(malformed)

    def test_decode_rejects_missing_operands(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.EMIT_RELATION),
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
            ),
            relation_pool=(relation("parent", 2),),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        with pytest.raises(LogicBytecodeError, match="requires an operand"):
            decode_program(malformed)

    def test_decode_rejects_out_of_range_pool_indexes(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_FACT,
                    operand=7,
                ),
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        with pytest.raises(LogicBytecodeError, match="fact pool index 7"):
            decode_program(malformed)

    def test_decode_rejects_negative_pool_indexes(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_FACT,
                    operand=-1,
                ),
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        with pytest.raises(LogicBytecodeError, match="fact pool index -1"):
            decode_program(malformed)

    def test_decode_rejects_missing_halt(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(
                    opcode=LogicBytecodeOp.EMIT_QUERY,
                    operand=0,
                ),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(query(relation("parent", 2)("homer", "bart")),),
        )

        with pytest.raises(LogicBytecodeError, match="missing a HALT"):
            decode_program(malformed)

    def test_decode_rejects_trailing_instructions_after_halt(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
                LogicBytecodeInstruction(opcode=LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        with pytest.raises(LogicBytecodeError, match="HALT must be the final"):
            decode_program(malformed)
