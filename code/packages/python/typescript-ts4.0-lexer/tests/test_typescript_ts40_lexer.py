r"""Tests for the TypeScript 4.0 (2020) Lexer.

TypeScript 4.0 (August 2020) brought:
- Variadic tuple types: ``[...T, ...U]``
- Labeled tuple elements: ``[start: number, end: number]``
- Template literal types: ``\`Hello, ${string}!\```
- Short-circuit assignment operators: ``&&=``, ``||=``, ``??=``
- Catch variable binding as ``unknown`` by default with ``useUnknownInCatchVariables``

At the lexer level the key additions are the new compound assignment operators.
The template literal and tuple features reuse existing token kinds.

Test strategy:
- Verify TS 4.0-specific operators (short-circuit assignment)
- Verify labeled tuple element syntax tokenizes correctly
- Verify template literal types use the same backtick tokens
- Verify all TS 3.0 and ES2020 features still work
- Verify the factory function and direct tokenize produce identical results
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts40_lexer import create_ts40_lexer, tokenize_ts40

# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Return a list of token type names from a list of tokens."""
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    """Return the name of a single token's type."""
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    """Return a list of token values from a list of tokens."""
    return [t.value for t in tokens]


# ============================================================================
# Test: EOF on empty input
# ============================================================================


class TestEOF:
    """Empty source must produce exactly one EOF token."""

    def test_empty_source_produces_eof(self) -> None:
        tokens = tokenize_ts40("")
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"


# ============================================================================
# Test: TypeScript 4.0 — short-circuit assignment operators (NEW)
# ============================================================================


class TestShortCircuitAssignment:
    """TypeScript 4.0 (ES2021) added ``&&=``, ``||=``, and ``??=`` operators.

    These are compound assignment operators that only assign when the left
    side is truthy (``&&=``), falsy (``||=``), or nullish (``??=``).

    Truth table for ``a ??= b``:
    - If a is null or undefined → assign b to a
    - Otherwise → leave a unchanged (and do NOT evaluate b)

    At the lexer level these are single multi-character tokens.
    """

    def test_logical_and_assign(self) -> None:
        """``&&=`` tokenizes as a single operator token."""
        tokens = tokenize_ts40("a &&= b;")
        values = token_values(tokens)
        assert "&&=" in values

    def test_logical_or_assign(self) -> None:
        """``||=`` tokenizes as a single operator token."""
        tokens = tokenize_ts40("a ||= b;")
        values = token_values(tokens)
        assert "||=" in values

    def test_nullish_coalescing_assign(self) -> None:
        """``??=`` tokenizes as a single operator token."""
        tokens = tokenize_ts40("a ??= b;")
        values = token_values(tokens)
        assert "??=" in values


# ============================================================================
# Test: TypeScript 4.0 — labeled tuple elements
# ============================================================================


class TestLabeledTupleElements:
    """TypeScript 4.0 labeled tuple elements: ``[start: number, end: number]``.

    Labels are purely for documentation — they do not change the type's
    structure. At the lexer level, ``start`` and ``end`` tokenize as NAME
    and the ``:`` is a COLON, same as any other annotation.
    """

    def test_labeled_tuple_names_tokenize_as_names(self) -> None:
        tokens = tokenize_ts40("type Range = [start: number, end: number];")
        names = [t.value for t in tokens if token_type_name(t) == "NAME"]
        assert "start" in names
        assert "end" in names

    def test_labeled_tuple_has_colons(self) -> None:
        tokens = tokenize_ts40("type Range = [start: number, end: number];")
        types = token_types(tokens)
        assert "COLON" in types

    def test_labeled_tuple_has_commas(self) -> None:
        tokens = tokenize_ts40("type Range = [start: number, end: number];")
        types = token_types(tokens)
        assert "COMMA" in types


# ============================================================================
# Test: Template literal types (TS 4.0)
# ============================================================================


class TestTemplateLiteralTypes:
    r"""TypeScript 4.0 template literal types use backtick syntax at the type level.

    Example: ``type Greeting = \`Hello, ${string}!\```

    At the lexer level this is identical to template literal values — the
    backtick produces template token(s). The parser distinguishes the usage.
    """

    def test_template_literal_produces_tokens(self) -> None:
        """Backtick string must produce at least one non-EOF token."""
        tokens = tokenize_ts40("`hello`")
        assert len(tokens) > 1

    def test_template_with_interpolation(self) -> None:
        """Template with ``${...}`` must produce multiple tokens."""
        tokens = tokenize_ts40("`Hello, ${name}!`")
        assert len(tokens) > 2


# ============================================================================
# Test: Variadic tuple types (TS 4.0)
# ============================================================================


