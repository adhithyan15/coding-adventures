"""Tests for the oct_lexer package.

Oct is a small, statically-typed, 8-bit systems programming language targeting
the Intel 8008 microprocessor (1972).  It exposes the 8008's native word size
(8 bits), port I/O, carry arithmetic, and byte rotations directly in the
language.

This test suite validates that ``tokenize_oct`` correctly classifies all token
kinds, promotes keywords (including intrinsic keywords like ``in``, ``carry``,
and ``parity``), produces the expected token sequence for representative Oct
programs, and handles edge cases such as binary literals, hex literals,
multi-character operators, and comment/whitespace skipping.

Coverage plan:
  - Empty source produces only EOF
  - All keyword tokens (control flow + intrinsics)
  - BIN_LIT, HEX_LIT, INT_LIT literals
  - All single- and multi-character operators
  - All delimiter tokens
  - NAME tokens (identifiers, type names u8/bool)
  - Boolean literals (true, false)
  - Whitespace and comment skipping
  - Multi-token expressions and statements
  - All five complete program examples from OCT00 spec
  - Intrinsic keyword promotion
  - Keyword/identifier boundary (e.g. "invert" is NAME, not "in")
"""

from __future__ import annotations

from lexer import Token

from oct_lexer import tokenize_oct

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _tok_type(tok: Token) -> str:
    """Normalize a token's type to a plain string.

    ``tokenize_oct`` promotes keyword tokens so their ``type`` field is
    already a string (e.g. ``"fn"``, ``"carry"``).  All other tokens keep
    the ``TokenType`` enum value returned by the ``GrammarLexer`` (e.g.
    ``TokenType.LPAREN``).  This helper normalises both cases so tests can
    compare uniformly against string names like ``"LPAREN"`` or ``"NAME"``.
    """
    return tok.type if isinstance(tok.type, str) else tok.type.name


def _types(source: str) -> list[str]:
    """Return the token type list for source, excluding the trailing EOF."""
    tokens = tokenize_oct(source)
    return [_tok_type(t) for t in tokens if _tok_type(t) != "EOF"]


def _values(source: str) -> list[str]:
    """Return the token value list for source, excluding the trailing EOF."""
    tokens = tokenize_oct(source)
    return [t.value for t in tokens if _tok_type(t) != "EOF"]


def _pairs(source: str) -> list[tuple[str, str]]:
    """Return (type, value) pairs for source, excluding EOF."""
    tokens = tokenize_oct(source)
    return [(_tok_type(t), t.value) for t in tokens if _tok_type(t) != "EOF"]


# ---------------------------------------------------------------------------
# Empty / whitespace-only source
# ---------------------------------------------------------------------------

class TestEmptySource:
    """Edge cases: nothing in the source at all."""

    def test_empty_string_produces_only_eof(self) -> None:
        """An empty string tokenizes to a single EOF token."""
        tokens = tokenize_oct("")
        assert len(tokens) == 1
        assert _tok_type(tokens[0]) == "EOF"

    def test_whitespace_only_produces_only_eof(self) -> None:
        """Whitespace is silently skipped; only EOF remains."""
        tokens = tokenize_oct("   \t\n\r\n   ")
        assert len(tokens) == 1
        assert _tok_type(tokens[0]) == "EOF"

    def test_comment_only_produces_only_eof(self) -> None:
        """A lone line comment is silently skipped; only EOF remains."""
        tokens = tokenize_oct("// This is a comment\n")
        assert len(tokens) == 1
        assert _tok_type(tokens[0]) == "EOF"


# ---------------------------------------------------------------------------
# Control-flow keyword tokens
# ---------------------------------------------------------------------------

