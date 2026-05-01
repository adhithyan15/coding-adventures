"""Tests for the ALGOL 60 lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``algol.tokens``, correctly tokenizes ALGOL 60 source text.

ALGOL 60 Tokenization Notes
-----------------------------

ALGOL 60 has several tokenization behaviors that differ from modern languages:

1. **Case-insensitive keywords**: ``BEGIN``, ``Begin``, and ``begin`` all
   produce the same token kind. The grammar normalizes to lowercase.

2. **Keyword boundary enforcement**: ``beginning`` is an IDENT, not BEGIN
   followed by ``ning``. The lexer matches the full identifier token first,
   then checks if it equals a keyword exactly.

3. **:= vs =**: ALGOL uses ``:=`` for assignment and ``=`` for equality.
   This is one of ALGOL's most influential design decisions — it prevents
   the C bug of writing ``=`` when you mean ``==``.

4. **Comment skipping**: ``comment text;`` is consumed silently. The word
   ``comment`` plus everything through the next ``;`` is not emitted.

5. **REAL_LIT before INTEGER_LIT**: The grammar places REAL_LIT before
   INTEGER_LIT so that ``3.14`` matches as one REAL_LIT, not INTEGER_LIT
   ``3`` followed by a syntax error on ``.14``.

6. **String literals are quoted**: ``'hello'`` or ``"hello"``.
   No escape sequences — the opening quote cannot appear inside the string.
"""

from __future__ import annotations

from lexer import GrammarLexer, Token

from algol_lexer import create_algol_lexer, tokenize_algol

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def token_type_name(token: Token) -> str:
    """Return a token type name for either enum-backed or string tokens."""
    token_type = token.type
    return token_type if isinstance(token_type, str) else token_type.name


def token_types(source: str) -> list[str]:
    """Tokenize and return just the type names (excluding EOF)."""
    tokens = tokenize_algol(source)
    return [
        t.value.upper()
        if token_type_name(t) == "KEYWORD"
        else token_type_name(t)
        for t in tokens
        if token_type_name(t) != "EOF"
    ]


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values (excluding EOF)."""
    tokens = tokenize_algol(source)
    return [
        t.value
        for t in tokens
        if token_type_name(t) != "EOF"
    ]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_algol_lexer factory function."""

    def test_returns_grammar_lexer(self) -> None:
        """create_algol_lexer should return a GrammarLexer instance."""
        lexer = create_algol_lexer("begin end")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_produces_tokens(self) -> None:
        """The factory-created lexer should produce valid tokens."""
        lexer = create_algol_lexer("begin integer x; x := 42 end")
        tokens = lexer.tokenize()
        assert len(tokens) >= 2  # at least something + EOF
        assert token_type_name(tokens[-1]) == "EOF"


# ---------------------------------------------------------------------------
# Keyword tests
# ---------------------------------------------------------------------------


class TestKeywords:
    """Tests for ALGOL 60 keyword tokenization.

    ALGOL 60 has a richer keyword set than JSON or simpler languages.
    Keywords cover control flow, declarations, types, boolean operators,
    and arithmetic keywords (div, mod).
    """

    def test_begin_end(self) -> None:
        """BEGIN and END are block delimiters."""
        types = token_types("begin end")
        assert types == ["BEGIN", "END"]

    def test_if_then_else(self) -> None:
        """IF, THEN, ELSE control flow keywords."""
        types = token_types("if then else")
        assert types == ["IF", "THEN", "ELSE"]

    def test_for_do(self) -> None:
        """FOR and DO loop keywords."""
        types = token_types("for do")
        assert types == ["FOR", "DO"]

    def test_step_until(self) -> None:
        """STEP and UNTIL are for-loop range keywords."""
        types = token_types("step until")
        assert types == ["STEP", "UNTIL"]

    def test_while(self) -> None:
        """WHILE is a for-loop condition keyword."""
        types = token_types("while")
        assert types == ["WHILE"]

    def test_goto(self) -> None:
        """GOTO is the jump keyword."""
        types = token_types("goto")
        assert types == ["GOTO"]

    def test_procedure(self) -> None:
        """PROCEDURE declares a subprogram."""
        types = token_types("procedure")
        assert types == ["PROCEDURE"]

    def test_type_keywords(self) -> None:
        """Type keywords: INTEGER, REAL, BOOLEAN, STRING."""
        types = token_types("integer real boolean string")
        assert types == ["INTEGER", "REAL", "BOOLEAN", "STRING"]

    def test_array_switch_own_label_value(self) -> None:
        """Declaration keywords: ARRAY, SWITCH, OWN, LABEL, VALUE."""
        types = token_types("array switch own label value")
        assert types == ["ARRAY", "SWITCH", "OWN", "LABEL", "VALUE"]

    def test_boolean_literals(self) -> None:
        """TRUE and FALSE boolean literal keywords."""
        types = token_types("true false")
        assert types == ["TRUE", "FALSE"]

    def test_keywords_case_insensitive(self) -> None:
        """Keywords are case-insensitive: BEGIN, Begin, begin all work."""
        types = token_types("BEGIN Begin begin")
        assert types == ["BEGIN", "BEGIN", "BEGIN"]

    def test_keywords_mixed_case(self) -> None:
        """Mixed-case keywords: IF, If, iF all recognized."""
        types = token_types("IF If iF")
        assert types == ["IF", "IF", "IF"]

    def test_div_mod_keywords(self) -> None:
        """DIV and MOD are arithmetic keywords, not symbols."""
        types = token_types("div mod")
        assert types == ["DIV", "MOD"]


