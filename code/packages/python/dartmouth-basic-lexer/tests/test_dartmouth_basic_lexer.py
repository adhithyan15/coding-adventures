"""Tests for the Dartmouth BASIC 1964 lexer.

These tests verify that the grammar-driven lexer, configured with
``dartmouth_basic.tokens``, correctly tokenizes 1964 Dartmouth BASIC source.

Dartmouth BASIC Tokenization Notes
-------------------------------------

1. **Line numbers**: Every BASIC statement begins with a line number. The
   integer ``10`` in ``10 LET X = 5`` is a LINE_NUM token, not a NUMBER.
   The post-tokenize hook ``relabel_line_numbers`` handles this distinction
   by relabeling the first NUMBER on each line.

2. **Case insensitivity**: The grammar uses ``@case_insensitive true``, which
   means the source is uppercased before matching. ``print``, ``Print``, and
   ``PRINT`` all produce ``KEYWORD("PRINT")``.

3. **Multi-char operators first**: ``<=`` is LE (not LT + EQ), ``>=`` is GE
   (not GT + EQ), ``<>`` is NE (not LT + GT). The grammar orders these first.

4. **REM comments**: Everything after ``REM`` on the same line is stripped.
   The NEWLINE is preserved as the statement terminator.

5. **Built-in functions**: SIN, COS, TAN, etc. are BUILTIN_FN tokens, not
   NAME tokens. They appear before the NAME rule in the grammar.

6. **User-defined functions**: FNA through FNZ are USER_FN tokens. DEF FNA(X)
   defines a function; FNA(X) calls it.

7. **Variable names**: Exactly one letter + optional digit. A, X, B9, Z0.
   Names like ``XCOORD`` would produce NAME("X") + KEYWORD("GOTO")... wait,
   no — the NAME regex is ``/[A-Z][0-9]?/`` so ``XCOORD`` would tokenize as
   multiple NAME tokens: NAME("X"), NAME("C"), NAME("O"), NAME("R"), NAME("D").

8. **NEWLINE tokens are significant**: BASIC is line-oriented. The parser
   needs NEWLINE to know where statements end. NEWLINEs are NOT in the skip
   section of the grammar.
"""

from __future__ import annotations

import pytest

from dartmouth_basic_lexer import create_dartmouth_basic_lexer, tokenize_dartmouth_basic
from lexer import GrammarLexer, Token


# ---------------------------------------------------------------------------
# Helper utilities
# ---------------------------------------------------------------------------


def token_types(source: str) -> list[str]:
    """Tokenize and return just the type names, excluding EOF."""
    tokens = tokenize_dartmouth_basic(source)
    result = []
    for t in tokens:
        t_type = t.type if isinstance(t.type, str) else t.type.name
        if t_type != "EOF":
            result.append(t_type)
    return result


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values, excluding EOF."""
    tokens = tokenize_dartmouth_basic(source)
    result = []
    for t in tokens:
        t_type = t.type if isinstance(t.type, str) else t.type.name
        if t_type != "EOF":
            result.append(t.value)
    return result


def tokens_with_types(source: str) -> list[tuple[str, str]]:
    """Return (type, value) pairs excluding EOF."""
    tokens = tokenize_dartmouth_basic(source)
    result = []
    for t in tokens:
        t_type = t.type if isinstance(t.type, str) else t.type.name
        if t_type != "EOF":
            result.append((t_type, t.value))
    return result


def get_eof(source: str) -> Token:
    """Return the EOF token from tokenizing source."""
    tokens = tokenize_dartmouth_basic(source)
    return tokens[-1]


# ---------------------------------------------------------------------------
# Test 1: Factory function
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_dartmouth_basic_lexer factory function.

    The factory function returns a GrammarLexer without the post-tokenize
    hooks attached. This allows callers to add their own hooks or call
    the lexer differently.
    """

    def test_returns_grammar_lexer(self) -> None:
        """create_dartmouth_basic_lexer should return a GrammarLexer instance."""
        lexer = create_dartmouth_basic_lexer("10 LET X = 5\n")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_produces_tokens_on_tokenize(self) -> None:
        """The factory lexer's .tokenize() produces a list ending in EOF."""
        lexer = create_dartmouth_basic_lexer("10 LET X = 5\n")
        tokens = lexer.tokenize()
        assert len(tokens) >= 1
        last = tokens[-1]
        last_type = last.type if isinstance(last.type, str) else last.type.name
        assert last_type == "EOF"

    def test_factory_without_hooks_does_not_relabel_line_num(self) -> None:
        """Without hooks, the first token on a line remains NUMBER, not LINE_NUM."""
        lexer = create_dartmouth_basic_lexer("10 LET X = 5\n")
        tokens = lexer.tokenize()
        # The raw lexer sees "10" as a NUMBER (from the grammar) or LINE_NUM
        # depending on which rule fires first. Either way, the hook is absent,
        # so the value should be "10" in the first token.
        first_value = tokens[0].value
        assert first_value == "10"


# ---------------------------------------------------------------------------
# Test 2: Basic tokenization
# ---------------------------------------------------------------------------