class TestControlKeywords:
    """Every control-flow keyword is reclassified from NAME to its string value."""

    def test_fn_keyword(self) -> None:
        assert _pairs("fn") == [("fn", "fn")]

    def test_let_keyword(self) -> None:
        assert _pairs("let") == [("let", "let")]

    def test_static_keyword(self) -> None:
        assert _pairs("static") == [("static", "static")]

    def test_if_keyword(self) -> None:
        assert _pairs("if") == [("if", "if")]

    def test_else_keyword(self) -> None:
        assert _pairs("else") == [("else", "else")]

    def test_while_keyword(self) -> None:
        assert _pairs("while") == [("while", "while")]

    def test_loop_keyword(self) -> None:
        assert _pairs("loop") == [("loop", "loop")]

    def test_break_keyword(self) -> None:
        assert _pairs("break") == [("break", "break")]

    def test_return_keyword(self) -> None:
        assert _pairs("return") == [("return", "return")]

    def test_true_keyword(self) -> None:
        assert _pairs("true") == [("true", "true")]

    def test_false_keyword(self) -> None:
        assert _pairs("false") == [("false", "false")]


# ---------------------------------------------------------------------------
# Intrinsic keyword tokens
# ---------------------------------------------------------------------------

class TestIntrinsicKeywords:
    """Intrinsic function names are keywords, not NAME tokens.

    The 8008 exposes special hardware operations (port I/O, carry arithmetic,
    byte rotations) as first-class language intrinsics.  All are reclassified
    from NAME to their keyword string so the parser can distinguish them from
    user-defined function names.
    """

    def test_in_keyword(self) -> None:
        """'in' is an intrinsic keyword (port read), not a NAME."""
        assert _pairs("in") == [("in", "in")]

    def test_out_keyword(self) -> None:
        """'out' is an intrinsic keyword (port write), not a NAME."""
        assert _pairs("out") == [("out", "out")]

    def test_adc_keyword(self) -> None:
        """'adc' (add with carry) is an intrinsic keyword."""
        assert _pairs("adc") == [("adc", "adc")]

    def test_sbb_keyword(self) -> None:
        """'sbb' (subtract with borrow) is an intrinsic keyword."""
        assert _pairs("sbb") == [("sbb", "sbb")]

    def test_rlc_keyword(self) -> None:
        """'rlc' (rotate left circular) is an intrinsic keyword."""
        assert _pairs("rlc") == [("rlc", "rlc")]

    def test_rrc_keyword(self) -> None:
        """'rrc' (rotate right circular) is an intrinsic keyword."""
        assert _pairs("rrc") == [("rrc", "rrc")]

    def test_ral_keyword(self) -> None:
        """'ral' (rotate left through carry) is an intrinsic keyword."""
        assert _pairs("ral") == [("ral", "ral")]

    def test_rar_keyword(self) -> None:
        """'rar' (rotate right through carry) is an intrinsic keyword."""
        assert _pairs("rar") == [("rar", "rar")]

    def test_carry_keyword(self) -> None:
        """'carry' (read carry flag) is an intrinsic keyword."""
        assert _pairs("carry") == [("carry", "carry")]

    def test_parity_keyword(self) -> None:
        """'parity' (read parity flag) is an intrinsic keyword."""
        assert _pairs("parity") == [("parity", "parity")]

    def test_invert_is_name_not_in(self) -> None:
        """'invert' starts with 'in' but is a NAME — keyword match is whole-token."""
        assert _pairs("invert") == [("NAME", "invert")]

    def test_outgoing_is_name_not_out(self) -> None:
        """'outgoing' starts with 'out' but is a NAME."""
        assert _pairs("outgoing") == [("NAME", "outgoing")]

    def test_carry_flag_is_name_not_carry(self) -> None:
        """'carry_flag' contains 'carry' but is a single NAME token."""
        assert _pairs("carry_flag") == [("NAME", "carry_flag")]


# ---------------------------------------------------------------------------
# Literal tokens
# ---------------------------------------------------------------------------