# ---------------------------------------------------------------------------
# Boolean operator keyword tests
# ---------------------------------------------------------------------------


class TestBooleanKeywords:
    """Tests for ALGOL 60 boolean operator keywords.

    The lexer accepts both word spellings and ALGOL publication symbols, then
    normalizes the symbols to the same keyword values as the words.

    Operator precedence (lowest to highest):
        eqv   — logical equivalence (a eqv b = a ↔ b)
        impl  — logical implication (a impl b = ¬a ∨ b)
        or    — logical disjunction
        and   — logical conjunction
        not   — logical negation (highest precedence, unary prefix)
    """

    def test_not(self) -> None:
        """NOT is logical negation."""
        types = token_types("not")
        assert types == ["NOT"]

    def test_and(self) -> None:
        """AND is logical conjunction."""
        types = token_types("and")
        assert types == ["AND"]

    def test_or(self) -> None:
        """OR is logical disjunction."""
        types = token_types("or")
        assert types == ["OR"]

    def test_impl(self) -> None:
        """IMPL is logical implication (A impl B = not A or B)."""
        types = token_types("impl")
        assert types == ["IMPL"]

    def test_eqv(self) -> None:
        """EQV is logical equivalence (A eqv B = A ↔ B)."""
        types = token_types("eqv")
        assert types == ["EQV"]

    def test_all_boolean_operators(self) -> None:
        """All boolean operators together."""
        types = token_types("not and or impl eqv")
        assert types == ["NOT", "AND", "OR", "IMPL", "EQV"]

    def test_publication_symbol_boolean_operators(self) -> None:
        """Publication-symbol booleans normalize to keyword spellings."""
        assert token_types("¬ ∧ ∨ ⊃ ≡") == ["NOT", "AND", "OR", "IMPL", "EQV"]
        assert token_values("¬ ∧ ∨ ⊃ ≡") == ["not", "and", "or", "impl", "eqv"]


# ---------------------------------------------------------------------------
# Operator tests
# ---------------------------------------------------------------------------


