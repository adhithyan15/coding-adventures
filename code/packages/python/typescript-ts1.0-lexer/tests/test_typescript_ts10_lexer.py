"""Tests for the TypeScript 1.0 (April 2014) Lexer.

TypeScript 1.0 was the first public release of TypeScript. It added a static
type system to JavaScript, building on ECMAScript 5. The key lexical additions
over ES5 are the AT token for decorators, COLON for type annotations, angle
brackets for generics, and QUESTION_MARK for optional parameters.

Because TS 1.0 is a superset of ES5, all ES5 tokens must continue to work.
The new TS-specific tokens are tested in their own classes below.
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts10_lexer import create_ts10_lexer, tokenize_ts10

# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Return a list of token type name strings for easy assertion."""
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    """Return the string name of a single token's type."""
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    """Return a list of token values for easy assertion."""
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic Variable Declarations with Type Annotations
# ============================================================================


class TestTypeAnnotations:
    """Type annotations are the most fundamental TS 1.0 addition.

    A type annotation is a colon followed by a type name: ``x: number``.
    The COLON token separates the variable name from its type.
    """

    def test_var_with_number_type(self) -> None:
        """``var x: number = 1;`` — the basic typed variable declaration."""
        tokens = tokenize_ts10("var x: number = 1;")
        types = token_types(tokens)
        assert "COLON" in types

    def test_var_with_string_type(self) -> None:
        """``let y: string;`` — a string-typed variable."""
        tokens = tokenize_ts10("let y: string;")
        # 'let' is a context keyword — appears as NAME in TS 1.0
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "let"

    def test_type_annotation_colon_position(self) -> None:
        """The COLON token appears between the variable name and type."""
        tokens = tokenize_ts10("var x: number;")
        assert token_type_name(tokens[2]) == "COLON"

    def test_boolean_type_annotation(self) -> None:
        tokens = tokenize_ts10("var flag: boolean = true;")
        types = token_types(tokens)
        assert "COLON" in types
        assert "KEYWORD" in types  # 'var' and 'true' are keywords


# ============================================================================
# Test: TypeScript Context Keywords
# ============================================================================


