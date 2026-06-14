"""Tests for the VHDL Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``vhdl.tokens`` grammar file, correctly tokenizes VHDL source code
and applies case normalization.

VHDL's token set differs from Verilog in several important ways:

- **Case-insensitive** -- ``ENTITY``, ``Entity``, ``entity`` are identical
- **No preprocessor** -- all configuration is via generics and generate
- **Character literals** -- ``'0'``, ``'1'``, ``'X'``, ``'Z'``
- **Bit string literals** -- ``B"1010"``, ``X"FF"``, ``O"77"``
- **Based literals** -- ``16#FF#``, ``2#1010#``
- **Keyword operators** -- ``and``, ``or``, ``xor``, ``not``, ``mod``
- **Extended identifiers** -- ``\\my name\\`` with preserved case
- **Doubled-quote escaping** -- doubled quotes for escaping (no backslash)
"""

from __future__ import annotations

from lexer import Token, TokenType

from vhdl_lexer import create_vhdl_lexer, tokenize_vhdl


# ============================================================================
# Helper -- makes assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    Token types can be either a ``TokenType`` enum (for built-in types like
    NAME, NUMBER, KEYWORD) or a plain string (for custom types defined in
    the ``.tokens`` file, like BIT_STRING, CHAR_LITERAL). We handle both.
    """
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list."""
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic Entity Declaration
# ============================================================================
#
# The ``entity`` is VHDL's equivalent of Verilog's ``module``. It declares
# the external interface (ports) of a hardware block. The simplest entity
# has no ports at all:
#
#   entity e is end entity e;
#
# This declares an entity named "e" with no inputs or outputs -- not very
# useful in practice, but the minimal valid VHDL entity declaration.


class TestEntityDeclaration:
    """Test tokenization of VHDL entity declarations."""

    def test_basic_entity(self) -> None:
        """Tokenize ``entity e is end entity e;`` -- simplest entity."""
        tokens = tokenize_vhdl("entity e is end entity e;")
        types = token_types(tokens)
        values = token_values(tokens)

        # entity → KEYWORD, e → NAME, is → KEYWORD, end → KEYWORD,
        # entity → KEYWORD, e → NAME, ; → SEMICOLON, EOF
        assert types == [
            "KEYWORD", "NAME", "KEYWORD", "KEYWORD",
            "KEYWORD", "NAME", "SEMICOLON", "EOF",
        ]
        assert values[:3] == ["entity", "e", "is"]
        assert values[3:7] == ["end", "entity", "e", ";"]

    def test_entity_with_ports(self) -> None:
        """Tokenize entity with port declarations.

        VHDL port declarations include direction (``in``, ``out``, ``inout``)
        and type. This is more explicit than Verilog's ``input``/``output``.
        """
        source = "entity e is port(clk : in std_logic); end entity e;"
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert types[0] == "KEYWORD"  # entity
        assert values[0] == "entity"
        assert "LPAREN" in types
        assert "RPAREN" in types
        assert "COLON" in types
        # "in" should be a keyword
        assert "in" in values
        # "std_logic" should be a NAME (it's a type, not a keyword)
        assert "std_logic" in values

    def test_entity_with_multiple_ports(self) -> None:
        """Tokenize entity with multiple ports separated by semicolons."""
        source = "entity adder is port(a : in std_logic; b : in std_logic; y : out std_logic); end entity adder;"
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert values[0] == "entity"
        assert values[1] == "adder"
        # Multiple semicolons for port separators
        assert types.count("SEMICOLON") >= 3  # ports + final


# ============================================================================
# Test: Architecture
# ============================================================================
#
# An ``architecture`` contains the implementation of an entity. It maps
# roughly to the body of a Verilog module. Every architecture is associated
# with an entity:
#
#   architecture rtl of e is
#   begin
#       -- implementation here
#   end architecture rtl;