class TestLiterals:
    """BIN_LIT, HEX_LIT, INT_LIT: correct classification and value preservation."""

    def test_decimal_zero(self) -> None:
        assert _pairs("0") == [("INT_LIT", "0")]

    def test_decimal_max_u8(self) -> None:
        assert _pairs("255") == [("INT_LIT", "255")]

    def test_decimal_mid(self) -> None:
        assert _pairs("42") == [("INT_LIT", "42")]

    def test_hex_lowercase(self) -> None:
        """0xff is a valid HEX_LIT — digit letters are case-insensitive."""
        assert _pairs("0xff") == [("HEX_LIT", "0xff")]

    def test_hex_uppercase(self) -> None:
        assert _pairs("0xFF") == [("HEX_LIT", "0xFF")]

    def test_hex_zero(self) -> None:
        assert _pairs("0x00") == [("HEX_LIT", "0x00")]

    def test_hex_max_byte(self) -> None:
        assert _pairs("0xFF") == [("HEX_LIT", "0xFF")]

    def test_hex_not_split(self) -> None:
        """0xFF must lex as one HEX_LIT token, not INT_LIT('0') + NAME('xFF')."""
        tokens = tokenize_oct("0xFF")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "HEX_LIT"

    def test_binary_zero(self) -> None:
        assert _pairs("0b00000000") == [("BIN_LIT", "0b00000000")]

    def test_binary_max(self) -> None:
        assert _pairs("0b11111111") == [("BIN_LIT", "0b11111111")]

    def test_binary_pattern(self) -> None:
        assert _pairs("0b10110011") == [("BIN_LIT", "0b10110011")]

    def test_binary_not_split(self) -> None:
        """0b101 must lex as one BIN_LIT token, not INT_LIT('0') + NAME('b101')."""
        tokens = tokenize_oct("0b101")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "BIN_LIT"


# ---------------------------------------------------------------------------
# Name and type-name tokens
# ---------------------------------------------------------------------------

class TestNameTokens:
    """Identifiers and type names are both NAME tokens."""

    def test_simple_identifier(self) -> None:
        assert _pairs("x") == [("NAME", "x")]

    def test_underscore_identifier(self) -> None:
        assert _pairs("_counter") == [("NAME", "_counter")]

    def test_mixed_case_identifier(self) -> None:
        assert _pairs("myVar2") == [("NAME", "myVar2")]

    def test_type_u8_is_name(self) -> None:
        """'u8' is a NAME token, not a keyword — types are resolved at parse time."""
        assert _pairs("u8") == [("NAME", "u8")]

    def test_type_bool_is_name(self) -> None:
        """'bool' is a NAME token, not a keyword."""
        assert _pairs("bool") == [("NAME", "bool")]


# ---------------------------------------------------------------------------
# Operator tokens
# ---------------------------------------------------------------------------

class TestOperatorTokens:
    """All operator tokens — single- and multi-character."""

    # Multi-character operators
    def test_eq_eq(self) -> None:
        assert _pairs("==") == [("EQ_EQ", "==")]

    def test_neq(self) -> None:
        assert _pairs("!=") == [("NEQ", "!=")]

    def test_leq(self) -> None:
        assert _pairs("<=") == [("LEQ", "<=")]

    def test_geq(self) -> None:
        assert _pairs(">=") == [("GEQ", ">=")]

    def test_land(self) -> None:
        assert _pairs("&&") == [("LAND", "&&")]

    def test_lor(self) -> None:
        assert _pairs("||") == [("LOR", "||")]

    def test_arrow(self) -> None:
        assert _pairs("->") == [("ARROW", "->")]

    # Multi-char must not be split into two single-char tokens
    def test_eq_eq_not_two_eq(self) -> None:
        """'==' is one EQ_EQ token, not two EQ tokens."""
        tokens = tokenize_oct("==")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "EQ_EQ"

    def test_land_not_two_amp(self) -> None:
        """'&&' is one LAND token, not two AMP tokens."""
        tokens = tokenize_oct("&&")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "LAND"

    def test_arrow_not_minus_gt(self) -> None:
        """'->' is one ARROW token, not MINUS + GT."""
        tokens = tokenize_oct("->")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "ARROW"

    # Single-character operators
    def test_plus(self) -> None:
        assert _pairs("+") == [("PLUS", "+")]

    def test_minus(self) -> None:
        assert _pairs("-") == [("MINUS", "-")]

    def test_amp(self) -> None:
        assert _pairs("&") == [("AMP", "&")]

    def test_pipe(self) -> None:
        assert _pairs("|") == [("PIPE", "|")]

    def test_caret(self) -> None:
        assert _pairs("^") == [("CARET", "^")]

    def test_tilde(self) -> None:
        assert _pairs("~") == [("TILDE", "~")]

    def test_bang(self) -> None:
        assert _pairs("!") == [("BANG", "!")]

    def test_lt(self) -> None:
        assert _pairs("<") == [("LT", "<")]

    def test_gt(self) -> None:
        assert _pairs(">") == [("GT", ">")]

    def test_eq(self) -> None:
        assert _pairs("=") == [("EQ", "=")]


