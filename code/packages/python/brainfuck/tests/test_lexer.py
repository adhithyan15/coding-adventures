"""Tests for the Brainfuck lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``brainfuck.tokens``, correctly tokenizes Brainfuck source text.

Brainfuck is one of the simplest languages to lex: exactly 8 single-character
command tokens, with everything else silently discarded. There are no strings,
no numbers, no keywords, and no multi-character tokens.

Test Categories
---------------

1. All 8 command tokens — each of ><+-.,[] produces the correct token type
2. Comment skipping — non-command characters are silently discarded
3. Position tracking — line and column numbers advance correctly
4. Empty source — empty string produces only EOF
5. Canonical ``++[>+<-]`` program — the classic loop pattern
6. Mixed commands and comments — realistic annotated Brainfuck source
7. Multi-line programs — newlines are skipped, line counter advances
8. EOF is always the last token
"""

from __future__ import annotations

import pytest

from brainfuck.lexer import create_brainfuck_lexer, tokenize_brainfuck
from lexer import Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def token_types(source: str) -> list[str]:
    """Tokenize and return just the type names, excluding EOF."""
    tokens = tokenize_brainfuck(source)
    return [
        t.type if isinstance(t.type, str) else t.type.name
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values, excluding EOF."""
    tokens = tokenize_brainfuck(source)
    return [
        t.value
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


# ---------------------------------------------------------------------------
# All 8 command tokens
# ---------------------------------------------------------------------------


class TestAllEightCommands:
    """Verify that each of the 8 Brainfuck command characters produces the correct token."""

    def test_right_command(self) -> None:
        """The ``>`` character produces a RIGHT token.

        ``>`` moves the data pointer one cell to the right on the tape.
        Starting from cell 0, after ``>``, the pointer is at cell 1.
        """
        tokens = tokenize_brainfuck(">")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "RIGHT"
        assert first.value == ">"

    def test_left_command(self) -> None:
        """The ``<`` character produces a LEFT token.

        ``<`` moves the data pointer one cell to the left on the tape.
        """
        tokens = tokenize_brainfuck("<")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "LEFT"
        assert first.value == "<"

    def test_inc_command(self) -> None:
        """The ``+`` character produces an INC token.

        ``+`` increments the byte at the current data pointer. After 256
        increments from 0, the cell wraps back to 0 (unsigned byte arithmetic).
        """
        tokens = tokenize_brainfuck("+")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "INC"
        assert first.value == "+"

    def test_dec_command(self) -> None:
        """The ``-`` character produces a DEC token.

        ``-`` decrements the byte at the current data pointer. From 0 it
        wraps to 255.
        """
        tokens = tokenize_brainfuck("-")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "DEC"
        assert first.value == "-"

    def test_output_command(self) -> None:
        """The ``.`` character produces an OUTPUT token.

        ``.`` outputs the byte at the current data pointer as an ASCII character.
        Cell value 65 outputs the letter 'A'.
        """
        tokens = tokenize_brainfuck(".")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "OUTPUT"
        assert first.value == "."

    def test_input_command(self) -> None:
        """The ``,`` character produces an INPUT token.

        ``,`` reads one byte from the input stream into the current cell.
        """
        tokens = tokenize_brainfuck(",")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "INPUT"
        assert first.value == ","

    def test_loop_start_command(self) -> None:
        """The ``[`` character produces a LOOP_START token.

        ``[`` begins a loop. If the current cell is zero, execution jumps
        past the matching ``]``. The lexer tokenizes it regardless of whether
        a matching ``]`` exists — bracket matching is the parser's job.
        """
        tokens = tokenize_brainfuck("[")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "LOOP_START"
        assert first.value == "["

    def test_loop_end_command(self) -> None:
        """The ``]`` character produces a LOOP_END token.

        ``]`` ends a loop. If the current cell is nonzero, execution jumps
        back to the matching ``[``. The idiom ``[-]`` clears the current cell.
        """
        tokens = tokenize_brainfuck("]")
        first = tokens[0]
        type_name = first.type if isinstance(first.type, str) else first.type.name
        assert type_name == "LOOP_END"
        assert first.value == "]"


# ---------------------------------------------------------------------------
# Comment skipping
# ---------------------------------------------------------------------------


class TestCommentSkipping:
    """Verify that non-command characters are silently discarded."""

    def test_alphabetic_comment_produces_no_tokens(self) -> None:
        """Alphabetic text with no commands produces an empty token list.

        In Brainfuck, any character that isn't one of the 8 commands is a
        comment. The COMMENT skip pattern in brainfuck.tokens matches all
        non-command, non-whitespace characters.
        """
        types = token_types("this is a comment")
        assert types == []

    def test_commands_with_trailing_comment(self) -> None:
        """Commands followed by a comment produce only the command tokens.

        Brainfuck programmers commonly write annotations like:
            +++ set cell to 3
        The text "set cell to 3" should be silently discarded.
        """
        types = token_types("+++ set cell to 3")
        assert types == ["INC", "INC", "INC"]

    def test_digits_are_comments(self) -> None:
        """Digit characters are not Brainfuck commands.

        ``42`` contains no commands, just two comment characters (digits).
        This is a common misconception: ``+ 10 times`` does NOT run ``+``
        ten times — the ``10`` is just prose comment text.
        """
        types = token_types("42")
        assert types == []

    def test_commands_interspersed_with_comments(self) -> None:
        """Commands amid prose: only the command characters survive.

        ``> move right < move left`` should produce RIGHT, LEFT — the
        surrounding words are comments.
        """
        types = token_types("> move right < move left")
        assert types == ["RIGHT", "LEFT"]


# ---------------------------------------------------------------------------
# Empty source
# ---------------------------------------------------------------------------


class TestEmptySource:
    """Verify behaviour on empty and whitespace-only sources."""

    def test_empty_string_produces_only_eof(self) -> None:
        """An empty source string returns exactly one token: EOF.

        An empty Brainfuck program is valid — it simply does nothing.
        The lexer always appends a synthetic EOF sentinel token.
        """
        tokens = tokenize_brainfuck("")
        assert len(tokens) == 1
        type_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert type_name == "EOF"

    def test_whitespace_only_produces_only_eof(self) -> None:
        """A source containing only whitespace returns exactly one token: EOF.

        All whitespace is consumed by the WHITESPACE skip pattern, leaving
        no command tokens. The result is the same as an empty source.
        """
        tokens = tokenize_brainfuck("   \\t\\n   ")
        assert len(tokens) == 1
        type_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert type_name == "EOF"


# ---------------------------------------------------------------------------
# EOF is always last
# ---------------------------------------------------------------------------


class TestEOFAlwaysLast:
    """Verify that the last token in every result is EOF."""

    def test_eof_is_last_for_single_command(self) -> None:
        """A single command produces [command, EOF].

        The EOF token is a synthetic sentinel that the generic lexer always
        appends. It tells the parser there are no more tokens.
        """
        tokens = tokenize_brainfuck("+")
        last = tokens[-1]
        type_name = last.type if isinstance(last.type, str) else last.type.name
        assert type_name == "EOF"

    def test_eof_is_last_for_multiple_commands(self) -> None:
        """Multiple commands produce [cmd, cmd, ..., EOF].

        Regardless of how many command tokens the source produces, the
        final token is always EOF.
        """
        tokens = tokenize_brainfuck("++--")
        last = tokens[-1]
        type_name = last.type if isinstance(last.type, str) else last.type.name
        assert type_name == "EOF"


# ---------------------------------------------------------------------------
# Position tracking
# ---------------------------------------------------------------------------


class TestPositionTracking:
    """Verify that line and column numbers are tracked correctly."""

    def test_first_command_is_at_line_1_column_1(self) -> None:
        """The first command in a source is at line 1, column 1.

        Both line and column are 1-indexed (first line is 1, first column
        is 1). This follows the Token interface convention.
        """
        tokens = tokenize_brainfuck(">")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_column_advances_along_first_line(self) -> None:
        """Commands on the same line have increasing column numbers.

        For ``><``, the ``>`` is at column 1 and ``<`` is at column 2.
        """
        tokens = tokenize_brainfuck("><")
        assert tokens[0].column == 1
        assert tokens[1].column == 2

    def test_line_advances_across_newlines(self) -> None:
        """A newline causes the line counter to increment.

        The WHITESPACE skip pattern consumes newlines, which causes the
        lexer engine to advance its line counter. A command on the second
        line should have line == 2.

        Source::

            +   <- line 1
            -   <- line 2
        """
        tokens = tokenize_brainfuck("+\n-")
        assert tokens[0].line == 1
        assert tokens[1].line == 2

    def test_column_after_spaces(self) -> None:
        """Spaces advance the column counter.

        For ``+   -``, the ``+`` is at column 1 and ``-`` is at column 5
        (after 3 spaces).
        """
        tokens = tokenize_brainfuck("+   -")
        assert tokens[0].column == 1
        assert tokens[1].column == 5


# ---------------------------------------------------------------------------
# Canonical ++[>+<-] pattern
# ---------------------------------------------------------------------------


class TestCanonicalPattern:
    """Tests for the canonical ``++[>+<-]`` Brainfuck idiom."""

    def test_produces_eight_command_tokens_plus_eof(self) -> None:
        """``++[>+<-]`` produces exactly 8 command tokens + EOF = 9 total.

        The sequence encodes 8 Brainfuck commands:
            ++     — set cell 0 to 2
            [      — loop while cell 0 is nonzero
              >    — move to cell 1
              +    — increment cell 1
              <    — return to cell 0
              -    — decrement cell 0
            ]      — end loop

        Result: cell 0 = 0, cell 1 = 2.
        """
        tokens = tokenize_brainfuck("++[>+<-]")
        assert len(tokens) == 9  # 8 commands + EOF

    def test_produces_correct_type_sequence(self) -> None:
        """``++[>+<-]`` produces the exact expected token type sequence."""
        types = token_types("++[>+<-]")
        assert types == [
            "INC",         # +
            "INC",         # +
            "LOOP_START",  # [
            "RIGHT",       # >
            "INC",         # +
            "LEFT",        # <
            "DEC",         # -
            "LOOP_END",    # ]
        ]

    def test_produces_correct_values(self) -> None:
        """``++[>+<-]`` produces the exact expected token value sequence."""
        values = token_values("++[>+<-]")
        assert values == ["+", "+", "[", ">", "+", "<", "-", "]"]

    def test_with_inline_comments_same_as_without(self) -> None:
        """Adding comments to ``++[>+<-]`` does not change the token sequence.

        A Brainfuck programmer might write:
            ++ setup  [ copy loop >+ cell1 <-  ]

        The comments ``setup``, ``copy loop``, ``cell1`` should be discarded,
        leaving the same 8 command tokens.
        """
        clean_types = token_types("++[>+<-]")
        commented_types = token_types("++ setup  [ copy loop >+ cell1 <-  ]")
        assert clean_types == commented_types

    def test_all_eight_distinct_commands(self) -> None:
        """``><+-.,[]`` contains one of each Brainfuck command.

        This verifies that all 8 token types are distinct and correctly
        identified by the lexer.
        """
        types = token_types("><+-.,[]")
        assert types == [
            "RIGHT", "LEFT", "INC", "DEC", "OUTPUT", "INPUT",
            "LOOP_START", "LOOP_END",
        ]


# ---------------------------------------------------------------------------
# create_brainfuck_lexer factory
# ---------------------------------------------------------------------------


class TestCreateBrainfuckLexer:
    """Verify that the ``create_brainfuck_lexer`` factory function works."""

    def test_factory_returns_grammar_lexer(self) -> None:
        """``create_brainfuck_lexer`` returns a ``GrammarLexer`` instance.

        The factory function is provided for callers who want to inspect
        the grammar or control tokenization step-by-step.
        """
        from lexer import GrammarLexer

        lexer = create_brainfuck_lexer("+")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_tokenize_matches_convenience_function(self) -> None:
        """``create_brainfuck_lexer(s).tokenize()`` equals ``tokenize_brainfuck(s)``.

        Both code paths should produce identical token lists.
        """
        source = "++[>+<-]"
        via_factory = create_brainfuck_lexer(source).tokenize()
        via_convenience = tokenize_brainfuck(source)

        factory_types = [
            t.type if isinstance(t.type, str) else t.type.name
            for t in via_factory
        ]
        convenience_types = [
            t.type if isinstance(t.type, str) else t.type.name
            for t in via_convenience
        ]
        assert factory_types == convenience_types