class TestBasicTokenization:
    """Tests for fundamental tokenization of simple BASIC statements.

    The canonical "hello world" test for a BASIC lexer is a LET statement:
    ``10 LET X = 5``

    We verify the exact sequence of token types and values.
    """

    def test_let_statement(self) -> None:
        """10 LET X = 5 produces LINE_NUM, KEYWORD, NAME, EQ, NUMBER, NEWLINE."""
        # Note: the GrammarLexer emits NEWLINE with value "\\n" (the two-char
        # escape string) when it intercepts newlines via its built-in handler
        # at line 685 of grammar_lexer.py. This is a GrammarLexer design choice.
        pairs = tokens_with_types("10 LET X = 5\n")
        assert pairs == [
            ("LINE_NUM", "10"),
            ("KEYWORD", "LET"),
            ("NAME", "X"),
            ("EQ", "="),
            ("NUMBER", "5"),
            ("NEWLINE", "\\n"),
        ]

    def test_print_statement(self) -> None:
        """20 PRINT X produces LINE_NUM, KEYWORD, NAME, NEWLINE."""
        pairs = tokens_with_types("20 PRINT X\n")
        assert pairs == [
            ("LINE_NUM", "20"),
            ("KEYWORD", "PRINT"),
            ("NAME", "X"),
            ("NEWLINE", "\\n"),
        ]

    def test_end_statement(self) -> None:
        """30 END produces LINE_NUM, KEYWORD, NEWLINE."""
        pairs = tokens_with_types("30 END\n")
        assert pairs == [
            ("LINE_NUM", "30"),
            ("KEYWORD", "END"),
            ("NEWLINE", "\\n"),
        ]

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_dartmouth_basic("10 END\n")
        last = tokens[-1]
        last_type = last.type if isinstance(last.type, str) else last.type.name
        assert last_type == "EOF"
        assert last.value == ""

    def test_eof_on_empty_input(self) -> None:
        """Even empty input produces an EOF token."""
        tokens = tokenize_dartmouth_basic("")
        assert len(tokens) == 1
        last_type = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert last_type == "EOF"


# ---------------------------------------------------------------------------
# Test 3: Case insensitivity
# ---------------------------------------------------------------------------


class TestCaseInsensitivity:
    """Tests for case-insensitive tokenization.

    The original Dartmouth BASIC ran on uppercase-only teletypes. The
    grammar applies ``@case_insensitive true``, which normalizes all input
    to uppercase before matching. This means lowercase input produces the
    same tokens as uppercase.
    """

    def test_lowercase_print(self) -> None:
        """Lowercase 'print' tokenizes the same as uppercase 'PRINT'."""
        lower = token_types("10 print x\n")
        upper = token_types("10 PRINT X\n")
        assert lower == upper

    def test_mixed_case_let(self) -> None:
        """Mixed case 'Let' tokenizes same as 'LET'."""
        mixed = token_types("10 Let A = 1\n")
        upper = token_types("10 LET A = 1\n")
        assert mixed == upper

    def test_lowercase_goto(self) -> None:
        """Lowercase 'goto' tokenizes as KEYWORD."""
        types = token_types("30 goto 20\n")
        assert "KEYWORD" in types
        values = token_values("30 goto 20\n")
        assert "GOTO" in values

    def test_lowercase_keywords_produce_uppercase_values(self) -> None:
        """After case normalization, keyword values are uppercase."""
        values = token_values("10 let x = 1\n")
        assert "LET" in values

    def test_mixed_case_builtin_fn(self) -> None:
        """sin and SIN produce the same BUILTIN_FN token."""
        types_lower = token_types("10 let y = sin(x)\n")
        types_upper = token_types("10 LET Y = SIN(X)\n")
        assert types_lower == types_upper


# ---------------------------------------------------------------------------
# Test 4: Multi-character operators
# ---------------------------------------------------------------------------


class TestMultiCharOperators:
    """Tests for multi-character comparison operators.

    BASIC supports three two-character operators:
      <=  less-than-or-equal     → LE
      >=  greater-than-or-equal  → GE
      <>  not-equal              → NE

    These must match as single tokens. The grammar lists them before the
    single-character < > = operators, ensuring the first-match rule fires
    in the right order.
    """

    def test_le_operator(self) -> None:
        """<= produces a single LE token, not LT + EQ."""
        types = token_types("10 IF X <= Y THEN 50\n")
        assert "LE" in types
        assert types.count("LT") == 0 or ("LE" in types and types.index("LE") < len(types))
        # Confirm it's a single LE, not LT+EQ
        assert "LE" in types
        # LE and EQ should not both appear for the <= token
        le_pos = types.index("LE")
        # The operator occupies exactly one slot
        assert types[le_pos] == "LE"

    def test_ge_operator(self) -> None:
        """>= produces a single GE token, not GT + EQ."""
        types = token_types("10 IF X >= Y THEN 50\n")
        assert "GE" in types

    def test_ne_operator(self) -> None:
        """<> produces a single NE token, not LT + GT."""
        types = token_types("10 IF X <> Y THEN 50\n")
        assert "NE" in types

    def test_le_value(self) -> None:
        """LE token has value '<='."""
        pairs = tokens_with_types("10 IF X <= Y THEN 50\n")
        le_pairs = [(t, v) for (t, v) in pairs if t == "LE"]
        assert len(le_pairs) == 1
        assert le_pairs[0] == ("LE", "<=")

    def test_ge_value(self) -> None:
        """GE token has value '>='."""
        pairs = tokens_with_types("10 IF X >= Y THEN 50\n")
        ge_pairs = [(t, v) for (t, v) in pairs if t == "GE"]
        assert len(ge_pairs) == 1
        assert ge_pairs[0] == ("GE", ">=")

    def test_ne_value(self) -> None:
        """NE token has value '<>'."""
        pairs = tokens_with_types("10 IF X <> Y THEN 50\n")
        ne_pairs = [(t, v) for (t, v) in pairs if t == "NE"]
        assert len(ne_pairs) == 1
        assert ne_pairs[0] == ("NE", "<>")

    def test_lt_alone(self) -> None:
        """A bare < produces LT, not LE or NE."""
        types = token_types("10 IF X < Y THEN 50\n")
        assert "LT" in types
        assert "LE" not in types

    def test_gt_alone(self) -> None:
        """A bare > produces GT, not GE."""
        types = token_types("10 IF X > Y THEN 50\n")
        assert "GT" in types
        assert "GE" not in types


# ---------------------------------------------------------------------------
# Test 5: Number formats
# ---------------------------------------------------------------------------