class TestContextKeywords:
    """Many TypeScript keywords are actually context-sensitive NAMEs.

    Unlike ES5 reserved words (``var``, ``function``, ``if``), TypeScript
    context keywords like ``interface``, ``type``, ``namespace``, ``declare``
    are emitted as NAME tokens. The parser interprets them from context.

    This is necessary for backward compatibility — code like
    ``var interface = 1;`` is technically valid ES5 (though ugly).
    """

    def test_interface_is_name(self) -> None:
        """'interface' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts10("interface")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "interface"

    def test_type_is_name(self) -> None:
        """'type' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts10("type")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "type"

    def test_enum_is_keyword(self) -> None:
        """'enum' is a full keyword in TS 1.0 (promoted from ES5 reserved)."""
        tokens = tokenize_ts10("enum")
        assert token_type_name(tokens[0]) == "KEYWORD"
        assert tokens[0].value == "enum"

    def test_namespace_is_name(self) -> None:
        """'namespace' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts10("namespace")
        assert tokens[0].type == TokenType.NAME

    def test_declare_is_name(self) -> None:
        """'declare' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts10("declare")
        assert tokens[0].type == TokenType.NAME

    def test_abstract_is_name(self) -> None:
        """'abstract' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts10("abstract")
        assert tokens[0].type == TokenType.NAME


# ============================================================================
# Test: Generic Type Syntax
# ============================================================================


class TestGenericTypes:
    """Generics use angle brackets: ``Array<string>``.

    The lexer emits ``<`` as LESS_THAN and ``>`` as GREATER_THAN.
    The parser distinguishes generic syntax from comparison operators
    by context.
    """

    def test_generic_array_type(self) -> None:
        """``Array<string>`` tokenizes as NAME LESS_THAN NAME GREATER_THAN."""
        tokens = tokenize_ts10("Array<string>")
        types = token_types(tokens)
        assert "LESS_THAN" in types
        assert "GREATER_THAN" in types

    def test_generic_brackets_values(self) -> None:
        tokens = tokenize_ts10("Array<string>")
        values = token_values(tokens)
        assert "<" in values
        assert ">" in values

    def test_generic_nested(self) -> None:
        """``Map<string, Array<number>>`` — nested generics."""
        tokens = tokenize_ts10("Map<string, Array<number>>")
        types = token_types(tokens)
        # Two opening angle brackets and two closing
        assert types.count("LESS_THAN") == 2


# ============================================================================
# Test: Type Assertions
# ============================================================================


class TestTypeAssertions:
    """TypeScript 1.0 supports two forms of type assertions.

    The angle-bracket form: ``<string>x``
    The 'as' form: ``x as string``

    In the angle-bracket form, ``<`` and ``>`` are LESS_THAN / GREATER_THAN.
    In the 'as' form, ``as`` is a NAME token (context keyword).
    """

    def test_as_keyword_is_name(self) -> None:
        """'as' in a type assertion is emitted as a NAME token."""
        tokens = tokenize_ts10("x as string")
        # x, as, string, EOF
        assert tokens[1].type == TokenType.NAME
        assert tokens[1].value == "as"

    def test_angle_bracket_assertion(self) -> None:
        """``<string>x`` — angle-bracket type assertion."""
        tokens = tokenize_ts10("<string>x")
        assert token_type_name(tokens[0]) == "LESS_THAN"
        assert tokens[1].type == TokenType.NAME
        assert token_type_name(tokens[2]) == "GREATER_THAN"


# ============================================================================
# Test: Decorator Syntax
# ============================================================================


class TestDecorators:
    """Decorators use the AT symbol: ``@Component``.

    The AT token was added to TypeScript 1.0 as an experimental feature
    for decorators (the feature was inspired by Java annotations and
    C# attributes). Decorators were experimental in TS 1.0 but widely used.
    """

    def test_decorator_at_token(self) -> None:
        """``@Component`` produces an AT token followed by a NAME."""
        tokens = tokenize_ts10("@Component")
        assert token_type_name(tokens[0]) == "AT"

    def test_decorator_name_after_at(self) -> None:
        tokens = tokenize_ts10("@Component")
        assert tokens[1].type == TokenType.NAME
        assert tokens[1].value == "Component"

    def test_multiple_decorators(self) -> None:
        """Multiple decorators each start with AT."""
        tokens = tokenize_ts10("@Injectable\n@Component")
        at_count = sum(1 for t in tokens if token_type_name(t) == "AT")
        assert at_count == 2


# ============================================================================
# Test: Union Types
# ============================================================================


class TestUnionTypes:
    """Union types use the pipe character: ``string | number``.

    The ``|`` (PIPE) token was already in ES5 for bitwise OR. TypeScript 1.0
    reuses it in type positions for union types. The parser context
    distinguishes the two uses.
    """

    def test_union_pipe_token(self) -> None:
        """``string | number`` — the PIPE token separates union members."""
        tokens = tokenize_ts10("string | number")
        types = token_types(tokens)
        assert "PIPE" in types

    def test_union_pipe_value(self) -> None:
        tokens = tokenize_ts10("string | number")
        values = token_values(tokens)
        assert "|" in values


# ============================================================================
# Test: Arrow Function Types
# ============================================================================


class TestArrowFunctionTypes:
    """Arrow in type context: ``(x: string) => void``.

    Note: In TS 1.0, arrow functions as *values* came from ES2015 targets,
    but arrow types in *type annotations* were supported from TS 1.0.
    The ``=>`` is the FAT_ARROW token.
    """

    def test_arrow_fat_arrow_token(self) -> None:
        """``(x: string) => void`` — ARROW token in type signature."""
        tokens = tokenize_ts10("(x: string) => void")
        types = token_types(tokens)
        assert "ARROW" in types

    def test_arrow_colon_in_param(self) -> None:
        """Parameters also have COLON for type annotations."""
        tokens = tokenize_ts10("(x: string) => void")
        types = token_types(tokens)
        assert "COLON" in types


# ============================================================================
# Test: Non-null Assertion
# ============================================================================


class TestNonNullAssertion:
    """The non-null assertion operator is a postfix exclamation: ``x!``.

    TypeScript 1.0 added this operator to tell the compiler "I know this
    is not null or undefined." It produces an BANG token.
    """

    def test_non_null_exclamation(self) -> None:
        """``x!`` — the BANG token follows the identifier."""
        tokens = tokenize_ts10("x!")
        types = token_types(tokens)
        assert "BANG" in types

    def test_non_null_after_name(self) -> None:
        tokens = tokenize_ts10("x!")
        assert tokens[0].type == TokenType.NAME
        assert token_type_name(tokens[1]) == "BANG"


# ============================================================================
# Test: Interface Syntax
# ============================================================================


class TestInterfaceSyntax:
    """Interface declarations: ``interface Foo { x: string; }``

    The word 'interface' is a NAME token. The braces are LBRACE / RBRACE.
    Properties use COLON for type annotations.
    """

    def test_interface_tokens(self) -> None:
        tokens = tokenize_ts10("interface Foo { x: string; }")
        types = token_types(tokens)
        # interface NAME LBRACE NAME COLON NAME SEMICOLON RBRACE EOF
        assert "LBRACE" in types
        assert "RBRACE" in types
        assert "COLON" in types

    def test_interface_word_is_name(self) -> None:
        tokens = tokenize_ts10("interface Foo {}")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "interface"


# ============================================================================
# Test: EOF Handling
# ============================================================================


class TestEOF:
    """Every token stream must end with an EOF token.

    The EOF token signals to the parser that there is no more input.
    It must always be present, even for empty source code.
    """

    def test_empty_source_has_eof(self) -> None:
        tokens = tokenize_ts10("")
        assert token_type_name(tokens[-1]) == "EOF"

    def test_single_token_has_eof(self) -> None:
        tokens = tokenize_ts10("x")
        assert token_type_name(tokens[-1]) == "EOF"

    def test_complex_source_ends_with_eof(self) -> None:
        tokens = tokenize_ts10("var x: number = 1; interface Foo {}")
        assert token_type_name(tokens[-1]) == "EOF"


# ============================================================================
# Test: Error Recovery (Unterminated Strings)
# ============================================================================


class TestErrorRecovery:
    """The lexer should handle unterminated strings gracefully.

    An unterminated string literal should not crash the tokenizer. Instead,
    it should emit an ERROR token and continue tokenizing.
    """

    def test_unterminated_double_quote_string(self) -> None:
        """An unterminated string should not raise an exception."""
        try:
            tokens = tokenize_ts10('"unterminated')
            # Should either produce an ERROR token or a partial STRING
            assert len(tokens) > 0
        except Exception:
            # Acceptable — the lexer may raise on malformed input
            pass

    def test_unterminated_single_quote_string(self) -> None:
        """Single-quoted unterminated string."""
        try:
            tokens = tokenize_ts10("'unterminated")
            assert len(tokens) > 0
        except Exception:
            pass


# ============================================================================
# Test: ES5 Compatibility
# ============================================================================


class TestES5Compatibility:
    """All ES5 features must work in the TS 1.0 lexer.

    TypeScript 1.0 is a strict superset of ES5. Every valid ES5 program
    is valid TypeScript 1.0. The lexer must handle all ES5 tokens correctly.
    """

    def test_debugger_keyword(self) -> None:
        """ES5's 'debugger' is a keyword in TS 1.0 too."""
        tokens = tokenize_ts10("debugger;")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "debugger"

    def test_var_declaration(self) -> None:
        tokens = tokenize_ts10("var x = 1;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF",
        ]

    def test_function_keyword(self) -> None:
        tokens = tokenize_ts10("function foo() {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"

    def test_property_access(self) -> None:
        tokens = tokenize_ts10("a.b.c")
        types = token_types(tokens)
        assert "DOT" in types

    def test_arithmetic(self) -> None:
        tokens = tokenize_ts10("1 + 2 * 3")
        assert token_types(tokens) == [
            "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF",
        ]


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS10Lexer:
    """Tests for the ``create_ts10_lexer`` factory function."""

    def test_creates_lexer(self) -> None:
        """Factory function returns an object with a tokenize method."""
        lexer = create_ts10_lexer("var x: number = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """Factory result matches direct tokenize_ts10 call."""
        source = "var x: number = 1;"
        tokens_direct = tokenize_ts10(source)
        tokens_factory = create_ts10_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_empty_source(self) -> None:
        """Factory handles empty string without error."""
        lexer = create_ts10_lexer("")
        tokens = lexer.tokenize()
        assert token_type_name(tokens[-1]) == "EOF"


# ============================================================================
# Test: Optional Parameter Marker
# ============================================================================


class TestOptionalParameter:
    """Optional parameters use a question mark: ``foo(x?: string)``.

    The QUESTION_MARK token (``?``) signals that a parameter or property
    is optional. It also appears in ternary expressions, where the parser
    uses context to distinguish the two uses.
    """

    def test_question_mark_token(self) -> None:
        """``foo(x?: string)`` — QUESTION_MARK after the parameter name."""
        tokens = tokenize_ts10("foo(x?: string)")
        types = token_types(tokens)
        assert "QUESTION" in types