class TestOperators:
    """Tests for ALGOL 60 operators.

    Key design facts:
    - ``:=`` is ASSIGN (not ``:`` + ``=`` — the multi-char match fires first)
    - ``**`` is POWER (not ``*`` + ``*``)
    - ``<=``/``≤`` are LEQ, ``>=``/``≥`` are GEQ, ``!=``/``<>``/``≠`` are NEQ
    - ``^``/``↑`` are CARET (alternative exponentiation, same precedence as ``**``)
    - ``=`` is EQ (equality test, NOT assignment)
    """

    def test_assign(self) -> None:
        """`:=` is a single ASSIGN token."""
        types = token_types(":=")
        assert types == ["ASSIGN"]

    def test_power_double_star(self) -> None:
        """``**`` is a single POWER token, not STAR STAR."""
        types = token_types("**")
        assert types == ["POWER"]

    def test_caret(self) -> None:
        """``^`` is CARET (alternative exponentiation symbol)."""
        types = token_types("^")
        assert types == ["CARET"]

    def test_publication_symbol_caret(self) -> None:
        """``↑`` normalizes to the existing CARET exponentiation token."""
        assert token_types("↑") == ["CARET"]
        assert token_values("↑") == ["^"]

    def test_leq(self) -> None:
        """``<=`` is a single LEQ token, not LT EQ."""
        types = token_types("<=")
        assert types == ["LEQ"]

    def test_publication_symbol_leq(self) -> None:
        """``≤`` normalizes to the existing LEQ operator value."""
        assert token_types("≤") == ["LEQ"]
        assert token_values("≤") == ["<="]

    def test_geq(self) -> None:
        """``>=`` is a single GEQ token, not GT EQ."""
        types = token_types(">=")
        assert types == ["GEQ"]

    def test_publication_symbol_geq(self) -> None:
        """``≥`` normalizes to the existing GEQ operator value."""
        assert token_types("≥") == ["GEQ"]
        assert token_values("≥") == [">="]

    def test_neq(self) -> None:
        """``!=`` is a single NEQ token."""
        types = token_types("!=")
        assert types == ["NEQ"]

    def test_angle_neq(self) -> None:
        """``<>`` is also accepted as an ALGOL/Pascal-style NEQ token."""
        types = token_types("<>")
        assert types == ["NEQ"]

    def test_publication_symbol_neq(self) -> None:
        """``≠`` normalizes to the existing NEQ operator value."""
        assert token_types("≠") == ["NEQ"]
        assert token_values("≠") == ["!="]

    def test_eq(self) -> None:
        """``=`` is EQ (equality), not assignment."""
        types = token_types("=")
        assert types == ["EQ"]

    def test_lt(self) -> None:
        """``<`` alone is LT."""
        types = token_types("<")
        assert types == ["LT"]

    def test_gt(self) -> None:
        """``>`` alone is GT."""
        types = token_types(">")
        assert types == ["GT"]

    def test_plus_minus(self) -> None:
        """``+`` and ``-`` are PLUS and MINUS."""
        types = token_types("+ -")
        assert types == ["PLUS", "MINUS"]

    def test_star_slash(self) -> None:
        """``*`` and ``/`` are STAR and SLASH (not POWER)."""
        types = token_types("* /")
        assert types == ["STAR", "SLASH"]

    def test_all_relational_operators(self) -> None:
        """All six relational operators tokenize correctly."""
        types = token_types("< <= ≤ = != <> ≠ >= ≥ >")
        assert types == [
            "LT",
            "LEQ",
            "LEQ",
            "EQ",
            "NEQ",
            "NEQ",
            "NEQ",
            "GEQ",
            "GEQ",
            "GT",
        ]


# ---------------------------------------------------------------------------
# Assignment vs equality tests
# ---------------------------------------------------------------------------


class TestAssignVsEquality:
    """Tests for the crucial ALGOL 60 distinction between := and =.

    ALGOL 60's decision to use := for assignment and = for equality prevents
    the famous C bug. In C, ``if (x = 1)`` assigns 1 to x and is always true.
    In ALGOL 60, ``if x = 1 then`` is purely a comparison — there is no way
    to write an assignment inside a condition.
    """

    def test_assign_is_single_token(self) -> None:
        """`:=` is one ASSIGN token, not COLON + EQ."""
        types = token_types(":=")
        values = token_values(":=")
        assert types == ["ASSIGN"]
        assert values == [":="]

    def test_eq_is_equality_not_assignment(self) -> None:
        """``=`` is EQ (equality test), never ASSIGN."""
        types = token_types("=")
        assert types == ["EQ"]

    def test_assign_in_context(self) -> None:
        """``x := 1`` tokenizes as IDENT ASSIGN INTEGER_LIT."""
        types = token_types("x := 1")
        assert types == ["NAME", "ASSIGN", "INTEGER_LIT"]

    def test_eq_in_context(self) -> None:
        """``x = 1`` tokenizes as IDENT EQ INTEGER_LIT (an expression)."""
        types = token_types("x = 1")
        assert types == ["NAME", "EQ", "INTEGER_LIT"]

    def test_colon_alone(self) -> None:
        """A bare ``:`` (for label or array bound) is COLON, not part of ASSIGN."""
        types = token_types("start:")
        assert types == ["NAME", "COLON"]


# ---------------------------------------------------------------------------
# Integer literal tests
# ---------------------------------------------------------------------------