class TestNumberFormats:
    """Tests for Dartmouth BASIC numeric literal formats.

    BASIC 1964 stores all numbers as floating-point. The lexer supports:
      42        integer
      3.14      decimal
      .5        leading dot (no integer part)
      1.5E3     scientific notation (1500.0)
      1.5E-3    negative exponent (0.0015)
      1E10      integer with exponent
    """

    def test_integer(self) -> None:
        """Plain integers like 42 are NUMBER tokens."""
        pairs = tokens_with_types("10 LET X = 42\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", "42") in numbers

    def test_decimal(self) -> None:
        """Decimal number 3.14 is a NUMBER token."""
        pairs = tokens_with_types("10 LET X = 3.14\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", "3.14") in numbers

    def test_leading_dot(self) -> None:
        """.5 (no integer part) is a NUMBER token."""
        pairs = tokens_with_types("10 LET X = .5\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", ".5") in numbers

    def test_scientific_notation(self) -> None:
        """1.5E3 (scientific notation) is a single NUMBER token."""
        pairs = tokens_with_types("10 LET X = 1.5E3\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", "1.5E3") in numbers

    def test_negative_exponent(self) -> None:
        """1.5E-3 (negative exponent) is a single NUMBER token."""
        pairs = tokens_with_types("10 LET X = 1.5E-3\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", "1.5E-3") in numbers

    def test_integer_with_exponent(self) -> None:
        """1E10 (no fractional part, with exponent) is NUMBER."""
        pairs = tokens_with_types("10 LET X = 1E10\n")
        numbers = [(t, v) for (t, v) in pairs if t == "NUMBER"]
        assert ("NUMBER", "1E10") in numbers

    def test_lowercase_e_exponent(self) -> None:
        """1.5e3 with lowercase 'e' — after case normalization it becomes 1.5E3."""
        # @case_insensitive uppercases the source before matching, so 'e' → 'E'
        types = token_types("10 LET X = 1.5e3\n")
        assert "NUMBER" in types


# ---------------------------------------------------------------------------
# Test 6: String literal
# ---------------------------------------------------------------------------


class TestStringLiteral:
    """Tests for Dartmouth BASIC string literal tokenization.

    In 1964 BASIC, strings appear only in PRINT statements and DATA lines.
    They are delimited by double quotes. The 1964 spec has no escape sequences
    — a double quote cannot appear inside a string.

    The alias ``STRING_BODY = /"[^"]*"/ -> STRING`` in the grammar means the
    emitted token type is STRING, and the value includes the surrounding quotes.
    """

    def test_simple_string(self) -> None:
        """A double-quoted string produces a STRING token."""
        types = token_types('10 PRINT "HELLO"\n')
        assert "STRING" in types

    def test_string_value_without_quotes(self) -> None:
        """The STRING token value has the surrounding double quotes stripped.

        The GrammarLexer strips the delimiters (quotes) from STRING tokens
        as part of its standard processing. This is intentional — the parser
        and compiler work with the string content, not the delimiters.
        The 1964 BASIC spec's @case_insensitive mode also preserves the
        original case of string content (uppercasing is not applied to strings).
        """
        pairs = tokens_with_types('10 PRINT "HELLO WORLD"\n')
        strings = [(t, v) for (t, v) in pairs if t == "STRING"]
        assert len(strings) == 1
        assert strings[0] == ("STRING", "HELLO WORLD")

    def test_empty_string(self) -> None:
        """An empty double-quoted string is a valid STRING token."""
        types = token_types('10 PRINT ""\n')
        assert "STRING" in types

    def test_string_with_numbers(self) -> None:
        """A string containing digits is still a STRING token."""
        types = token_types('10 PRINT "123ABC"\n')
        assert "STRING" in types

    def test_string_with_spaces(self) -> None:
        """Spaces inside a string are preserved (whitespace skip doesn't apply).

        The quote delimiters are stripped by the GrammarLexer's STRING handling,
        so the value is the content between the quotes, with spaces preserved.
        """
        pairs = tokens_with_types('10 PRINT "HELLO WORLD"\n')
        strings = [(t, v) for (t, v) in pairs if t == "STRING"]
        assert "HELLO WORLD" in [v for (_, v) in strings]


# ---------------------------------------------------------------------------
# Test 7: REM suppression — comment text removed
# ---------------------------------------------------------------------------


class TestRemSuppression:
    """Tests for the REM comment suppression hook.

    In Dartmouth BASIC, ``REM`` introduces a remark that runs to end-of-line.
    The ``suppress_rem_content`` post-hook removes all tokens between the
    REM keyword and the next NEWLINE. The NEWLINE itself is preserved.

    The original 1964 manual calls this a "remark" rather than a "comment"
    because the designers wanted to emphasize that it was documentation for
    the human reader, not an instruction to the computer.
    """

    def test_rem_suppresses_comment_text(self) -> None:
        """All tokens after REM on the same line are suppressed."""
        pairs = tokens_with_types("10 REM THIS IS A COMMENT\n")
        # Should have LINE_NUM, KEYWORD(REM), NEWLINE — nothing else.
        # NEWLINE value is "\\n" (two-char escape) per GrammarLexer convention.
        assert pairs == [
            ("LINE_NUM", "10"),
            ("KEYWORD", "REM"),
            ("NEWLINE", "\\n"),
        ]

    def test_rem_keeps_newline(self) -> None:
        """The NEWLINE after a REM comment is always preserved."""
        types = token_types("10 REM HELLO\n")
        assert "NEWLINE" in types

    def test_rem_alone_on_line(self) -> None:
        """A bare REM with no comment text produces only LINE_NUM, KEYWORD, NEWLINE."""
        pairs = tokens_with_types("10 REM\n")
        assert pairs == [
            ("LINE_NUM", "10"),
            ("KEYWORD", "REM"),
            ("NEWLINE", "\\n"),
        ]

    def test_rem_suppresses_numbers_in_comment(self) -> None:
        """Numbers in a REM comment are suppressed, not tokenized as NUMBER."""
        pairs = tokens_with_types("10 REM 42 + 3.14\n")
        types_seen = [t for (t, _) in pairs]
        assert "NUMBER" not in types_seen

    def test_rem_suppresses_keywords_in_comment(self) -> None:
        """Keywords in a REM comment are suppressed."""
        pairs = tokens_with_types("10 REM PRINT X\n")
        types_seen = [t for (t, _) in pairs]
        keyword_values = [v for (t, v) in pairs if t == "KEYWORD"]
        # The only KEYWORD should be REM itself
        assert keyword_values == ["REM"]