class TestVariadicTupleTypes:
    """TypeScript 4.0 variadic tuples:
    ``type Concat<T extends unknown[], U extends unknown[]> = [...T, ...U]``."""

    def test_spread_in_tuple_type(self) -> None:
        """``...T`` in a tuple type — spread token must appear."""
        tokens = tokenize_ts40("type T = [...string[]];")
        values = token_values(tokens)
        has_spread = "..." in values or any(
            "ELLIPSIS" in token_type_name(t) or "DOT_DOT_DOT" in token_type_name(t)
            for t in tokens
        )
        assert has_spread

    def test_variadic_concat_type_names(self) -> None:
        tokens = tokenize_ts40("type Concat<T, U> = [...T[], ...U[]];")
        names = [t.value for t in tokens if token_type_name(t) == "NAME"]
        assert "Concat" in names
        assert "T" in names
        assert "U" in names


# ============================================================================
# Test: TypeScript type annotations
# ============================================================================


class TestTypeAnnotations:
    """TypeScript type annotations use the colon syntax ``name: Type``."""

    def test_colon_in_annotation(self) -> None:
        tokens = tokenize_ts40("var x: number;")
        types = token_types(tokens)
        assert "COLON" in types

    def test_unknown_type_annotation(self) -> None:
        """``unknown`` type from TS 3.0 still works in TS 4.0."""
        tokens = tokenize_ts40("const x: unknown = 42;")
        values = token_values(tokens)
        assert "unknown" in values

    def test_string_type(self) -> None:
        tokens = tokenize_ts40("var s: string = 'hi';")
        values = token_values(tokens)
        assert "string" in values

    def test_number_type(self) -> None:
        tokens = tokenize_ts40("var n: number = 0;")
        values = token_values(tokens)
        assert "number" in values


# ============================================================================
# Test: ES keywords still work
# ============================================================================


class TestESKeywords:
    """TypeScript 4.0 is a superset of ES2020, so all ES keywords must work."""

    def test_var_keyword(self) -> None:
        tokens = tokenize_ts40("var x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "var"

    def test_const_keyword(self) -> None:
        tokens = tokenize_ts40("const x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_function_keyword(self) -> None:
        tokens = tokenize_ts40("function foo() {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"

    def test_class_keyword(self) -> None:
        tokens = tokenize_ts40("class Foo {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_return_keyword(self) -> None:
        tokens = tokenize_ts40("return x;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "return"

    def test_if_else_keywords(self) -> None:
        tokens = tokenize_ts40("if (x) {} else {}")
        keyword_values = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "if" in keyword_values
        assert "else" in keyword_values

    def test_extends_keyword(self) -> None:
        tokens = tokenize_ts40("class B extends A {}")
        keyword_values = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "extends" in keyword_values


# ============================================================================
# Test: Literals
# ============================================================================


class TestLiterals:
    """Number, string, boolean, and null literals."""

    def test_number_literal(self) -> None:
        tokens = tokenize_ts40("42")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_string_literal_single_quote(self) -> None:
        tokens = tokenize_ts40("'hello'")
        assert token_type_name(tokens[0]) == "STRING"

    def test_string_literal_double_quote(self) -> None:
        tokens = tokenize_ts40('"world"')
        assert token_type_name(tokens[0]) == "STRING"

    def test_true_false_literals(self) -> None:
        for lit in ["true", "false"]:
            tokens = tokenize_ts40(lit)
            assert tokens[0].type == TokenType.KEYWORD

    def test_null_literal(self) -> None:
        tokens = tokenize_ts40("null")
        assert tokens[0].type == TokenType.KEYWORD


# ============================================================================
# Test: Comments
# ============================================================================


class TestComments:
    """Comments must be skipped by the lexer."""

    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_ts40("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_ts40("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]


# ============================================================================
# Test: Operators
# ============================================================================


class TestOperators:
    """Key operators used in TypeScript code."""

    def test_arrow_function(self) -> None:
        """``=>`` is the arrow function operator."""
        tokens = tokenize_ts40("const f = (x) => x;")
        values = token_values(tokens)
        assert "=>" in values

    def test_nullish_coalescing(self) -> None:
        """``??`` is the nullish coalescing operator from ES2020."""
        tokens = tokenize_ts40("const y = x ?? 0;")
        values = token_values(tokens)
        assert "??" in values

    def test_optional_chaining(self) -> None:
        """``?.`` is the optional chaining operator from ES2020."""
        tokens = tokenize_ts40("const n = obj?.name;")
        values = token_values(tokens)
        assert "?." in values

    def test_pipe_for_union(self) -> None:
        tokens = tokenize_ts40("type T = string | number;")
        values = token_values(tokens)
        assert "|" in values


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS40Lexer:
    """The factory function ``create_ts40_lexer`` should return a usable lexer."""

    def test_creates_lexer_with_tokenize_method(self) -> None:
        lexer = create_ts40_lexer("const x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_and_direct_produce_same_result(self) -> None:
        """``tokenize_ts40`` and ``create_ts40_lexer(...).tokenize()`` must agree."""
        source = "type Pair = [first: string, second: number];"
        tokens_direct = tokenize_ts40(source)
        tokens_factory = create_ts40_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_factory_with_empty_source(self) -> None:
        lexer = create_ts40_lexer("")
        tokens = lexer.tokenize()
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"