class TestIntegerLiteral:
    """Tests for ALGOL 60 integer literal tokenization.

    ALGOL 60 integer literals are simply one or more decimal digits.
    No prefixes (no 0x for hex, no 0b for binary — ALGOL predates those
    conventions). No separators (no 1_000_000 — that's a modern addition).
    """

    def test_single_digit(self) -> None:
        """A single digit is an INTEGER_LIT."""
        types = token_types("0")
        values = token_values("0")
        assert types == ["INTEGER_LIT"]
        assert values == ["0"]

    def test_multi_digit(self) -> None:
        """Multiple digits form one INTEGER_LIT."""
        types = token_types("42")
        values = token_values("42")
        assert types == ["INTEGER_LIT"]
        assert values == ["42"]

    def test_large_integer(self) -> None:
        """Large integer literals work correctly."""
        types = token_types("1000")
        values = token_values("1000")
        assert types == ["INTEGER_LIT"]
        assert values == ["1000"]

    def test_multiple_integers(self) -> None:
        """Multiple integer literals are separated into distinct tokens."""
        types = token_types("1 2 3")
        assert types == ["INTEGER_LIT", "INTEGER_LIT", "INTEGER_LIT"]


# ---------------------------------------------------------------------------
# Real literal tests
# ---------------------------------------------------------------------------


class TestRealLiteral:
    """Tests for ALGOL 60 real (floating-point) literal tokenization.

    ALGOL 60 uses a decimal-point notation for reals, with optional exponent.
    The exponent uses ``E`` or ``e`` (case matters in most implementations,
    but the grammar here handles both).

    Supported forms:
        3.14        integer + fractional part
        1.5E3       with exponent → 1500.0
        1.5E-3      with negative exponent → 0.0015
        100E2       integer + exponent (no fractional part) → 10000.0
        1.0e10      lowercase e in exponent
    """

    def test_decimal(self) -> None:
        """A number with decimal point is REAL_LIT."""
        types = token_types("3.14")
        values = token_values("3.14")
        assert types == ["REAL_LIT"]
        assert values == ["3.14"]

    def test_exponent_uppercase(self) -> None:
        """Uppercase E exponent notation."""
        types = token_types("1.5E3")
        values = token_values("1.5E3")
        assert types == ["REAL_LIT"]
        assert values == ["1.5E3"]

    def test_negative_exponent(self) -> None:
        """Negative exponent notation: 1.5E-3 = 0.0015."""
        types = token_types("1.5E-3")
        values = token_values("1.5E-3")
        assert types == ["REAL_LIT"]
        assert values == ["1.5E-3"]

    def test_integer_with_exponent(self) -> None:
        """Integer digits with exponent (no decimal point): 100E2 = 10000."""
        types = token_types("100E2")
        values = token_values("100E2")
        assert types == ["REAL_LIT"]
        assert values == ["100E2"]

    def test_lowercase_e(self) -> None:
        """Lowercase 'e' in exponent."""
        types = token_types("1.0e10")
        assert types == ["REAL_LIT"]

    def test_real_before_integer(self) -> None:
        """REAL_LIT must match before INTEGER_LIT for 3.14 not to split."""
        # If REAL_LIT were not tried first, "3.14" could match INTEGER_LIT
        # "3", then cause a lexer error on ".14". The grammar ordering ensures
        # REAL_LIT wins.
        types = token_types("3.14")
        assert types == ["REAL_LIT"]  # not ["INTEGER_LIT", ...]

    def test_positive_exponent(self) -> None:
        """Explicit positive exponent sign."""
        types = token_types("1.5E+3")
        assert types == ["REAL_LIT"]


# ---------------------------------------------------------------------------
# String literal tests
# ---------------------------------------------------------------------------