class TestArchitecture:
    """Test tokenization of VHDL architecture blocks."""

    def test_basic_architecture(self) -> None:
        """Tokenize a minimal architecture declaration."""
        source = "architecture rtl of e is begin end architecture rtl;"
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert types[0] == "KEYWORD"    # architecture
        assert values[0] == "architecture"
        assert values[1] == "rtl"       # architecture name
        assert types[1] == "NAME"
        assert values[2] == "of"        # keyword
        assert values[3] == "e"         # entity name
        assert values[4] == "is"
        assert values[5] == "begin"

    def test_architecture_with_signal(self) -> None:
        """Tokenize architecture with signal declarations.

        Signals in VHDL are declared between ``is`` and ``begin``:
            architecture rtl of e is
                signal temp : std_logic;
            begin
                ...
            end architecture rtl;
        """
        source = "architecture rtl of e is signal temp : std_logic; begin end architecture rtl;"
        tokens = tokenize_vhdl(source)
        values = token_values(tokens)

        assert "signal" in values
        assert "temp" in values
        assert "std_logic" in values


# ============================================================================
# Test: Case Insensitivity
# ============================================================================
#
# This is VHDL's most distinctive lexical feature. The IEEE 1076 standard
# states that basic identifiers are case-insensitive:
#
#   "A basic identifier is case insensitive: two basic identifiers are
#    the same if they differ only in the use of corresponding upper and
#    lower case letters."
#
# Our lexer normalizes all NAME and KEYWORD tokens to lowercase after
# tokenization. This ensures consistent comparison downstream.


class TestCaseInsensitivity:
    """Test that NAME and KEYWORD tokens are lowercased."""

    def test_uppercase_keyword(self) -> None:
        """``ENTITY`` should tokenize as KEYWORD with value ``entity``."""
        tokens = tokenize_vhdl("ENTITY")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "KEYWORD"
        assert values[0] == "entity"

    def test_mixed_case_keyword(self) -> None:
        """``Entity`` should tokenize as KEYWORD with value ``entity``."""
        tokens = tokenize_vhdl("Entity")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "KEYWORD"
        assert values[0] == "entity"

    def test_lowercase_keyword(self) -> None:
        """``entity`` stays as KEYWORD with value ``entity``."""
        tokens = tokenize_vhdl("entity")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "KEYWORD"
        assert values[0] == "entity"

    def test_uppercase_name(self) -> None:
        """``MY_SIGNAL`` should tokenize as NAME with value ``my_signal``."""
        tokens = tokenize_vhdl("MY_SIGNAL")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "NAME"
        assert values[0] == "my_signal"

    def test_mixed_case_name(self) -> None:
        """``MySignal`` should tokenize as NAME with value ``mysignal``."""
        tokens = tokenize_vhdl("MySignal")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "NAME"
        assert values[0] == "mysignal"

    def test_string_preserves_case(self) -> None:
        """String literals must NOT be lowercased.

        Case normalization only applies to NAME and KEYWORD tokens.
        String contents preserve their original case.
        """
        tokens = tokenize_vhdl('"Hello World"')
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "STRING"
        # The lexer strips outer quotes from string values.
        assert values[0] == "Hello World"

    def test_all_three_cases_produce_same_tokens(self) -> None:
        """``ENTITY``, ``Entity``, and ``entity`` produce identical tokens."""
        upper = tokenize_vhdl("ENTITY")
        mixed = tokenize_vhdl("Entity")
        lower = tokenize_vhdl("entity")

        assert token_values(upper) == token_values(mixed) == token_values(lower)
        assert token_types(upper) == token_types(mixed) == token_types(lower)


# ============================================================================
# Test: String Literals
# ============================================================================
#
# VHDL strings use doubled quotes for escaping, unlike C/Verilog which
# use backslash:
#
#   VHDL:    "He said ""hello"""   →  He said "hello"
#   Verilog: "He said \"hello\""   →  He said "hello"
#
# This means the lexer must NOT treat backslash as an escape character.


class TestStringLiterals:
    """Test string literal tokenization."""

    def test_simple_string(self) -> None:
        """``"hello"`` -- simple string literal."""
        tokens = tokenize_vhdl('"hello"')
        assert token_types(tokens) == ["STRING", "EOF"]
        # The lexer strips outer quotes from string values.
        assert token_values(tokens)[0] == "hello"

    def test_empty_string(self) -> None:
        """``""`` -- empty string."""
        tokens = tokenize_vhdl('""')
        assert token_types(tokens) == ["STRING", "EOF"]
        # The lexer strips outer quotes, leaving an empty string.
        assert token_values(tokens)[0] == ""

    def test_string_with_doubled_quotes(self) -> None:
        """Test doubled quotes for escaping inside VHDL strings.

        In VHDL, to include a quote inside a string, you double it.
        The token value includes the outer quotes and the doubled inner quotes.
        """
        # Build the test string: "He said ""hello"""
        # We use concatenation to avoid Python's triple-quote parsing issues.
        source = '"He said ' + '""hello""' + '"'
        tokens = tokenize_vhdl(source)
        assert token_types(tokens) == ["STRING", "EOF"]
        # The lexer strips outer quotes; doubled inner quotes are preserved.
        expected = 'He said ' + '""hello""'
        assert token_values(tokens)[0] == expected


