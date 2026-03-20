"""End-to-end tests — real Brainfuck programs translated and executed."""

from __future__ import annotations

from brainfuck.vm import execute_brainfuck


class TestSimplePrograms:
    """Small programs that test fundamental behavior."""

    def test_empty_program(self) -> None:
        result = execute_brainfuck("")
        assert result.output == ""
        assert result.tape[0] == 0

    def test_single_inc(self) -> None:
        result = execute_brainfuck("+")
        assert result.tape[0] == 1

    def test_addition(self) -> None:
        """2 + 5 = 7 — classic BF addition pattern.

        Put 2 in cell 0, 5 in cell 1.
        Loop: decrement cell 1, increment cell 0.
        Result: 7 in cell 0, 0 in cell 1.
        """
        result = execute_brainfuck("++>+++++[<+>-]")
        assert result.tape[0] == 7
        assert result.tape[1] == 0

    def test_move_value(self) -> None:
        """Move value from cell 0 to cell 1.

        Set cell 0 to 5, then [>+<-] moves it to cell 1.
        """
        result = execute_brainfuck("+++++[>+<-]")
        assert result.tape[0] == 0
        assert result.tape[1] == 5

    def test_cell_wrapping_overflow(self) -> None:
        """255 + 1 = 0."""
        source = "+" * 256
        result = execute_brainfuck(source)
        assert result.tape[0] == 0

    def test_cell_wrapping_underflow(self) -> None:
        """0 - 1 = 255."""
        result = execute_brainfuck("-")
        assert result.tape[0] == 255

    def test_skip_empty_loop(self) -> None:
        """[] is skipped when cell is 0 (which it starts as)."""
        result = execute_brainfuck("[]+++")
        assert result.tape[0] == 3


class TestOutput:
    """Programs that produce output."""

    def test_output_h(self) -> None:
        """Output 'H' (ASCII 72).

        9 * 8 = 72 → +++++++++[>++++++++<-]>.
        """
        result = execute_brainfuck("+++++++++[>++++++++<-]>.")
        assert result.output == "H"

    def test_output_multiple_chars(self) -> None:
        """Output 'AB' by setting cells to 65 and 66."""
        # Cell 0 = 65 ('A'), output, inc, output ('B')
        source = "+" * 65 + ".+."
        result = execute_brainfuck(source)
        assert result.output == "AB"

    def test_hello_world(self) -> None:
        """The classic Brainfuck Hello World program.

        This is the canonical test — if this works, everything works.
        Source: https://esolangs.org/wiki/Brainfuck
        """
        hello_world = (
            "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]"
            ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
        )
        result = execute_brainfuck(hello_world)
        assert result.output == "Hello World!\n"


class TestInput:
    """Programs that read input."""

    def test_echo_single_char(self) -> None:
        """, reads one byte, . outputs it."""
        result = execute_brainfuck(",.", input_data="X")
        assert result.output == "X"

    def test_cat_program(self) -> None:
        """,[.,] — echo input until EOF.

        Reads a character. If nonzero, output it and read the next.
        On EOF (0), the loop exits.
        """
        result = execute_brainfuck(",[.,]", input_data="Hi")
        assert result.output == "Hi"

    def test_input_to_cell(self) -> None:
        """Verify the cell holds the input byte value."""
        result = execute_brainfuck(",", input_data="A")
        assert result.tape[0] == 65  # ord('A')

    def test_eof_is_zero(self) -> None:
        """Reading with no input gives 0."""
        result = execute_brainfuck(",")
        assert result.tape[0] == 0


class TestNestedLoops:
    """Programs with nested loop structures."""

    def test_nested_multiplication(self) -> None:
        """2 * 3 = 6 using nested loops.

        Cell 0 = 2, Cell 1 = 3.
        Outer loop (cell 0): for each unit, add cell 1 to cell 2.
        Result: cell 2 = 6.

        ++           cell[0] = 2
        >+++         cell[1] = 3
        <            back to cell[0]
        [            while cell[0] != 0:
          >          move to cell[1]
          [>+>+<<-]  copy cell[1] to cell[2] and cell[3]
          >>         move to cell[3]
          [<<+>>-]   move cell[3] back to cell[1] (restore)
          <<<        back to cell[0]
          -          dec cell[0]
        ]
        """
        source = "++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]"
        result = execute_brainfuck(source)
        assert result.tape[2] == 6

    def test_deeply_nested(self) -> None:
        """++[>++[>+<-]<-] — nested decrement loops."""
        result = execute_brainfuck("++[>++[>+<-]<-]")
        # Outer loop runs 2 times.
        # Each time: cell[1] = 2, inner loop moves cell[1] to cell[2].
        # After 2 outer iterations: cell[2] = 2 + 2 = 4.
        assert result.tape[2] == 4
        assert result.tape[1] == 0
        assert result.tape[0] == 0


class TestBrainfuckResult:
    """Verify the BrainfuckResult dataclass."""

    def test_result_fields(self) -> None:
        result = execute_brainfuck("+++.")
        assert isinstance(result.output, str)
        assert isinstance(result.tape, list)
        assert isinstance(result.dp, int)
        assert isinstance(result.traces, list)
        assert isinstance(result.steps, int)

    def test_step_count(self) -> None:
        result = execute_brainfuck("+++")
        # 3 INCs + 1 HALT = 4 steps
        assert result.steps == 4

    def test_final_dp(self) -> None:
        result = execute_brainfuck(">>>")
        assert result.dp == 3

    def test_traces_populated(self) -> None:
        result = execute_brainfuck("+")
        assert len(result.traces) == 2  # INC + HALT


class TestComments:
    """Non-BF characters are comments and should not affect execution."""

    def test_comments_in_code(self) -> None:
        """Arbitrary text around BF commands is ignored."""
        result = execute_brainfuck("This is + a + program + .")
        assert result.tape[0] == 3

    def test_numbers_ignored(self) -> None:
        result = execute_brainfuck("123+456")
        assert result.tape[0] == 1

    def test_newlines_ignored(self) -> None:
        result = execute_brainfuck("+\n+\n+")
        assert result.tape[0] == 3