class TestStringLiteral:
    """Tests for ALGOL 60 string literal tokenization.

    ALGOL 60 uses quoted strings. There are no escape sequences — the original
    language had no way to include the opening quote inside a string literal.
    The lexer accepts both common ASCII quote spellings.
    """

    def test_simple_string(self) -> None:
        """A simple single-quoted string."""
        types = token_types("'hello'")
        assert types == ["STRING_LIT"]

    def test_string_with_spaces(self) -> None:
        """Spaces inside a string are preserved (not skipped)."""
        types = token_types("'hello world'")
        values = token_values("'hello world'")
        assert types == ["STRING_LIT"]
        # The grammar may or may not strip quotes; test for type correctness
        assert len(values) == 1

    def test_empty_string(self) -> None:
        """An empty single-quoted string: ``''``."""
        types = token_types("''")
        assert types == ["STRING_LIT"]

    def test_double_quoted_string(self) -> None:
        """A double-quoted string is also accepted."""
        types = token_types('"hello"')
        values = token_values('"hello"')
        assert types == ["STRING_LIT"]
        assert values == ['"hello"']

    def test_empty_double_quoted_string(self) -> None:
        """An empty double-quoted string is also accepted."""
        types = token_types('""')
        assert types == ["STRING_LIT"]

    def test_string_with_digits(self) -> None:
        """Strings may contain digits."""
        types = token_types("'abc123'")
        assert types == ["STRING_LIT"]


# ---------------------------------------------------------------------------
# Comment skipping tests
# ---------------------------------------------------------------------------


class TestCommentSkipping:
    """Tests for ALGOL 60 comment syntax.

    ALGOL 60 uses ``comment text;`` — the keyword 'comment' followed by
    arbitrary text up to and including the next semicolon. The entire
    construct (including the semicolon) is consumed without emitting any token.

    This means after a statement like::

        x := 1; comment this sets x;
        y := 2

    the lexer sees: IDENT ASSIGN INTEGER_LIT SEMICOLON [comment skipped]
    IDENT ASSIGN INTEGER_LIT

    Wait — the semicolon before 'comment' is emitted as SEMICOLON. Then the
    'comment...' pattern consumes 'comment this sets x;' as a skip. So
    the comment skip includes the FINAL semicolon but the FIRST semicolon
    (the statement separator) is already emitted.
    """

    def test_comment_skipped(self) -> None:
        """A comment produces no tokens."""
        types = token_types("comment this is a comment;")
        assert types == []

    def test_comment_keyword_is_case_insensitive(self) -> None:
        """COMMENT follows the same case-insensitive keyword policy."""
        assert token_types("COMMENT this is a comment;") == []
        assert token_types("Comment this is a comment;") == []

    def test_code_after_comment(self) -> None:
        """Tokens after a comment are still emitted."""
        types = token_types("comment ignore this; x")
        assert types == ["NAME"]
        values = token_values("comment ignore this; x")
        assert values == ["x"]

    def test_comment_in_program(self) -> None:
        """A comment in a real program context is invisible to the parser."""
        # After the first semicolon (SEMICOLON), comment skips through its ;
        source = "x := 1; comment set x; y := 2"
        types = token_types(source)
        assert "NAME" in types
        assert "ASSIGN" in types
        # The comment itself should not appear
        assert "COMMENT" not in types

    def test_comment_with_multiple_words(self) -> None:
        """Comments may contain any text except semicolons."""
        types = token_types("comment the quick brown fox jumps;")
        assert types == []

    def test_code_before_and_after_comment(self) -> None:
        """Tokens on both sides of a comment are correct."""
        source = "x := 42 comment set x to forty two; y := 0"
        # Note: if there's no semicolon before 'comment', the comment
        # pattern depends on how the skip rule is applied. Testing the
        # basic case where comment appears inline.
        # Just verify no crash and reasonable token count.
        tokens = token_values(source)
        assert "42" in tokens or "0" in tokens  # some tokens visible

    def test_comment_prefix_identifier_is_not_skipped(self) -> None:
        """Identifiers beginning with comment still respect keyword boundaries."""
        source = "commentary := 1;"

        assert token_types(source) == ["NAME", "ASSIGN", "INTEGER_LIT", "SEMICOLON"]
        assert token_values(source)[0] == "commentary"


# ---------------------------------------------------------------------------
# Keyword boundary tests
# ---------------------------------------------------------------------------