# ---------------------------------------------------------------------------
# Delimiter tokens
# ---------------------------------------------------------------------------

class TestDelimiterTokens:
    """All delimiter tokens."""

    def test_lbrace(self) -> None:
        assert _pairs("{") == [("LBRACE", "{")]

    def test_rbrace(self) -> None:
        assert _pairs("}") == [("RBRACE", "}")]

    def test_lparen(self) -> None:
        assert _pairs("(") == [("LPAREN", "(")]

    def test_rparen(self) -> None:
        assert _pairs(")") == [("RPAREN", ")")]

    def test_colon(self) -> None:
        assert _pairs(":") == [("COLON", ":")]

    def test_semicolon(self) -> None:
        assert _pairs(";") == [("SEMICOLON", ";")]

    def test_comma(self) -> None:
        assert _pairs(",") == [("COMMA", ",")]


# ---------------------------------------------------------------------------
# Whitespace and comment skipping
# ---------------------------------------------------------------------------

class TestSkipping:
    """Whitespace and line comments are consumed silently."""

    def test_inline_comment_skipped(self) -> None:
        """A comment at the end of a line does not produce a token."""
        tokens = tokenize_oct("let // comment\n")
        non_eof = [t for t in tokens if _tok_type(t) != "EOF"]
        assert len(non_eof) == 1
        assert _tok_type(non_eof[0]) == "let"

    def test_leading_whitespace_ignored(self) -> None:
        assert _types("   fn") == ["fn"]

    def test_tokens_separated_by_newlines(self) -> None:
        assert _types("let\nstatic") == ["let", "static"]

    def test_mixed_whitespace_between_tokens(self) -> None:
        assert _types("fn\t  \r\nmain") == ["fn", "NAME"]


# ---------------------------------------------------------------------------
# Multi-token expressions and statements
# ---------------------------------------------------------------------------

