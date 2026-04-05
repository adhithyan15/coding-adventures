"""Tests for the TypeScript 3.0 (2018) Lexer.

TypeScript 3.0 introduced the ``unknown`` top type as a safer alternative to
``any``. The lexical changes are minimal — the token set is essentially the
TypeScript 2.x set with ``unknown`` added and better tuple spread handling.

Test strategy:
- Verify TypeScript 3.0-specific features (unknown type)
- Verify TypeScript general features (interfaces, type annotations, generics)
- Verify ES2018 baseline features still work
- Verify the factory function and direct tokenize function produce identical results
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts30_lexer import create_ts30_lexer, tokenize_ts30

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
        tokens = tokenize_ts30("")
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"


# ============================================================================
# Test: TypeScript 3.0 — unknown type (NEW in TS 3.0)
# ============================================================================


class TestUnknownType:
    """TypeScript 3.0 introduced ``unknown`` as the top type.

    Unlike ``any``, you cannot use a value of type ``unknown`` without first
    narrowing its type. At the lexer level, ``unknown`` is emitted as a NAME
    token (it is a contextual keyword, not a reserved word).
    """

    def test_unknown_tokenizes_as_name(self) -> None:
        """``unknown`` is a contextual keyword — lexed as NAME."""
        tokens = tokenize_ts30("unknown")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "unknown"

    def test_unknown_type_annotation(self) -> None:
        """``const x: unknown = 42;`` uses unknown as a type annotation."""
        tokens = tokenize_ts30("const x: unknown = 42;")
        values = token_values(tokens)
        assert "unknown" in values

    def test_unknown_in_union_type(self) -> None:
        """``string | unknown`` reduces to ``unknown`` — but we just lex it."""
        tokens = tokenize_ts30("type T = string | unknown;")
        values = token_values(tokens)
        assert "unknown" in values
        assert "string" in values

    def test_unknown_as_parameter_type(self) -> None:
        """Function parameters can have the ``unknown`` type."""
        tokens = tokenize_ts30("function foo(x: unknown): void {}")
        values = token_values(tokens)
        assert "unknown" in values
        assert "void" in values


# ============================================================================
# Test: TypeScript type annotations
# ============================================================================


class TestTypeAnnotations:
    """TypeScript type annotations use the colon syntax ``name: Type``."""

    def test_colon_in_annotation(self) -> None:
        tokens = tokenize_ts30("var x: number;")
        types = token_types(tokens)
        assert "COLON" in types

    def test_number_type(self) -> None:
        tokens = tokenize_ts30("var n: number = 0;")
        values = token_values(tokens)
        assert "number" in values

    def test_string_type(self) -> None:
        tokens = tokenize_ts30("var s: string = 'hi';")
        values = token_values(tokens)
        assert "string" in values

    def test_boolean_type(self) -> None:
        tokens = tokenize_ts30("var b: boolean = true;")
        values = token_values(tokens)
        assert "boolean" in values

    def test_void_type(self) -> None:
        tokens = tokenize_ts30("function f(): void {}")
        values = token_values(tokens)
        assert "void" in values

    def test_any_type(self) -> None:
        """``any`` is a contextual keyword, emitted as NAME."""
        tokens = tokenize_ts30("var x: any;")
        values = token_values(tokens)
        assert "any" in values


# ============================================================================
# Test: TypeScript interface and class
# ============================================================================


class TestInterfaceAndClass:
    """``interface`` is a contextual keyword in TypeScript — emitted as NAME.
    ``class`` is a reserved keyword — emitted as KEYWORD."""

    def test_interface_keyword_as_name(self) -> None:
        """interface is contextual — the name after it is also NAME."""
        tokens = tokenize_ts30("interface Foo { }")
        names = [t.value for t in tokens if token_type_name(t) == "NAME"]
        assert "Foo" in names

    def test_class_is_keyword(self) -> None:
        tokens = tokenize_ts30("class Foo {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "class"

    def test_extends_keyword(self) -> None:
        tokens = tokenize_ts30("class B extends A {}")
        keyword_values = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "extends" in keyword_values

    def test_implements_as_name(self) -> None:
        """``implements`` is a contextual keyword in TypeScript."""
        tokens = tokenize_ts30("class Foo implements Bar {}")
        values = token_values(tokens)
        assert "implements" in values


# ============================================================================
# Test: Generics
# ============================================================================


class TestGenerics:
    """TypeScript generics use angle bracket syntax ``Array<T>``."""

    def test_generic_type_parameter(self) -> None:
        tokens = tokenize_ts30("const a: Array<number> = [];")
        types = token_types(tokens)
        assert "LT" in types or "LANGLE" in types or any(
            t == "<" for t in token_values(tokens)
        )

    def test_generic_function(self) -> None:
        tokens = tokenize_ts30("function identity<T>(x: T): T { return x; }")
        values = token_values(tokens)
        assert "T" in values
        assert "identity" in values


# ============================================================================
# Test: Tuple type rest elements (TS 3.0 feature)
# ============================================================================


class TestTupleRestElements:
    """TypeScript 3.0 added rest elements in tuple types: ``[string, ...number[]]``."""

    def test_rest_in_tuple_type(self) -> None:
        """The spread operator ``...`` appears inside a tuple type annotation."""
        tokens = tokenize_ts30("type T = [string, ...number[]];")
        values = token_values(tokens)
        assert "string" in values
        assert "number" in values

    def test_spread_token_present(self) -> None:
        """``...`` should tokenize as ELLIPSIS or DOT_DOT_DOT."""
        tokens = tokenize_ts30("type T = [string, ...number[]];")
        types = token_types(tokens)
        has_spread = (
            "ELLIPSIS" in types
            or "DOT_DOT_DOT" in types
            or "..." in token_values(tokens)
        )
        assert has_spread


# ============================================================================
# Test: ES keywords still work
# ============================================================================


class TestESKeywords:
    """TypeScript 3.0 is a superset of ES2018, so all ES keywords must work."""

    def test_var_keyword(self) -> None:
        tokens = tokenize_ts30("var x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "var"

    def test_const_keyword(self) -> None:
        tokens = tokenize_ts30("const x = 1;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_let_keyword(self) -> None:
        tokens = tokenize_ts30("let x = 1;")
        # let is a KEYWORD in ES2015+ (promoted from reserved word)
        assert "let" in token_values(tokens)

    def test_function_keyword(self) -> None:
        tokens = tokenize_ts30("function foo() {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"

    def test_return_keyword(self) -> None:
        tokens = tokenize_ts30("return x;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "return"

    def test_if_else_keywords(self) -> None:
        tokens = tokenize_ts30("if (x) {} else {}")
        keyword_values = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "if" in keyword_values
        assert "else" in keyword_values

    def test_for_keyword(self) -> None:
        tokens = tokenize_ts30("for (var i = 0; i < 10; i++) {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "for"


# ============================================================================
# Test: Literals
# ============================================================================


class TestLiterals:
    """Number, string, boolean, and null literals."""

    def test_number_literal(self) -> None:
        tokens = tokenize_ts30("42")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_string_literal_single_quote(self) -> None:
        tokens = tokenize_ts30("'hello'")
        assert token_type_name(tokens[0]) == "STRING"

    def test_string_literal_double_quote(self) -> None:
        tokens = tokenize_ts30('"world"')
        assert token_type_name(tokens[0]) == "STRING"

    def test_template_literal(self) -> None:
        """Template literals (backtick strings) are an ES6+ feature."""
        tokens = tokenize_ts30("`hello ${name}`")
        assert len(tokens) > 1  # at least some tokens produced

    def test_true_false_literals(self) -> None:
        for lit in ["true", "false"]:
            tokens = tokenize_ts30(lit)
            assert tokens[0].type == TokenType.KEYWORD

    def test_null_literal(self) -> None:
        tokens = tokenize_ts30("null")
        assert tokens[0].type == TokenType.KEYWORD


# ============================================================================
# Test: Comments
# ============================================================================


class TestComments:
    """Comments must be skipped by the lexer."""

    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_ts30("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_ts30("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]

    def test_jsdoc_comment_skipped(self) -> None:
        """JSDoc comments are block comments and should be skipped."""
        tokens = tokenize_ts30("/** @param x */ function f() {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"


# ============================================================================
# Test: Operators and punctuation
# ============================================================================


class TestOperators:
    """Key operators used in TypeScript code."""

    def test_arrow_function(self) -> None:
        """``=>`` is the arrow function operator."""
        tokens = tokenize_ts30("const f = (x) => x;")
        values = token_values(tokens)
        assert "=>" in values

    def test_question_mark(self) -> None:
        """``?`` is used for optional properties: ``{ name?: string }``."""
        tokens = tokenize_ts30("var x?: string;")
        types = token_types(tokens)
        assert "QUESTION" in types or "?" in token_values(tokens)

    def test_pipe_for_union(self) -> None:
        """``|`` is used for union types: ``string | number``."""
        tokens = tokenize_ts30("type T = string | number;")
        values = token_values(tokens)
        assert "|" in values

    def test_ampersand_for_intersection(self) -> None:
        """``&`` is used for intersection types: ``A & B``."""
        tokens = tokenize_ts30("type T = A & B;")
        values = token_values(tokens)
        assert "&" in values


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS30Lexer:
    """The factory function ``create_ts30_lexer`` should return a usable lexer."""

    def test_creates_lexer_with_tokenize_method(self) -> None:
        lexer = create_ts30_lexer("const x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_and_direct_produce_same_result(self) -> None:
        """``tokenize_ts30`` and ``create_ts30_lexer(...).tokenize()`` must agree."""
        source = "const x: unknown = 42;"
        tokens_direct = tokenize_ts30(source)
        tokens_factory = create_ts30_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_factory_with_empty_source(self) -> None:
        lexer = create_ts30_lexer("")
        tokens = lexer.tokenize()
        assert len(tokens) == 1
        assert token_type_name(tokens[0]) == "EOF"