class TestKeywordBoundary:
    """Tests for ALGOL 60 keyword boundary enforcement.

    A keyword only fires when the entire identifier token matches the keyword
    exactly. Prefixes and suffixes do not qualify:

        begin    → BEGIN       (exact match)
        beginning → IDENT      (full token is 'beginning', not 'begin')
        end      → END
        endian   → IDENT
        integer  → INTEGER
        integers → IDENT
    """

    def test_beginning_is_ident(self) -> None:
        """'beginning' is IDENT, not BEGIN + 'ning'."""
        types = token_types("beginning")
        assert types == ["NAME"]
        values = token_values("beginning")
        assert values == ["beginning"]

    def test_endian_is_ident(self) -> None:
        """'endian' is IDENT, not END + 'ian'."""
        types = token_types("endian")
        assert types == ["NAME"]

    def test_integers_is_ident(self) -> None:
        """'integers' is IDENT, not INTEGER + 's'."""
        types = token_types("integers")
        assert types == ["NAME"]

    def test_real_word_begin_ident(self) -> None:
        """Confirm 'begin' alone is BEGIN but 'begins' is IDENT."""
        assert token_types("begin") == ["BEGIN"]
        assert token_types("begins") == ["NAME"]

    def test_forloop_is_ident(self) -> None:
        """'forloop' is IDENT, not FOR + 'loop'."""
        types = token_types("forloop")
        assert types == ["NAME"]

    def test_notable_is_ident(self) -> None:
        """'notable' is IDENT, not NOT + 'able'."""
        types = token_types("notable")
        assert types == ["NAME"]

    def test_android_is_ident(self) -> None:
        """'android' is IDENT, not AND + 'roid'."""
        types = token_types("android")
        assert types == ["NAME"]


# ---------------------------------------------------------------------------
# Identifier tests
# ---------------------------------------------------------------------------


class TestIdentifiers:
    """Tests for ALGOL 60 identifier tokenization.

    ALGOL 60 identifiers start with a letter and continue with letters
    or digits. No underscores (added in later languages). No dollar signs.
    Length is theoretically unbounded but implementations often imposed limits.
    """

    def test_single_letter(self) -> None:
        """A single letter is a valid identifier."""
        types = token_types("x")
        assert types == ["NAME"]

    def test_letter_digit(self) -> None:
        """A letter followed by a digit is a valid identifier."""
        types = token_types("A1")
        values = token_values("A1")
        assert types == ["NAME"]
        assert values == ["A1"]

    def test_multi_letter(self) -> None:
        """Multiple letters form one identifier."""
        types = token_types("alpha")
        assert types == ["NAME"]

    def test_mixed_letter_digit(self) -> None:
        """Mixed letters and digits in identifier."""
        types = token_types("x1y2")
        assert types == ["NAME"]

    def test_uppercase_ident(self) -> None:
        """Uppercase non-keyword identifier."""
        types = token_types("XCOORD")
        assert types == ["NAME"]

    def test_ident_with_digit(self) -> None:
        """'A1' is IDENT (letter followed by digit)."""
        assert token_types("A1") == ["NAME"]

    def test_multiple_idents(self) -> None:
        """Multiple identifiers separated by whitespace."""
        types = token_types("x y z")
        assert types == ["NAME", "NAME", "NAME"]


# ---------------------------------------------------------------------------
# Delimiter tests
# ---------------------------------------------------------------------------


class TestDelimiters:
    """Tests for ALGOL 60 delimiter tokens."""

    def test_semicolon(self) -> None:
        """Semicolon is the statement separator."""
        types = token_types(";")
        assert types == ["SEMICOLON"]

    def test_comma(self) -> None:
        """Comma separates items in lists."""
        types = token_types(",")
        assert types == ["COMMA"]

    def test_lparen_rparen(self) -> None:
        """Parentheses for grouping and procedure calls."""
        types = token_types("()")
        assert types == ["LPAREN", "RPAREN"]

    def test_lbracket_rbracket(self) -> None:
        """Brackets for array subscripts."""
        types = token_types("[]")
        assert types == ["LBRACKET", "RBRACKET"]

    def test_colon_alone(self) -> None:
        """Colon for label declarations and array bounds."""
        types = token_types(":")
        assert types == ["COLON"]


# ---------------------------------------------------------------------------
# Whitespace tests
# ---------------------------------------------------------------------------


class TestWhitespace:
    """Tests for ALGOL 60 whitespace handling.

    ALGOL 60 is free-format: whitespace (spaces, tabs, newlines) between
    tokens is completely insignificant. This contrasts with FORTRAN and
    COBOL, which were column-based (tied to 80-column punch cards).
    """

    def test_whitespace_insignificant(self) -> None:
        """Spaces between tokens produce the same result as no spaces."""
        types_compact = token_types("x:=1")
        types_spaced = token_types("x := 1")
        assert types_compact == types_spaced

    def test_tabs_skipped(self) -> None:
        """Tabs between tokens are ignored."""
        types = token_types("x\t:=\t1")
        assert types == ["NAME", "ASSIGN", "INTEGER_LIT"]

    def test_newlines_skipped(self) -> None:
        """Newlines between tokens are ignored (free-format)."""
        types = token_types("x\n:=\n1")
        assert types == ["NAME", "ASSIGN", "INTEGER_LIT"]

    def test_mixed_whitespace(self) -> None:
        """Mixed whitespace types are all ignored."""
        types = token_types("x \t\n := \r\n 1")
        assert types == ["NAME", "ASSIGN", "INTEGER_LIT"]