class TestMultiTokenExpressions:
    """Representative token sequences for common Oct constructs."""

    def test_let_declaration(self) -> None:
        """let x: u8 = 42;"""
        assert _pairs("let x: u8 = 42;") == [
            ("let", "let"),
            ("NAME", "x"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("EQ", "="),
            ("INT_LIT", "42"),
            ("SEMICOLON", ";"),
        ]

    def test_let_declaration_hex(self) -> None:
        """let mask: u8 = 0xFF;"""
        assert _pairs("let mask: u8 = 0xFF;") == [
            ("let", "let"),
            ("NAME", "mask"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("EQ", "="),
            ("HEX_LIT", "0xFF"),
            ("SEMICOLON", ";"),
        ]

    def test_let_declaration_binary(self) -> None:
        """let flags: u8 = 0b00001111;"""
        assert _pairs("let flags: u8 = 0b00001111;") == [
            ("let", "let"),
            ("NAME", "flags"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("EQ", "="),
            ("BIN_LIT", "0b00001111"),
            ("SEMICOLON", ";"),
        ]

    def test_assign_statement(self) -> None:
        """n = n + 1;"""
        assert _pairs("n = n + 1;") == [
            ("NAME", "n"),
            ("EQ", "="),
            ("NAME", "n"),
            ("PLUS", "+"),
            ("INT_LIT", "1"),
            ("SEMICOLON", ";"),
        ]

    def test_fn_signature_no_params(self) -> None:
        """fn main()"""
        assert _pairs("fn main()") == [
            ("fn", "fn"),
            ("NAME", "main"),
            ("LPAREN", "("),
            ("RPAREN", ")"),
        ]

    def test_fn_signature_with_return_type(self) -> None:
        """fn clamp(val: u8, limit: u8) -> u8"""
        assert _pairs("fn clamp(val: u8, limit: u8) -> u8") == [
            ("fn", "fn"),
            ("NAME", "clamp"),
            ("LPAREN", "("),
            ("NAME", "val"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("COMMA", ","),
            ("NAME", "limit"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("RPAREN", ")"),
            ("ARROW", "->"),
            ("NAME", "u8"),
        ]

    def test_if_condition(self) -> None:
        """if n != 0 {"""
        assert _pairs("if n != 0 {") == [
            ("if", "if"),
            ("NAME", "n"),
            ("NEQ", "!="),
            ("INT_LIT", "0"),
            ("LBRACE", "{"),
        ]

    def test_while_with_comparison(self) -> None:
        """while i != 8 {"""
        assert _pairs("while i != 8 {") == [
            ("while", "while"),
            ("NAME", "i"),
            ("NEQ", "!="),
            ("INT_LIT", "8"),
            ("LBRACE", "{"),
        ]

    def test_bitwise_and_expression(self) -> None:
        """checksum = checksum ^ b;"""
        assert _pairs("checksum = checksum ^ b;") == [
            ("NAME", "checksum"),
            ("EQ", "="),
            ("NAME", "checksum"),
            ("CARET", "^"),
            ("NAME", "b"),
            ("SEMICOLON", ";"),
        ]

    def test_not_expression(self) -> None:
        """~x"""
        assert _pairs("~x") == [("TILDE", "~"), ("NAME", "x")]

    def test_logical_not(self) -> None:
        """!flag"""
        assert _pairs("!flag") == [("BANG", "!"), ("NAME", "flag")]

    def test_static_declaration(self) -> None:
        """static counter: u8 = 0;"""
        assert _pairs("static counter: u8 = 0;") == [
            ("static", "static"),
            ("NAME", "counter"),
            ("COLON", ":"),
            ("NAME", "u8"),
            ("EQ", "="),
            ("INT_LIT", "0"),
            ("SEMICOLON", ";"),
        ]

    def test_bool_variable_declaration(self) -> None:
        """let flag: bool = false;"""
        assert _pairs("let flag: bool = false;") == [
            ("let", "let"),
            ("NAME", "flag"),
            ("COLON", ":"),
            ("NAME", "bool"),
            ("EQ", "="),
            ("false", "false"),
            ("SEMICOLON", ";"),
        ]


# ---------------------------------------------------------------------------
# Intrinsic call token sequences
# ---------------------------------------------------------------------------

class TestIntrinsicCallTokens:
    """Token sequences for all 10 intrinsic calls in Oct."""

    def test_in_call(self) -> None:
        """in(0) — read from port 0."""
        assert _pairs("in(0)") == [
            ("in", "in"),
            ("LPAREN", "("),
            ("INT_LIT", "0"),
            ("RPAREN", ")"),
        ]

    def test_out_call(self) -> None:
        """out(1, x) — write x to port 1."""
        assert _pairs("out(1, x)") == [
            ("out", "out"),
            ("LPAREN", "("),
            ("INT_LIT", "1"),
            ("COMMA", ","),
            ("NAME", "x"),
            ("RPAREN", ")"),
        ]

    def test_adc_call(self) -> None:
        """adc(hi_a, hi_b) — add with carry."""
        assert _pairs("adc(hi_a, hi_b)") == [
            ("adc", "adc"),
            ("LPAREN", "("),
            ("NAME", "hi_a"),
            ("COMMA", ","),
            ("NAME", "hi_b"),
            ("RPAREN", ")"),
        ]

    def test_sbb_call(self) -> None:
        """sbb(a, b) — subtract with borrow."""
        assert _pairs("sbb(a, b)") == [
            ("sbb", "sbb"),
            ("LPAREN", "("),
            ("NAME", "a"),
            ("COMMA", ","),
            ("NAME", "b"),
            ("RPAREN", ")"),
        ]

    def test_rlc_call(self) -> None:
        """rlc(x) — rotate left circular."""
        assert _pairs("rlc(x)") == [
            ("rlc", "rlc"),
            ("LPAREN", "("),
            ("NAME", "x"),
            ("RPAREN", ")"),
        ]

    def test_rrc_call(self) -> None:
        """rrc(x) — rotate right circular."""
        assert _pairs("rrc(x)") == [
            ("rrc", "rrc"),
            ("LPAREN", "("),
            ("NAME", "x"),
            ("RPAREN", ")"),
        ]

    def test_ral_call(self) -> None:
        """ral(x) — rotate left through carry."""
        assert _pairs("ral(x)") == [
            ("ral", "ral"),
            ("LPAREN", "("),
            ("NAME", "x"),
            ("RPAREN", ")"),
        ]

    def test_rar_call(self) -> None:
        """rar(x) — rotate right through carry."""
        assert _pairs("rar(x)") == [
            ("rar", "rar"),
            ("LPAREN", "("),
            ("NAME", "x"),
            ("RPAREN", ")"),
        ]

    def test_carry_call(self) -> None:
        """carry() — read carry flag (zero arguments)."""
        assert _pairs("carry()") == [
            ("carry", "carry"),
            ("LPAREN", "("),
            ("RPAREN", ")"),
        ]

    def test_parity_call(self) -> None:
        """parity(b) — read parity flag of b."""
        assert _pairs("parity(b)") == [
            ("parity", "parity"),
            ("LPAREN", "("),
            ("NAME", "b"),
            ("RPAREN", ")"),
        ]


# ---------------------------------------------------------------------------
# Complete Oct programs from OCT00 spec examples
# ---------------------------------------------------------------------------

class TestCompletePrograms:
    """Token counts and key token types for the five spec examples.

    These tests do not assert every token — instead they verify that the
    tokenizer runs without error and produces tokens with the expected key
    kinds, validating that the full program is accepted by the lexer.
    """

    def test_example1_echo_input_to_output(self) -> None:
        """Echo input to output — uses 'in', 'out', 'loop', 'let'."""
        source = """
        fn main() {
            loop {
                let b: u8 = in(0);
                out(8, b);
            }
        }
        """
        types = _types(source)
        assert "fn" in types
        assert "loop" in types
        assert "let" in types
        assert "in" in types
        assert "out" in types
        assert "SEMICOLON" in types

    def test_example2_count_to_255(self) -> None:
        """Count from 0 to 255 — uses 'while', 'out', 'let', '!='."""
        source = """
        fn main() {
            let n: u8 = 0;
            while n != 255 {
                out(1, n);
                n = n + 1;
            }
            out(1, 255);
        }
        """
        types = _types(source)
        assert "while" in types
        assert "NEQ" in types
        assert "out" in types
        assert "INT_LIT" in types

    def test_example3_xor_checksum(self) -> None:
        """XOR checksum — uses bitwise XOR (^), 'in', 'while', 'out'."""
        source = """
        fn main() {
            let checksum: u8 = 0;
            let i: u8 = 0;
            while i != 8 {
                let b: u8 = in(0);
                checksum = checksum ^ b;
                i = i + 1;
            }
            out(1, checksum);
        }
        """
        types = _types(source)
        assert "CARET" in types
        assert "in" in types
        assert "while" in types

    def test_example4_16bit_counter_with_carry(self) -> None:
        """16-bit counter using carry() — uses 'static', 'carry', 'if'."""
        source = """
        static lo: u8 = 0;
        static hi: u8 = 0;

        fn tick() {
            let l: u8 = lo;
            l = l + 1;
            lo = l;
            if carry() {
                let h: u8 = hi;
                h = h + 1;
                hi = h;
                out(1, h);
            }
        }

        fn main() {
            loop {
                tick();
            }
        }
        """
        types = _types(source)
        assert "static" in types
        assert "carry" in types
        assert "if" in types
        assert "loop" in types

    def test_example5_bit_reversal_with_rotations(self) -> None:
        """Bit reversal using ral/rar — uses rotation intrinsics."""
        source = """
        fn reverse_bits(x: u8) -> u8 {
            let result: u8 = 0;
            let i: u8 = 0;
            while i != 8 {
                x = ral(x);
                result = rar(result);
                i = i + 1;
            }
            return result;
        }

        fn main() {
            let b: u8 = in(0);
            out(1, reverse_bits(b));
        }
        """
        types = _types(source)
        assert "ral" in types
        assert "rar" in types
        assert "return" in types
        assert "ARROW" in types