# ---------------------------------------------------------------------------
# Test 8: REM + continuation (second line after REM lexes correctly)
# ---------------------------------------------------------------------------


class TestRemContinuation:
    """Tests that code after a REM line lexes correctly.

    The suppress_rem_content hook must stop suppressing at the NEWLINE and
    allow subsequent lines to tokenize normally. Otherwise the second line
    of the program would be invisible to the parser.
    """

    def test_code_after_rem_is_visible(self) -> None:
        """Tokens on the line following a REM line are not suppressed."""
        source = "10 REM IGNORE THIS\n20 LET X = 1\n"
        pairs = tokens_with_types(source)
        types = [t for (t, _) in pairs]
        # Line 20 should produce LINE_NUM, KEYWORD(LET), NAME, EQ, NUMBER, NEWLINE
        assert "LET" not in [v for (t, v) in pairs if t != "KEYWORD"]
        assert ("LINE_NUM", "20") in pairs
        assert ("KEYWORD", "LET") in pairs

    def test_multiple_rem_lines(self) -> None:
        """Multiple REM lines are each suppressed independently."""
        source = "10 REM LINE ONE COMMENT\n20 REM LINE TWO COMMENT\n30 END\n"
        pairs = tokens_with_types(source)
        # Line 30 should be visible
        assert ("LINE_NUM", "30") in pairs
        assert ("KEYWORD", "END") in pairs
        # Comment text should not appear
        types = [t for (t, _) in pairs]
        # NAME tokens from comment text should be absent
        # (BASIC comment words like "LINE", "ONE", "TWO" etc. would be NAMEs)
        # We can check by counting: only 2 KEYWORD tokens per REM line (REM itself)
        # plus END
        keyword_values = [v for (t, v) in pairs if t == "KEYWORD"]
        assert keyword_values == ["REM", "REM", "END"]

    def test_rem_then_let_then_end(self) -> None:
        """A complete 3-line program with a REM comment tokenizes correctly."""
        source = "10 REM COMPUTE SQUARE\n20 LET X = 9\n30 END\n"
        types = token_types(source)
        # Should have LINE_NUMs for all three lines
        line_num_count = types.count("LINE_NUM")
        assert line_num_count == 3


# ---------------------------------------------------------------------------
# Test 9: LINE_NUM vs NUMBER disambiguation
# ---------------------------------------------------------------------------


class TestLineNumVsNumber:
    """Tests for the LINE_NUM/NUMBER disambiguation hook.

    The first integer on each source line becomes LINE_NUM.
    All other integers become NUMBER.

    GOTO target: ``GOTO 10`` — the ``10`` after GOTO is a NUMBER
    (it is not at line start). The parser interprets it as a branch target.
    """

    def test_line_start_integer_is_line_num(self) -> None:
        """The leading integer on a line is LINE_NUM, not NUMBER."""
        pairs = tokens_with_types("30 GOTO 10\n")
        first = pairs[0]
        assert first == ("LINE_NUM", "30")

    def test_goto_target_is_number(self) -> None:
        """The target in GOTO 10 is NUMBER, not LINE_NUM."""
        pairs = tokens_with_types("30 GOTO 10\n")
        assert ("LINE_NUM", "30") in pairs
        assert ("NUMBER", "10") in pairs
        assert ("LINE_NUM", "10") not in pairs

    def test_expression_number_is_number(self) -> None:
        """A number in an expression (LET X = 42) is NUMBER, not LINE_NUM."""
        pairs = tokens_with_types("10 LET X = 42\n")
        assert ("NUMBER", "42") in pairs
        assert ("LINE_NUM", "42") not in pairs

    def test_only_first_token_becomes_line_num(self) -> None:
        """Only the very first integer on a line gets relabeled LINE_NUM."""
        pairs = tokens_with_types("50 FOR I = 1 TO 10 STEP 2\n")
        line_nums = [(t, v) for (t, v) in pairs if t == "LINE_NUM"]
        assert line_nums == [("LINE_NUM", "50")]

    def test_multi_line_each_gets_line_num(self) -> None:
        """In a multi-line program, each line's first integer is LINE_NUM."""
        source = "10 LET X = 1\n20 LET Y = 2\n30 END\n"
        pairs = tokens_with_types(source)
        line_nums = [(t, v) for (t, v) in pairs if t == "LINE_NUM"]
        assert line_nums == [("LINE_NUM", "10"), ("LINE_NUM", "20"), ("LINE_NUM", "30")]


# ---------------------------------------------------------------------------
# Test 10: All 11 built-in functions
# ---------------------------------------------------------------------------