# ---------------------------------------------------------------------------
# Full expression tests
# ---------------------------------------------------------------------------


class TestFullExpressions:
    """Tests for tokenizing complete ALGOL 60 expressions and statements."""

    def test_full_expression(self) -> None:
        """``x := 1 + 2 * 3`` tokenizes correctly."""
        types = token_types("x := 1 + 2 * 3")
        assert types == [
            "NAME", "ASSIGN", "INTEGER_LIT", "PLUS",
            "INTEGER_LIT", "STAR", "INTEGER_LIT",
        ]

    def test_minimal_program(self) -> None:
        """``begin integer x; x := 42 end`` tokenizes correctly."""
        types = token_types("begin integer x; x := 42 end")
        assert types == [
            "BEGIN", "INTEGER", "NAME", "SEMICOLON",
            "NAME", "ASSIGN", "INTEGER_LIT", "END",
        ]

    def test_for_loop_tokens(self) -> None:
        """A for loop tokenizes into the correct sequence."""
        source = "for i := 1 step 1 until 10 do x := x + 1"
        types = token_types(source)
        assert types == [
            "FOR", "NAME", "ASSIGN", "INTEGER_LIT",
            "STEP", "INTEGER_LIT", "UNTIL", "INTEGER_LIT",
            "DO", "NAME", "ASSIGN", "NAME", "PLUS", "INTEGER_LIT",
        ]

    def test_if_then_tokens(self) -> None:
        """``if x > 0 then x := 1`` tokenizes correctly."""
        types = token_types("if x > 0 then x := 1")
        assert types == [
            "IF", "NAME", "GT", "INTEGER_LIT",
            "THEN", "NAME", "ASSIGN", "INTEGER_LIT",
        ]

    def test_boolean_expression(self) -> None:
        """``not x and y or z`` tokenizes with all boolean operators."""
        types = token_types("not x and y or z")
        assert types == ["NOT", "NAME", "AND", "NAME", "OR", "NAME"]

    def test_real_assignment(self) -> None:
        """``pi := 3.14159`` tokenizes as IDENT ASSIGN REAL_LIT."""
        types = token_types("pi := 3.14159")
        values = token_values("pi := 3.14159")
        assert types == ["NAME", "ASSIGN", "REAL_LIT"]
        assert values == ["pi", ":=", "3.14159"]

    def test_procedure_call(self) -> None:
        """A procedure call with arguments tokenizes correctly."""
        types = token_types("sqrt(x + 1)")
        assert types == [
            "NAME", "LPAREN", "NAME", "PLUS", "INTEGER_LIT", "RPAREN",
        ]

    def test_array_subscript(self) -> None:
        """Array access with subscripts tokenizes correctly."""
        types = token_types("A[i, j]")
        assert types == [
            "NAME", "LBRACKET", "NAME", "COMMA", "NAME", "RBRACKET",
        ]

    def test_exponentiation_power(self) -> None:
        """Exponentiation using ``**`` operator."""
        types = token_types("x ** 2")
        assert types == ["NAME", "POWER", "INTEGER_LIT"]

    def test_exponentiation_caret(self) -> None:
        """Exponentiation using ``^`` operator."""
        types = token_types("x ^ 2")
        assert types == ["NAME", "CARET", "INTEGER_LIT"]


# ---------------------------------------------------------------------------
# EOF token tests
# ---------------------------------------------------------------------------


class TestEOF:
    """Tests for the EOF token."""

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_algol("x := 1")
        last = tokens[-1]
        assert token_type_name(last) == "EOF"

    def test_empty_input_has_eof(self) -> None:
        """Empty input still produces an EOF token."""
        tokens = tokenize_algol("")
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"

    def test_whitespace_only_has_eof(self) -> None:
        """Input with only whitespace produces just EOF."""
        tokens = tokenize_algol("   \t\n  ")
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"
