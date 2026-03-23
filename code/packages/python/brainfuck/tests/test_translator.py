"""Tests for the Brainfuck translator (source → CodeObject)."""

from __future__ import annotations

import pytest

from brainfuck.opcodes import Op
from brainfuck.translator import TranslationError, translate


class TestBasicTranslation:
    """Each BF character maps to one instruction."""

    def test_empty_program(self) -> None:
        code = translate("")
        assert len(code.instructions) == 1  # just HALT
        assert code.instructions[0].opcode == Op.HALT

    def test_single_right(self) -> None:
        code = translate(">")
        assert code.instructions[0].opcode == Op.RIGHT
        assert code.instructions[1].opcode == Op.HALT

    def test_single_left(self) -> None:
        code = translate("<")
        assert code.instructions[0].opcode == Op.LEFT

    def test_single_inc(self) -> None:
        code = translate("+")
        assert code.instructions[0].opcode == Op.INC

    def test_single_dec(self) -> None:
        code = translate("-")
        assert code.instructions[0].opcode == Op.DEC

    def test_single_output(self) -> None:
        code = translate(".")
        assert code.instructions[0].opcode == Op.OUTPUT

    def test_single_input(self) -> None:
        code = translate(",")
        assert code.instructions[0].opcode == Op.INPUT

    def test_multiple_commands(self) -> None:
        code = translate("+++>.")
        ops = [i.opcode for i in code.instructions]
        assert ops == [Op.INC, Op.INC, Op.INC, Op.RIGHT, Op.OUTPUT, Op.HALT]

    def test_comments_ignored(self) -> None:
        """Non-BF characters are treated as comments."""
        code = translate("hello + world - !")
        ops = [i.opcode for i in code.instructions]
        assert ops == [Op.INC, Op.DEC, Op.HALT]

    def test_whitespace_ignored(self) -> None:
        code = translate("  +  +  +  ")
        ops = [i.opcode for i in code.instructions]
        assert ops == [Op.INC, Op.INC, Op.INC, Op.HALT]

    def test_empty_constant_pool(self) -> None:
        code = translate("+++")
        assert code.constants == []

    def test_empty_name_pool(self) -> None:
        code = translate("+++")
        assert code.names == []


class TestBracketMatching:
    """[ and ] are matched during translation."""

    def test_simple_loop(self) -> None:
        """[>+<-] — the simplest loop."""
        code = translate("[>+<-]")
        # Instructions: LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END, HALT
        assert len(code.instructions) == 7

        loop_start = code.instructions[0]
        loop_end = code.instructions[5]

        assert loop_start.opcode == Op.LOOP_START
        assert loop_start.operand == 6  # jump past LOOP_END (index 5) to HALT (index 6)

        assert loop_end.opcode == Op.LOOP_END
        assert loop_end.operand == 0  # jump back to LOOP_START

    def test_nested_loops(self) -> None:
        """++[>++[>+<-]<-] — outer and inner loops.

        Instruction layout:
            0: INC, 1: INC, 2: LOOP_START(15),
            3: RIGHT, 4: INC, 5: INC, 6: LOOP_START(12),
            7: RIGHT, 8: INC, 9: LEFT, 10: DEC, 11: LOOP_END(6),
            12: LEFT, 13: DEC, 14: LOOP_END(2),
            15: HALT
        """
        code = translate("++[>++[>+<-]<-]")
        # Find the loop instructions
        loops = [
            (i, inst)
            for i, inst in enumerate(code.instructions)
            if inst.opcode in (Op.LOOP_START, Op.LOOP_END)
        ]
        # Should have 2 LOOP_STARTs and 2 LOOP_ENDs
        assert len(loops) == 4

        # Outer [ at index 2, inner [ at index 6
        outer_start = code.instructions[2]
        inner_start = code.instructions[6]

        assert outer_start.opcode == Op.LOOP_START
        assert inner_start.opcode == Op.LOOP_START

        # Inner ] at index 11, outer ] at index 14
        inner_end = code.instructions[11]
        outer_end = code.instructions[14]

        # Inner loop: [ at 6 jumps to 12 (past ] at 11), ] at 11 jumps back to 6
        assert inner_start.operand == 12
        assert inner_end.operand == 6

        # Outer loop: [ at 2 jumps to 15 (past ] at 14), ] at 14 jumps back to 2
        assert outer_start.operand == 15
        assert outer_end.operand == 2

    def test_empty_loop(self) -> None:
        """[] — an empty loop (infinite if cell != 0)."""
        code = translate("[]")
        assert code.instructions[0].opcode == Op.LOOP_START
        assert code.instructions[0].operand == 2  # past LOOP_END
        assert code.instructions[1].opcode == Op.LOOP_END
        assert code.instructions[1].operand == 0  # back to LOOP_START

    def test_adjacent_loops(self) -> None:
        """[][] — two loops side by side."""
        code = translate("[][]")
        # First loop: [0] → LOOP_START(2), [1] → LOOP_END(0)
        # Second loop: [2] → LOOP_START(4), [3] → LOOP_END(2)
        assert code.instructions[0].operand == 2
        assert code.instructions[1].operand == 0
        assert code.instructions[2].operand == 4
        assert code.instructions[3].operand == 2


class TestBracketErrors:
    """Mismatched brackets are caught during translation."""

    def test_unmatched_open_bracket(self) -> None:
        with pytest.raises(TranslationError, match="Unmatched '\\['"):
            translate("[")

    def test_unmatched_close_bracket(self) -> None:
        with pytest.raises(TranslationError, match="Unmatched '\\]'"):
            translate("]")

    def test_extra_open_bracket(self) -> None:
        with pytest.raises(TranslationError, match="Unmatched '\\['"):
            translate("[[]")

    def test_extra_close_bracket(self) -> None:
        with pytest.raises(TranslationError, match="Unmatched '\\]'"):
            translate("[]]")

    def test_multiple_unmatched(self) -> None:
        with pytest.raises(TranslationError, match="2 unclosed"):
            translate("[[")