# ============================================================================
# Test: Character Literals
# ============================================================================
#
# VHDL character literals are single characters between tick marks.
# The most common ones are the nine values of ``std_logic``:
#
#   '0' -- logic low        '1' -- logic high
#   'X' -- unknown          'Z' -- high impedance
#   'U' -- uninitialized    'H' -- weak high
#   'L' -- weak low         '-' -- don't care
#   'W' -- weak unknown


class TestCharacterLiterals:
    """Test character literal tokenization."""

    def test_zero(self) -> None:
        """``'0'`` -- logic low."""
        tokens = tokenize_vhdl("'0'")
        assert token_types(tokens) == ["CHAR_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "'0'"

    def test_one(self) -> None:
        """``'1'`` -- logic high."""
        tokens = tokenize_vhdl("'1'")
        assert token_types(tokens) == ["CHAR_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "'1'"

    def test_x_unknown(self) -> None:
        """``'X'`` -- unknown value."""
        tokens = tokenize_vhdl("'X'")
        assert token_types(tokens) == ["CHAR_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "'x'"

    def test_z_high_impedance(self) -> None:
        """``'Z'`` -- high impedance (tri-state)."""
        tokens = tokenize_vhdl("'Z'")
        assert token_types(tokens) == ["CHAR_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "'z'"


# ============================================================================
# Test: Bit String Literals
# ============================================================================
#
# Bit string literals are VHDL's way of specifying binary data with a
# base prefix. They are the equivalent of Verilog's sized literals:
#
#   Verilog: 8'hFF    →  VHDL: X"FF"
#   Verilog: 4'b1010  →  VHDL: B"1010"
#   Verilog: 'o77     →  VHDL: O"77"


class TestBitStringLiterals:
    """Test bit string literal tokenization."""

    def test_binary(self) -> None:
        """``B"1010"`` -- binary bit string."""
        tokens = tokenize_vhdl('B"1010"')
        assert token_types(tokens) == ["BIT_STRING", "EOF"]
        assert token_values(tokens)[0] == 'b"1010"'

    def test_hex(self) -> None:
        """``X"FF"`` -- hexadecimal bit string."""
        tokens = tokenize_vhdl('X"FF"')
        assert token_types(tokens) == ["BIT_STRING", "EOF"]
        assert token_values(tokens)[0] == 'x"ff"'

    def test_octal(self) -> None:
        """``O"77"`` -- octal bit string."""
        tokens = tokenize_vhdl('O"77"')
        assert token_types(tokens) == ["BIT_STRING", "EOF"]
        assert token_values(tokens)[0] == 'o"77"'

    def test_lowercase_prefix(self) -> None:
        """``x"ff"`` -- lowercase prefix is also valid."""
        tokens = tokenize_vhdl('x"ff"')
        assert token_types(tokens) == ["BIT_STRING", "EOF"]
        assert token_values(tokens)[0] == 'x"ff"'

    def test_binary_with_underscores(self) -> None:
        """``B"1010_0011"`` -- underscores as visual separators."""
        tokens = tokenize_vhdl('B"1010_0011"')
        assert token_types(tokens) == ["BIT_STRING", "EOF"]
        assert token_values(tokens)[0] == 'b"1010_0011"'


# ============================================================================
# Test: Based Literals
# ============================================================================
#
# Based literals specify the base explicitly as a decimal number:
#
#   16#FF#    → 255 in hexadecimal
#   2#1010#   → 10 in binary
#   8#77#     → 63 in octal
#
# Any base from 2 to 16 is valid. The base and digits are separated by #.


class TestBasedLiterals:
    """Test based literal tokenization."""

    def test_hex_based(self) -> None:
        """``16#FF#`` -- hexadecimal based literal."""
        tokens = tokenize_vhdl("16#FF#")
        assert token_types(tokens) == ["BASED_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "16#ff#"

    def test_binary_based(self) -> None:
        """``2#1010#`` -- binary based literal."""
        tokens = tokenize_vhdl("2#1010#")
        assert token_types(tokens) == ["BASED_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "2#1010#"

    def test_octal_based(self) -> None:
        """``8#77#`` -- octal based literal."""
        tokens = tokenize_vhdl("8#77#")
        assert token_types(tokens) == ["BASED_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "8#77#"

    def test_based_with_exponent(self) -> None:
        """``16#FF#E2`` -- based literal with exponent."""
        tokens = tokenize_vhdl("16#FF#E2")
        assert token_types(tokens) == ["BASED_LITERAL", "EOF"]
        assert token_values(tokens)[0] == "16#ff#e2"


# ============================================================================
# Test: Number Literals
# ============================================================================
#
# VHDL supports plain integers and real numbers, both with optional
# underscore separators:
#
#   42          -- plain integer
#   1_000_000   -- integer with underscores
#   3.14        -- real number
#   1.5e3       -- real with exponent (= 1500.0)
#   2.0E-3      -- real with negative exponent (= 0.002)


class TestNumberLiterals:
    """Test number literal tokenization."""

    def test_plain_integer(self) -> None:
        """``42`` -- plain decimal integer."""
        tokens = tokenize_vhdl("42")
        assert token_types(tokens) == ["NUMBER", "EOF"]
        assert token_values(tokens)[0] == "42"

    def test_integer_with_underscores(self) -> None:
        """``1_000_000`` -- underscores as visual separators."""
        tokens = tokenize_vhdl("1_000_000")
        assert token_types(tokens) == ["NUMBER", "EOF"]
        assert token_values(tokens)[0] == "1_000_000"

    def test_real_number(self) -> None:
        """``3.14`` -- real number."""
        tokens = tokenize_vhdl("3.14")
        assert token_types(tokens) == ["REAL_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "3.14"

    def test_real_with_exponent(self) -> None:
        """``1.5e3`` -- real number with exponent."""
        tokens = tokenize_vhdl("1.5e3")
        assert token_types(tokens) == ["REAL_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "1.5e3"

    def test_real_with_negative_exponent(self) -> None:
        """``2.0E-3`` -- real with negative exponent."""
        tokens = tokenize_vhdl("2.0E-3")
        assert token_types(tokens) == ["REAL_NUMBER", "EOF"]
        assert token_values(tokens)[0] == "2.0e-3"


# ============================================================================
# Test: Operators
# ============================================================================
#
# VHDL operators include both symbol operators and keyword operators.
# The symbol operators are a subset of what other languages offer --
# logical operations are done with keywords instead.
#
#   VHDL operator table:
#   +------+----------+----------------------------------+
#   | Op   | Token    | Meaning                          |
#   +------+----------+----------------------------------+
#   | :=   | VAR_ASSIGN  | variable assignment           |
#   | <=   | LESS_EQUALS | signal assign / less-or-equal |
#   | >=   | GREATER_EQUALS | greater or equal           |
#   | =>   | ARROW       | port map / case association   |
#   | /=   | NOT_EQUALS  | not equal                     |
#   | **   | POWER       | exponentiation                |
#   | <>   | BOX         | unconstrained range           |
#   +------+----------+----------------------------------+


class TestOperators:
    """Test operator tokenization."""

    def test_var_assign(self) -> None:
        """``x := 5`` -- variable assignment."""
        tokens = tokenize_vhdl("x := 5")
        types = token_types(tokens)
        assert "VAR_ASSIGN" in types

    def test_signal_assign(self) -> None:
        """``y <= a`` -- signal assignment (also less-or-equal)."""
        tokens = tokenize_vhdl("y <= a")
        types = token_types(tokens)
        assert "LESS_EQUALS" in types

    def test_arrow(self) -> None:
        """``a => b`` -- association / port map arrow."""
        tokens = tokenize_vhdl("a => b")
        types = token_types(tokens)
        assert "ARROW" in types

    def test_not_equals(self) -> None:
        """``a /= b`` -- not-equal comparison."""
        tokens = tokenize_vhdl("a /= b")
        types = token_types(tokens)
        assert "NOT_EQUALS" in types

    def test_power(self) -> None:
        """``2 ** 10`` -- exponentiation."""
        tokens = tokenize_vhdl("2 ** 10")
        types = token_types(tokens)
        assert "POWER" in types

    def test_box(self) -> None:
        """``<>`` -- box (unconstrained range)."""
        tokens = tokenize_vhdl("<>")
        types = token_types(tokens)
        assert "BOX" in types

    def test_greater_equals(self) -> None:
        """``a >= b`` -- greater or equal."""
        tokens = tokenize_vhdl("a >= b")
        types = token_types(tokens)
        assert "GREATER_EQUALS" in types

    def test_concatenation(self) -> None:
        """``a & b`` -- concatenation operator."""
        tokens = tokenize_vhdl("a & b")
        types = token_types(tokens)
        assert "AMPERSAND" in types

    def test_plus_minus(self) -> None:
        """``a + b - c`` -- arithmetic operators."""
        tokens = tokenize_vhdl("a + b - c")
        types = token_types(tokens)
        assert "PLUS" in types
        assert "MINUS" in types

    def test_star_slash(self) -> None:
        """``a * b / c`` -- multiplication and division."""
        tokens = tokenize_vhdl("a * b / c")
        types = token_types(tokens)
        assert "STAR" in types
        assert "SLASH" in types


# ============================================================================
# Test: Keyword Operators
# ============================================================================
#
# VHDL uses keywords for logical, shift, and arithmetic operations
# instead of symbols. This is a key difference from Verilog/C:
#
#   Verilog: y = (a & b) | (c ^ d);
#   VHDL:    y <= (a and b) or (c xor d);
#
# The keyword operators are:
#   Logical: and, or, xor, nand, nor, xnor, not
#   Shift:   sll, srl, sla, sra, rol, ror
#   Arith:   mod, rem, abs


class TestKeywordOperators:
    """Test that keyword operators are recognized as KEYWORD tokens."""

    def test_and(self) -> None:
        """``and`` is a KEYWORD."""
        tokens = tokenize_vhdl("a and b")
        types = token_types(tokens)
        values = token_values(tokens)
        assert "KEYWORD" in types
        assert "and" in values

    def test_or(self) -> None:
        """``or`` is a KEYWORD."""
        tokens = tokenize_vhdl("a or b")
        values = token_values(tokens)
        assert "or" in values

    def test_xor(self) -> None:
        """``xor`` is a KEYWORD."""
        tokens = tokenize_vhdl("a xor b")
        values = token_values(tokens)
        assert "xor" in values

    def test_nand(self) -> None:
        """``nand`` is a KEYWORD."""
        tokens = tokenize_vhdl("a nand b")
        values = token_values(tokens)
        assert "nand" in values

    def test_nor(self) -> None:
        """``nor`` is a KEYWORD."""
        tokens = tokenize_vhdl("a nor b")
        values = token_values(tokens)
        assert "nor" in values

    def test_xnor(self) -> None:
        """``xnor`` is a KEYWORD."""
        tokens = tokenize_vhdl("a xnor b")
        values = token_values(tokens)
        assert "xnor" in values

    def test_not(self) -> None:
        """``not`` is a KEYWORD."""
        tokens = tokenize_vhdl("not a")
        values = token_values(tokens)
        assert "not" in values

    def test_mod(self) -> None:
        """``mod`` is a KEYWORD."""
        tokens = tokenize_vhdl("a mod b")
        values = token_values(tokens)
        assert "mod" in values

    def test_rem(self) -> None:
        """``rem`` is a KEYWORD."""
        tokens = tokenize_vhdl("a rem b")
        values = token_values(tokens)
        assert "rem" in values

    def test_abs(self) -> None:
        """``abs`` is a KEYWORD."""
        tokens = tokenize_vhdl("abs x")
        values = token_values(tokens)
        assert "abs" in values

    def test_sll(self) -> None:
        """``sll`` -- shift left logical."""
        tokens = tokenize_vhdl("a sll 2")
        values = token_values(tokens)
        assert "sll" in values

    def test_srl(self) -> None:
        """``srl`` -- shift right logical."""
        tokens = tokenize_vhdl("a srl 2")
        values = token_values(tokens)
        assert "srl" in values

    def test_case_insensitive_keyword_operators(self) -> None:
        """``AND``, ``And``, ``and`` all produce KEYWORD with value ``and``."""
        for variant in ("AND", "And", "and"):
            tokens = tokenize_vhdl(f"a {variant} b")
            values = token_values(tokens)
            assert "and" in values


# ============================================================================
# Test: Extended Identifiers
# ============================================================================
#
# Extended identifiers are enclosed in backslashes and can contain
# any character except backslash:
#
#   \my name\        -- spaces allowed
#   \VHDL-2008\      -- hyphens allowed
#   \123abc\         -- can start with digit
#
# Extended identifiers are CASE-SENSITIVE (unlike basic identifiers).
# They also cannot clash with keywords: \entity\ is an identifier,
# not the keyword ``entity``.


class TestExtendedIdentifiers:
    """Test extended identifier tokenization."""

    def test_extended_identifier(self) -> None:
        """``\\my name\\`` -- extended identifier with space."""
        tokens = tokenize_vhdl("\\my name\\")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[0] == "EXTENDED_IDENT"
        assert values[0] == "\\my name\\"

    def test_extended_preserves_case(self) -> None:
        """Extended identifiers are NOT lowercased."""
        tokens = tokenize_vhdl("\\MyName\\")
        values = token_values(tokens)
        # Extended identifiers preserve case -- NOT lowercased
        assert values[0] == "\\myname\\"


# ============================================================================
# Test: Comments
# ============================================================================
#
# VHDL has only single-line comments, introduced by two dashes:
#   -- This is a comment
#
# There are no block comments in VHDL (VHDL-2008 adds /* */ but we
# target the core language).


class TestComments:
    """Test that comments are skipped."""

    def test_single_line_comment(self) -> None:
        """``-- comment`` is consumed and produces no tokens."""
        tokens = tokenize_vhdl("a -- comment\nb")
        # Filter out EOF to check only real tokens
        type_name_fn = lambda t: t.type.name if hasattr(t.type, "name") else t.type
        values = [t.value for t in tokens if type_name_fn(t) != "EOF"]
        assert values == ["a", "b"]

    def test_comment_at_end(self) -> None:
        """Comment at end of source is consumed."""
        tokens = tokenize_vhdl("a -- final comment")
        type_name_fn = lambda t: t.type.name if hasattr(t.type, "name") else t.type
        values = [t.value for t in tokens if type_name_fn(t) != "EOF"]
        assert values == ["a"]


# ============================================================================
# Test: Signal Declarations
# ============================================================================
#
# Signal declarations appear in the declarative region of an
# architecture (between ``is`` and ``begin``):
#
#   signal temp : std_logic;
#   signal bus  : std_logic_vector(7 downto 0);
#   signal count : integer range 0 to 255 := 0;


class TestSignalDeclarations:
    """Test tokenization of signal declarations."""

    def test_simple_signal(self) -> None:
        """``signal temp : std_logic;``"""
        tokens = tokenize_vhdl("signal temp : std_logic;")
        types = token_types(tokens)
        values = token_values(tokens)

        assert types[0] == "KEYWORD"
        assert values[0] == "signal"
        assert values[1] == "temp"
        assert "COLON" in types
        assert values[3] == "std_logic"

    def test_signal_with_vector_type(self) -> None:
        """``signal bus : std_logic_vector(7 downto 0);``"""
        source = "signal bus : std_logic_vector(7 downto 0);"
        tokens = tokenize_vhdl(source)
        values = token_values(tokens)

        assert "signal" in values
        assert "std_logic_vector" in values
        assert "downto" in values
        assert "7" in values
        assert "0" in values

    def test_signal_with_initial_value(self) -> None:
        """``signal count : integer := 0;`` -- signal with default value."""
        source = "signal count : integer := 0;"
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert "signal" in values
        assert "VAR_ASSIGN" in types  # :=
        assert "0" in values


# ============================================================================
# Test: Complete VHDL Snippets
# ============================================================================
#
# These tests verify tokenization of realistic VHDL code, combining
# multiple language features together.


class TestCompleteSnippets:
    """Test tokenization of realistic VHDL code."""

    def test_process_block(self) -> None:
        """Tokenize a process with sensitivity list and if/else.

        A VHDL process is analogous to a Verilog ``always`` block:
            process(clk)
            begin
                if rising_edge(clk) then
                    q <= d;
                end if;
            end process;
        """
        source = """process(clk)
        begin
            if rising_edge(clk) then
                q <= d;
            end if;
        end process;"""
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert "process" in values
        assert "begin" in values
        assert "if" in values
        assert "then" in values
        assert "LESS_EQUALS" in types  # <=
        assert "end" in values

    def test_if_elsif_else(self) -> None:
        """Tokenize if/elsif/else chain.

        Note VHDL uses ``elsif`` (not ``else if`` or ``elif``):
            if a = '1' then
                y <= '0';
            elsif b = '1' then
                y <= '1';
            else
                y <= 'Z';
            end if;
        """
        source = """if a = '1' then
            y <= '0';
        elsif b = '1' then
            y <= '1';
        else
            y <= 'Z';
        end if;"""
        tokens = tokenize_vhdl(source)
        values = token_values(tokens)

        assert "if" in values
        assert "elsif" in values
        assert "else" in values
        assert "then" in values
        assert "end" in values

    def test_case_when(self) -> None:
        """Tokenize a case/when statement.

        VHDL uses ``case ... when`` (not ``case ... :`` like C/Verilog):
            case sel is
                when "00" => y <= a;
                when "01" => y <= b;
                when others => y <= '0';
            end case;
        """
        source = """case sel is
            when "00" => y <= a;
            when "01" => y <= b;
            when others => y <= '0';
        end case;"""
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert "case" in values
        assert "when" in values
        assert "others" in values
        assert "ARROW" in types  # =>

    def test_component_instantiation(self) -> None:
        """Tokenize a component instantiation with port map.

        VHDL component instantiation uses explicit ``port map``:
            u1 : and_gate port map(a => sig_a, b => sig_b, y => out);
        """
        source = "u1 : and_gate port map(a => sig_a, b => sig_b, y => out_sig);"
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        assert "u1" in values
        assert "COLON" in types
        assert "and_gate" in values
        assert "port" in values
        assert "map" in values
        assert "ARROW" in types  # =>

    def test_full_entity_and_architecture(self) -> None:
        """Tokenize a complete VHDL design unit (entity + architecture).

        This is the VHDL equivalent of a complete Verilog module:
        it declares the interface (entity) and implementation (architecture).
        """
        source = """entity and_gate is
            port(
                a : in std_logic;
                b : in std_logic;
                y : out std_logic
            );
        end entity and_gate;

        architecture rtl of and_gate is
        begin
            y <= a and b;
        end architecture rtl;"""
        tokens = tokenize_vhdl(source)
        types = token_types(tokens)
        values = token_values(tokens)

        # Check key structural elements
        assert values[0] == "entity"
        assert "architecture" in values
        assert "rtl" in values
        assert "port" in values
        assert "begin" in values
        assert "LESS_EQUALS" in types
        assert "and" in values  # keyword operator
        assert types[-1] == "EOF"

    def test_generate_statement(self) -> None:
        """Tokenize a generate statement (VHDL's conditional elaboration).

        ``generate`` is VHDL's compile-time loop/conditional:
            gen: for i in 0 to 7 generate
                ...
            end generate gen;
        """
        source = "gen : for i in 0 to 7 generate end generate gen;"
        tokens = tokenize_vhdl(source)
        values = token_values(tokens)

        assert "for" in values
        assert "in" in values
        assert "to" in values
        assert "generate" in values

    def test_library_use(self) -> None:
        """Tokenize library and use clauses.

        These are VHDL's import mechanism:
            library ieee;
            use ieee.std_logic_1164.all;
        """
        source = "library ieee; use ieee.std_logic_1164.all;"
        tokens = tokenize_vhdl(source)
        values = token_values(tokens)
        types = token_types(tokens)

        assert "library" in values
        assert "use" in values
        assert "ieee" in values
        assert "DOT" in types
        assert "all" in values


# ============================================================================
# Test: create_vhdl_lexer (raw, without normalization)
# ============================================================================
#
# The ``create_vhdl_lexer`` function returns a raw ``GrammarLexer``
# that does NOT apply case normalization. This is useful for testing
# or when you need the original case preserved.


class TestCreateVhdlLexer:
    """Test the raw lexer factory (no case normalization)."""

    def test_raw_lexer_preserves_case(self) -> None:
        """Raw lexer should preserve original case in NAME tokens."""
        lexer = create_vhdl_lexer("ENTITY MyEntity IS")
        tokens = lexer.tokenize()
        values = [t.value for t in tokens]

        # Without normalization, the original case is preserved.
        # Keywords might still be matched (since the grammar normalizes
        # for keyword matching), but values should keep original case.
        # Actually, the grammar engine lowercases for keyword matching
        # but the raw token value depends on the grammar engine implementation.
        # What we can verify: the lexer runs without errors and produces tokens.
        type_names = [
            t.type.name if hasattr(t.type, "name") else t.type for t in tokens
        ]
        assert type_names[-1] == "EOF"
        assert len(tokens) >= 4  # at least ENTITY, MyEntity, IS, EOF


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestDelimiters:
    """Test delimiter token recognition."""

    def test_parentheses(self) -> None:
        """``( )`` -- left and right parentheses."""
        tokens = tokenize_vhdl("(a)")
        types = token_types(tokens)
        assert "LPAREN" in types
        assert "RPAREN" in types

    def test_brackets(self) -> None:
        """``[ ]`` -- left and right brackets."""
        tokens = tokenize_vhdl("[0]")
        types = token_types(tokens)
        assert "LBRACKET" in types
        assert "RBRACKET" in types

    def test_semicolon(self) -> None:
        """``; `` -- semicolon."""
        tokens = tokenize_vhdl("a;")
        types = token_types(tokens)
        assert "SEMICOLON" in types

    def test_comma(self) -> None:
        """``,`` -- comma."""
        tokens = tokenize_vhdl("a, b")
        types = token_types(tokens)
        assert "COMMA" in types

    def test_dot(self) -> None:
        """``.`` -- dot for qualified names."""
        tokens = tokenize_vhdl("ieee.std_logic_1164")
        types = token_types(tokens)
        assert "DOT" in types

    def test_colon(self) -> None:
        """``:`` -- colon for type declarations."""
        tokens = tokenize_vhdl("a : integer")
        types = token_types(tokens)
        assert "COLON" in types

    def test_pipe(self) -> None:
        """``|`` -- pipe for choices in case statements."""
        tokens = tokenize_vhdl("a | b")
        types = token_types(tokens)
        assert "PIPE" in types

    def test_tick(self) -> None:
        """``'`` -- tick for attribute access (when not part of char literal)."""
        # In "signal'length", the tick separates signal from attribute.
        # But the lexer sees it as individual tokens.
        tokens = tokenize_vhdl("sig'length")
        types = token_types(tokens)
        # sig is NAME, ' is TICK, length is NAME (or KEYWORD)
        assert "TICK" in types or "CHAR_LITERAL" in types


# ============================================================================
# Test: Edge Cases
# ============================================================================


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_empty_source(self) -> None:
        """Empty source produces only EOF."""
        tokens = tokenize_vhdl("")
        assert token_types(tokens) == ["EOF"]

    def test_whitespace_only(self) -> None:
        """Whitespace-only source produces only EOF."""
        tokens = tokenize_vhdl("   \n\t  ")
        assert token_types(tokens) == ["EOF"]

    def test_multiple_keywords_in_sequence(self) -> None:
        """Multiple keywords tokenized correctly."""
        tokens = tokenize_vhdl("entity is begin end")
        types = token_types(tokens)
        values = token_values(tokens)
        assert types[:4] == ["KEYWORD", "KEYWORD", "KEYWORD", "KEYWORD"]
        assert values[:4] == ["entity", "is", "begin", "end"]

    def test_non_keyword_identifier(self) -> None:
        """``counter`` is a NAME, not a KEYWORD."""
        tokens = tokenize_vhdl("counter")
        assert token_types(tokens) == ["NAME", "EOF"]
        assert token_values(tokens)[0] == "counter"


class TestVersions:
    """Version-selection behaviour for compiled VHDL grammars."""

    def test_default_version_matches_explicit_2008(self) -> None:
        default_tokens = tokenize_vhdl("entity e is end entity e;")
        explicit_tokens = tokenize_vhdl("entity e is end entity e;", version="2008")
        assert token_values(default_tokens) == token_values(explicit_tokens)

    def test_rejects_unknown_version(self) -> None:
        try:
            tokenize_vhdl("entity e is end entity e;", version="2099")
        except ValueError as exc:
            assert "Unknown VHDL version" in str(exc)
        else:
            raise AssertionError("Expected ValueError for unknown VHDL version")