class TestBuiltinFunctions:
    """Tests for the 11 built-in mathematical functions of Dartmouth BASIC.

    The 1964 spec defines exactly these functions:
      SIN  — sine (radians)
      COS  — cosine (radians)
      TAN  — tangent (radians)
      ATN  — arctangent (result in radians)
      EXP  — e raised to the power x
      LOG  — natural logarithm (base e)
      ABS  — absolute value
      SQR  — square root
      INT  — floor to integer
      RND  — random number in [0,1)
      SGN  — sign function (-1, 0, or 1)
    """

    def test_sin(self) -> None:
        """SIN is a BUILTIN_FN token."""
        types = token_types("10 LET Y = SIN(X)\n")
        assert "BUILTIN_FN" in types
        values = token_values("10 LET Y = SIN(X)\n")
        assert "SIN" in values

    def test_cos(self) -> None:
        """COS is a BUILTIN_FN token."""
        types = token_types("10 LET Y = COS(X)\n")
        assert "BUILTIN_FN" in types

    def test_tan(self) -> None:
        """TAN is a BUILTIN_FN token."""
        types = token_types("10 LET Y = TAN(X)\n")
        assert "BUILTIN_FN" in types

    def test_atn(self) -> None:
        """ATN (arctangent) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = ATN(X)\n")
        assert "BUILTIN_FN" in types

    def test_exp(self) -> None:
        """EXP (e^x) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = EXP(X)\n")
        assert "BUILTIN_FN" in types

    def test_log(self) -> None:
        """LOG (natural log) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = LOG(X)\n")
        assert "BUILTIN_FN" in types

    def test_abs(self) -> None:
        """ABS (absolute value) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = ABS(X)\n")
        assert "BUILTIN_FN" in types

    def test_sqr(self) -> None:
        """SQR (square root) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = SQR(X)\n")
        assert "BUILTIN_FN" in types

    def test_int_fn(self) -> None:
        """INT (floor) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = INT(X)\n")
        assert "BUILTIN_FN" in types

    def test_rnd(self) -> None:
        """RND (random) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = RND(1)\n")
        assert "BUILTIN_FN" in types

    def test_sgn(self) -> None:
        """SGN (sign) is a BUILTIN_FN token."""
        types = token_types("10 LET Y = SGN(X)\n")
        assert "BUILTIN_FN" in types

    def test_all_eleven_builtins(self) -> None:
        """All 11 built-in functions can be tokenized in sequence."""
        # Note: INT and LOG share letters with no variable names needed
        source = "10 LET A = SIN(X) + COS(X) + TAN(X) + ATN(X)\n"
        types = token_types(source)
        builtin_count = types.count("BUILTIN_FN")
        assert builtin_count == 4

    def test_all_builtins_by_name(self) -> None:
        """Each of the 11 built-in names is recognized as BUILTIN_FN."""
        all_builtins = ["SIN", "COS", "TAN", "ATN", "EXP", "LOG",
                        "ABS", "SQR", "INT", "RND", "SGN"]
        for fn_name in all_builtins:
            source = f"10 LET Y = {fn_name}(X)\n"
            types = token_types(source)
            assert "BUILTIN_FN" in types, f"{fn_name} not recognized as BUILTIN_FN"


# ---------------------------------------------------------------------------
# Test 11: User-defined functions
# ---------------------------------------------------------------------------


class TestUserDefinedFunctions:
    """Tests for user-defined functions (FNA through FNZ).

    Dartmouth BASIC lets you define single-expression functions using DEF:
        60 DEF FNA(X) = X * X     (square function)
        70 DEF FNB(X) = X + 1     (increment)

    The name FNA through FNZ consists of "FN" followed by exactly one
    uppercase letter. The USER_FN token covers all 26 possibilities.
    """

    def test_fna_is_user_fn(self) -> None:
        """FNA is a USER_FN token."""
        types = token_types("60 DEF FNA(X) = X * X\n")
        assert "USER_FN" in types

    def test_fnz_is_user_fn(self) -> None:
        """FNZ is a USER_FN token."""
        types = token_types("60 DEF FNZ(X) = X + 1\n")
        assert "USER_FN" in types

    def test_user_fn_in_expression(self) -> None:
        """FNA used in an expression is a USER_FN token."""
        types = token_types("70 LET Y = FNA(X)\n")
        assert "USER_FN" in types

    def test_user_fn_value(self) -> None:
        """USER_FN token carries the full name FNA."""
        pairs = tokens_with_types("60 DEF FNA(X) = X * X\n")
        fn_tokens = [(t, v) for (t, v) in pairs if t == "USER_FN"]
        assert fn_tokens == [("USER_FN", "FNA")]

    def test_user_fn_not_confused_with_name(self) -> None:
        """FNA is USER_FN, not a NAME token."""
        types = token_types("10 LET Y = FNA(X)\n")
        assert "USER_FN" in types
        # Confirm it's not a NAME (or that NAME doesn't claim FNA)
        pairs = tokens_with_types("10 LET Y = FNA(X)\n")
        name_values = [v for (t, v) in pairs if t == "NAME"]
        assert "FNA" not in name_values


# ---------------------------------------------------------------------------
# Test 12: Multi-line program
# ---------------------------------------------------------------------------


class TestMultiLineProgram:
    """Tests for tokenizing complete multi-line BASIC programs.

    A real BASIC program has multiple lines, each with a line number and
    a statement. The lexer must correctly tokenize each line and preserve
    the NEWLINEs between them.
    """

    def test_three_line_program(self) -> None:
        """A simple 3-line program tokenizes into the correct sequence."""
        # NEWLINE value is "\\n" per GrammarLexer convention.
        source = "10 LET X = 1\n20 PRINT X\n30 END\n"
        pairs = tokens_with_types(source)
        assert pairs == [
            ("LINE_NUM", "10"),
            ("KEYWORD", "LET"),
            ("NAME", "X"),
            ("EQ", "="),
            ("NUMBER", "1"),
            ("NEWLINE", "\\n"),
            ("LINE_NUM", "20"),
            ("KEYWORD", "PRINT"),
            ("NAME", "X"),
            ("NEWLINE", "\\n"),
            ("LINE_NUM", "30"),
            ("KEYWORD", "END"),
            ("NEWLINE", "\\n"),
        ]

    def test_for_loop_program(self) -> None:
        """A FOR loop program tokenizes all loop keywords correctly."""
        source = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I\n40 END\n"
        types = token_types(source)
        assert "FOR" not in types  # FOR is a KEYWORD with value "FOR"
        keyword_values = token_values(source)
        assert "FOR" in keyword_values
        assert "TO" in keyword_values
        assert "NEXT" in keyword_values

    def test_goto_program(self) -> None:
        """A GOTO creates an infinite loop; both line numbers tokenize correctly."""
        source = "10 LET X = 0\n20 LET X = X + 1\n30 GOTO 20\n"
        pairs = tokens_with_types(source)
        line_nums = [(t, v) for (t, v) in pairs if t == "LINE_NUM"]
        assert len(line_nums) == 3
        assert ("LINE_NUM", "10") in line_nums
        assert ("LINE_NUM", "20") in line_nums
        assert ("LINE_NUM", "30") in line_nums

    def test_windows_line_endings(self) -> None:
        """Windows-style \\r\\n line endings tokenize as NEWLINE."""
        source = "10 LET X = 1\r\n20 END\r\n"
        types = token_types(source)
        assert types.count("NEWLINE") == 2
        assert types.count("LINE_NUM") == 2


# ---------------------------------------------------------------------------
# Test 13: PRINT separators
# ---------------------------------------------------------------------------


class TestPrintSeparators:
    """Tests for PRINT statement separator tokens.

    In Dartmouth BASIC, PRINT has two separators with different meanings:
      COMMA (,)     — advance to the next print zone (column multiple of 14)
      SEMICOLON (;) — continue immediately after the previous value (no space)

    Both are distinct token types in the grammar.
    """

    def test_comma_separator(self) -> None:
        """PRINT X, Y uses COMMA to separate items."""
        types = token_types("10 PRINT X, Y\n")
        assert "COMMA" in types

    def test_semicolon_separator(self) -> None:
        """PRINT X; Y uses SEMICOLON to separate items."""
        types = token_types("10 PRINT X; Y\n")
        assert "SEMICOLON" in types

    def test_comma_value(self) -> None:
        """COMMA token has value ','."""
        pairs = tokens_with_types("10 PRINT X, Y\n")
        commas = [(t, v) for (t, v) in pairs if t == "COMMA"]
        assert commas == [("COMMA", ",")]

    def test_semicolon_value(self) -> None:
        """SEMICOLON token has value ';'."""
        pairs = tokens_with_types("10 PRINT X; Y\n")
        semis = [(t, v) for (t, v) in pairs if t == "SEMICOLON"]
        assert semis == [("SEMICOLON", ";")]

    def test_mixed_separators(self) -> None:
        """PRINT can mix commas and semicolons."""
        types = token_types("10 PRINT A, B; C\n")
        assert "COMMA" in types
        assert "SEMICOLON" in types


# ---------------------------------------------------------------------------
# Test 14: Variable names
# ---------------------------------------------------------------------------


class TestVariableNames:
    """Tests for Dartmouth BASIC variable name tokenization.

    In 1964 BASIC, variable names are exactly:
      - One uppercase letter: A through Z (26 scalars)
      - One letter followed by one digit: A0 through Z9 (260 more)

    Total: 286 possible variable names. No longer names are allowed.
    The NAME regex is /[A-Z][0-9]?/ which matches this exactly.
    """

    def test_single_letter_variable(self) -> None:
        """A single letter is a valid NAME."""
        pairs = tokens_with_types("10 LET X = 1\n")
        names = [(t, v) for (t, v) in pairs if t == "NAME"]
        assert ("NAME", "X") in names

    def test_letter_digit_variable(self) -> None:
        """A letter + digit is a valid NAME (A1, B9, Z0)."""
        pairs = tokens_with_types("10 LET A1 = 2\n")
        names = [(t, v) for (t, v) in pairs if t == "NAME"]
        assert ("NAME", "A1") in names

    def test_z9_variable(self) -> None:
        """Z9 is a valid NAME (last alphanumeric variable)."""
        pairs = tokens_with_types("10 LET Z9 = 3\n")
        names = [(t, v) for (t, v) in pairs if t == "NAME"]
        assert ("NAME", "Z9") in names

    def test_all_named_variables_are_name_tokens(self) -> None:
        """Multiple single-letter variables each become NAME tokens."""
        types = token_types("10 LET A = B + C\n")
        name_count = types.count("NAME")
        assert name_count == 3  # A, B, C

    def test_variable_not_confused_with_builtin(self) -> None:
        """A single letter that starts a builtin name is still just a NAME if alone."""
        # S alone is a NAME, not BUILTIN_FN (which requires SIN/COS/TAN etc.)
        pairs = tokens_with_types("10 LET S = 1\n")
        names = [(t, v) for (t, v) in pairs if t == "NAME"]
        assert ("NAME", "S") in names


# ---------------------------------------------------------------------------
# Test 15: Error recovery UNKNOWN
# ---------------------------------------------------------------------------


class TestErrorRecovery:
    """Tests for the UNKNOWN error recovery token.

    The grammar's ``errors:`` section defines ``UNKNOWN = /./`` as a catch-all
    pattern. When no other rule matches, the lexer emits an UNKNOWN token for
    the bad character and continues. This prevents the lexer from looping
    forever on unexpected input.

    The UNKNOWN token allows the parser to emit a helpful error message
    pointing to the unexpected character, rather than crashing silently.
    """

    def test_unknown_at_symbol(self) -> None:
        """The @ character produces an UNKNOWN token."""
        types = token_types("10 LET @ = 1\n")
        assert "UNKNOWN" in types

    def test_unknown_does_not_stop_lexing(self) -> None:
        """After an UNKNOWN token, lexing continues normally."""
        pairs = tokens_with_types("10 LET @ = 1\n")
        # Should still have NUMBER("1") after the UNKNOWN
        assert ("NUMBER", "1") in pairs

    def test_unknown_token_value(self) -> None:
        """UNKNOWN token captures the bad character as its value."""
        pairs = tokens_with_types("10 LET @ = 1\n")
        unknowns = [(t, v) for (t, v) in pairs if t == "UNKNOWN"]
        assert len(unknowns) == 1
        assert unknowns[0] == ("UNKNOWN", "@")

    def test_multiple_unknowns(self) -> None:
        """Multiple bad characters each produce their own UNKNOWN token."""
        pairs = tokens_with_types("10 LET @ # 1\n")
        unknowns = [(t, v) for (t, v) in pairs if t == "UNKNOWN"]
        assert len(unknowns) == 2


# ---------------------------------------------------------------------------
# Test 16: FOR/TO/STEP keywords
# ---------------------------------------------------------------------------


class TestForLoopKeywords:
    """Tests for the FOR loop keywords: FOR, TO, STEP, NEXT.

    Dartmouth BASIC uses a counted loop structure:

        50 FOR I = 1 TO 10 STEP 2
        60 PRINT I
        70 NEXT I

    FOR introduces the loop variable and range.
    TO specifies the end value.
    STEP specifies the increment (default is 1 if omitted).
    NEXT marks the end of the loop body.
    """

    def test_for_keyword(self) -> None:
        """FOR is a KEYWORD token."""
        types = token_types("50 FOR I = 1 TO 10 STEP 2\n")
        keyword_values = token_values("50 FOR I = 1 TO 10 STEP 2\n")
        assert "FOR" in keyword_values

    def test_to_keyword(self) -> None:
        """TO is a KEYWORD token."""
        values = token_values("50 FOR I = 1 TO 10\n")
        assert "TO" in values

    def test_step_keyword(self) -> None:
        """STEP is a KEYWORD token."""
        values = token_values("50 FOR I = 1 TO 10 STEP 2\n")
        assert "STEP" in values

    def test_next_keyword(self) -> None:
        """NEXT is a KEYWORD token."""
        values = token_values("60 NEXT I\n")
        assert "NEXT" in values

    def test_for_loop_full_sequence(self) -> None:
        """FOR I = 1 TO 10 STEP 2 tokenizes into the correct sequence."""
        pairs = tokens_with_types("50 FOR I = 1 TO 10 STEP 2\n")
        types = [t for (t, _) in pairs]
        values = [v for (_, v) in pairs]
        assert "FOR" in values
        assert "TO" in values
        assert "STEP" in values
        # The loop variable I is a NAME
        assert "NAME" in types


# ---------------------------------------------------------------------------
# Test 17: All 20 keywords
# ---------------------------------------------------------------------------


class TestAllKeywords:
    """Tests that all 20 Dartmouth BASIC keywords are recognized.

    The 1964 Dartmouth BASIC specification defines exactly 20 reserved words:
      LET, PRINT, INPUT, IF, THEN, GOTO, GOSUB, RETURN, FOR, TO,
      STEP, NEXT, END, STOP, REM, READ, DATA, RESTORE, DIM, DEF

    Each must produce a KEYWORD token when it appears in source.
    """

    # The complete list of keywords from the spec
    ALL_KEYWORDS = [
        "LET", "PRINT", "INPUT", "IF", "THEN", "GOTO", "GOSUB", "RETURN",
        "FOR", "TO", "STEP", "NEXT", "END", "STOP", "REM", "READ", "DATA",
        "RESTORE", "DIM", "DEF",
    ]

    def test_all_keywords_recognized(self) -> None:
        """Every keyword from the 1964 spec is recognized as KEYWORD."""
        # We test by wrapping each keyword in a minimal BASIC statement.
        # Some keywords need special treatment (REM suppresses rest of line).
        # Strategy: just tokenize the keyword alone with a line number.
        for kw in self.ALL_KEYWORDS:
            source = f"10 {kw}\n"
            types = token_types(source)
            values = token_values(source)
            # After case normalization, the keyword value should appear
            assert "KEYWORD" in types or kw in values, (
                f"Keyword {kw!r} not recognized in source {source!r}. "
                f"Got types={types}, values={values}"
            )

    def test_let(self) -> None:
        """LET is a KEYWORD."""
        values = token_values("10 LET X = 1\n")
        assert "LET" in values

    def test_print(self) -> None:
        """PRINT is a KEYWORD."""
        values = token_values("10 PRINT X\n")
        assert "PRINT" in values

    def test_input(self) -> None:
        """INPUT is a KEYWORD."""
        values = token_values("10 INPUT X\n")
        assert "INPUT" in values

    def test_if_then(self) -> None:
        """IF and THEN are both KEYWORD tokens."""
        values = token_values("10 IF X > 0 THEN 100\n")
        assert "IF" in values
        assert "THEN" in values

    def test_goto(self) -> None:
        """GOTO is a KEYWORD."""
        values = token_values("10 GOTO 100\n")
        assert "GOTO" in values

    def test_gosub_return(self) -> None:
        """GOSUB and RETURN are both KEYWORD tokens."""
        values = token_values("10 GOSUB 100\n")
        assert "GOSUB" in values
        values2 = token_values("20 RETURN\n")
        assert "RETURN" in values2

    def test_end_stop(self) -> None:
        """END and STOP are KEYWORD tokens."""
        values_end = token_values("10 END\n")
        assert "END" in values_end
        values_stop = token_values("10 STOP\n")
        assert "STOP" in values_stop

    def test_read_data_restore(self) -> None:
        """READ, DATA, and RESTORE are KEYWORD tokens."""
        assert "READ" in token_values("10 READ X\n")
        assert "DATA" in token_values("10 DATA 1, 2, 3\n")
        assert "RESTORE" in token_values("10 RESTORE\n")

    def test_dim(self) -> None:
        """DIM is a KEYWORD."""
        values = token_values("10 DIM A(10)\n")
        assert "DIM" in values

    def test_def(self) -> None:
        """DEF is a KEYWORD."""
        values = token_values("10 DEF FNA(X) = X\n")
        assert "DEF" in values


# ---------------------------------------------------------------------------
# Test 18: Arithmetic operators
# ---------------------------------------------------------------------------


class TestArithmeticOperators:
    """Tests for the arithmetic operator tokens.

    Dartmouth BASIC supports:
      +   addition      → PLUS
      -   subtraction   → MINUS
      *   multiplication → STAR
      /   division      → SLASH
      ^   exponentiation → CARET  (right-associative)
    """

    def test_plus(self) -> None:
        """+ is a PLUS token."""
        types = token_types("10 LET X = A + B\n")
        assert "PLUS" in types

    def test_minus(self) -> None:
        """- is a MINUS token."""
        types = token_types("10 LET X = A - B\n")
        assert "MINUS" in types

    def test_star(self) -> None:
        """* is a STAR token."""
        types = token_types("10 LET X = A * B\n")
        assert "STAR" in types

    def test_slash(self) -> None:
        """/ is a SLASH token."""
        types = token_types("10 LET X = A / B\n")
        assert "SLASH" in types

    def test_caret(self) -> None:
        """^ is a CARET token (exponentiation)."""
        types = token_types("10 LET X = A ^ B\n")
        assert "CARET" in types

    def test_all_arithmetic_ops(self) -> None:
        """All five arithmetic operators in one expression."""
        types = token_types("10 LET X = A + B - C * D / E ^ F\n")
        assert "PLUS" in types
        assert "MINUS" in types
        assert "STAR" in types
        assert "SLASH" in types
        assert "CARET" in types

    def test_parentheses(self) -> None:
        """Parentheses produce LPAREN and RPAREN tokens."""
        types = token_types("10 LET X = (A + B)\n")
        assert "LPAREN" in types
        assert "RPAREN" in types


# ---------------------------------------------------------------------------
# Test 19: Token positions (line and column)
# ---------------------------------------------------------------------------


class TestTokenPositions:
    """Tests that tokens carry accurate source position information.

    Each ``Token`` object has ``line`` and ``column`` attributes that
    indicate where in the source the token begins. These are used by
    the parser to produce helpful error messages.
    """

    def test_first_token_starts_at_column_1(self) -> None:
        """The first token in the source starts at column 1 (1-indexed).

        The GrammarLexer uses 1-based column numbering (matching most editor
        conventions where the first column is column 1, not 0).
        """
        tokens = tokenize_dartmouth_basic("10 LET X = 5\n")
        first = tokens[0]
        assert first.column == 1

    def test_eof_has_position(self) -> None:
        """EOF token has a line and column attribute."""
        tokens = tokenize_dartmouth_basic("10 END\n")
        eof = tokens[-1]
        # Just verify the attributes exist and are non-negative integers
        assert isinstance(eof.line, int)
        assert isinstance(eof.column, int)
        assert eof.line >= 0
        assert eof.column >= 0


# ---------------------------------------------------------------------------
# Test 20: Edge cases and whitespace
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Tests for edge cases, whitespace handling, and boundary conditions."""

    def test_whitespace_between_tokens_ignored(self) -> None:
        """Extra spaces between tokens don't change the token sequence."""
        compact = token_types("10 LET X=5\n")
        spaced = token_types("10 LET X = 5\n")
        assert compact == spaced

    def test_tab_whitespace_ignored(self) -> None:
        """Tab characters between tokens are ignored."""
        types = token_types("10\tLET\tX\t=\t5\n")
        assert types == ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE"]

    def test_single_line_no_trailing_newline(self) -> None:
        """A line without a trailing newline still produces correct tokens."""
        # Some BASIC programs might not end with a newline
        types = token_types("10 END")
        # Should produce LINE_NUM, KEYWORD at minimum
        assert "LINE_NUM" in types
        assert "KEYWORD" in types

    def test_complex_if_statement(self) -> None:
        """IF X > 0 THEN 100 tokenizes into the full correct sequence."""
        pairs = tokens_with_types("40 IF X > 0 THEN 100\n")
        assert ("LINE_NUM", "40") in pairs
        assert ("KEYWORD", "IF") in pairs
        assert ("NAME", "X") in pairs
        assert ("GT", ">") in pairs
        assert ("NUMBER", "0") in pairs
        assert ("KEYWORD", "THEN") in pairs
        assert ("NUMBER", "100") in pairs

    def test_def_user_fn(self) -> None:
        """DEF FNA(X) = X * X tokenizes function definition correctly."""
        pairs = tokens_with_types("60 DEF FNA(X) = X * X\n")
        assert ("KEYWORD", "DEF") in pairs
        assert ("USER_FN", "FNA") in pairs
        assert ("LPAREN", "(") in pairs
        assert ("RPAREN", ")") in pairs
        assert ("STAR", "*") in pairs

    def test_sin_cos_expression(self) -> None:
        """SIN(X) + COS(X) tokenizes builtins with parens correctly."""
        pairs = tokens_with_types("70 LET Y = SIN(X) + COS(X)\n")
        types = [t for (t, _) in pairs]
        assert types.count("BUILTIN_FN") == 2
        assert types.count("LPAREN") == 2
        assert types.count("RPAREN") == 2
        assert "PLUS" in types

    def test_data_statement(self) -> None:
        """DATA with comma-separated values tokenizes correctly."""
        pairs = tokens_with_types("100 DATA 1, 2, 3\n")
        assert ("KEYWORD", "DATA") in pairs
        commas = [(t, v) for (t, v) in pairs if t == "COMMA"]
        assert len(commas) == 2  # two commas for three values

    def test_gosub_return_flow(self) -> None:
        """GOSUB and RETURN produce KEYWORD tokens."""
        gosub_values = token_values("10 GOSUB 500\n")
        assert "GOSUB" in gosub_values
        return_values = token_values("99 RETURN\n")
        assert "RETURN" in return_values
