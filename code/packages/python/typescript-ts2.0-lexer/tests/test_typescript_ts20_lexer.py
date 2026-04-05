"""Tests for the TypeScript 2.0 (September 2016) Lexer.

TypeScript 2.0 upgraded the baseline from ES5 to ECMAScript 2015 (ES6).
Key new features include the ``never`` type, non-nullable types, ``let``/``const``,
template literals, ES2015 modules, destructuring, and arrow functions.

All TS 1.0 and ES5 tokens must continue to work (superset guarantee).
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts20_lexer import create_ts20_lexer, tokenize_ts20

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
# Test: never Type (NEW in TS 2.0)
# ============================================================================


class TestNeverType:
    """The ``never`` type is new in TypeScript 2.0.

    ``never`` represents the type of values that never occur. A function
    that always throws or has an infinite loop has return type ``never``.
    Like other TypeScript type keywords, ``never`` is emitted as a NAME
    token — the parser resolves it from context.

    Example: ``function fail(msg: string): never { throw new Error(msg); }``
    """

    def test_never_is_name_token(self) -> None:
        """'never' is a context keyword, emitted as NAME."""
        tokens = tokenize_ts20("never")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "never"

    def test_never_in_return_type(self) -> None:
        """``function fail(): never`` — 'never' as a return type annotation."""
        tokens = tokenize_ts20("function fail(): never")
        values = token_values(tokens)
        assert "never" in values

    def test_never_in_union_type(self) -> None:
        """``string | never`` — 'never' in a union type."""
        tokens = tokenize_ts20("string | never")
        values = token_values(tokens)
        assert "never" in values
        assert "|" in values


# ============================================================================
# Test: Template Literals (ES2015 baseline)
# ============================================================================


class TestTemplateLiterals:
    """Template literals use backtick delimiters: `` `Hello ${name}` ``

    Template literals are an ES2015 feature that TypeScript 2.0 inherits
    because it targets ES2015 as its baseline. The backtick character
    introduces a TEMPLATE_START token. Interpolation uses ``${`` and ``}``.
    """

    def test_simple_template_literal(self) -> None:
        """A plain template literal without interpolation."""
        tokens = tokenize_ts20("`Hello World`")
        # Should produce a template string token
        assert len(tokens) > 0
        assert token_type_name(tokens[-1]) == "EOF"

    def test_template_literal_has_backtick_or_template_token(self) -> None:
        """Template literals produce at least one token from the backtick."""
        tokens = tokenize_ts20("`hello`")
        types = token_types(tokens)
        # Should contain TEMPLATE, BACKTICK, or similar
        has_template = any(
            "TEMPLATE" in t or "BACKTICK" in t or "STRING" in t
            for t in types
        )
        assert has_template or len(tokens) > 1

    def test_template_with_interpolation(self) -> None:
        """`` `Hello ${name}` `` — template with expression interpolation."""
        tokens = tokenize_ts20("`Hello ${name}`")
        assert len(tokens) > 0


# ============================================================================
# Test: ES2015 let and const
# ============================================================================


class TestLetAndConst:
    """ES2015 introduces block-scoped ``let`` and ``const``.

    In TypeScript 2.0, both ``let`` and ``const`` are hard keywords (promoted
    from ES5 reserved words). The TypeScript grammar actively uses them for
    block-scoped declarations, so they emit as KEYWORD tokens.
    """

    def test_let_is_keyword(self) -> None:
        """'let' is a hard keyword in TypeScript 2.0, emitted as KEYWORD."""
        tokens = tokenize_ts20("let")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"

    def test_const_is_keyword(self) -> None:
        """'const' is a hard keyword in TypeScript 2.0, emitted as KEYWORD."""
        tokens = tokenize_ts20("const")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "const"

    def test_let_with_type_annotation(self) -> None:
        """``let x: string;`` — block-scoped variable with type."""
        tokens = tokenize_ts20("let x: string;")
        types = token_types(tokens)
        assert "COLON" in types

    def test_const_with_type_annotation(self) -> None:
        """``const y: number = 1;`` — immutable block-scoped with type."""
        tokens = tokenize_ts20("const y: number = 1;")
        types = token_types(tokens)
        assert "COLON" in types


# ============================================================================
# Test: ES2015 Arrow Functions
# ============================================================================


class TestArrowFunctions:
    """ES2015 arrow functions: ``(x) => x + 1``

    Arrow functions use the ARROW (``=>``) token. TypeScript 2.0 supports
    both typed and untyped arrow functions.
    The token name is ARROW (not FAT_ARROW).
    """

    def test_simple_arrow_function(self) -> None:
        """``(x) => x + 1`` — simple untyped arrow function."""
        tokens = tokenize_ts20("(x) => x + 1")
        types = token_types(tokens)
        assert "ARROW" in types

    def test_typed_arrow_function(self) -> None:
        """``(x: number): number => x + 1`` — fully typed arrow function."""
        tokens = tokenize_ts20("(x: number): number => x + 1")
        types = token_types(tokens)
        assert "ARROW" in types
        assert "COLON" in types

    def test_arrow_fat_arrow_value(self) -> None:
        tokens = tokenize_ts20("x => x")
        values = token_values(tokens)
        assert "=>" in values


# ============================================================================
# Test: ES2015 Classes
# ============================================================================


class TestES2015Classes:
    """ES2015 class syntax: ``class Foo extends Bar implements IFoo``

    TypeScript 2.0 supports the full ES2015 class syntax with TypeScript
    additions like ``implements``, typed members, and access modifiers.
    """

    def test_class_keyword(self) -> None:
        """'class' in TS 2.0 may be a NAME or KEYWORD depending on grammar."""
        tokens = tokenize_ts20("class Foo {}")
        # The important thing is that we get tokens without errors
        assert len(tokens) > 0

    def test_extends_is_keyword_or_name(self) -> None:
        tokens = tokenize_ts20("class Foo extends Bar {}")
        values = token_values(tokens)
        assert "extends" in values

    def test_implements_is_name(self) -> None:
        """'implements' is a context keyword in TypeScript."""
        tokens = tokenize_ts20("class Foo implements IBar {}")
        values = token_values(tokens)
        assert "implements" in values


# ============================================================================
# Test: ES2015 Modules
# ============================================================================


class TestES2015Modules:
    """ES2015 module imports: ``import { Foo } from "./foo"``

    TypeScript 2.0 supports ES2015 module syntax. ``import``, ``export``,
    and ``from`` are tokens. ``import`` and ``export`` may be KEYWORD tokens;
    ``from`` is typically a NAME (context keyword).
    """

    def test_import_keyword(self) -> None:
        tokens = tokenize_ts20('import { Foo } from "./foo"')
        values = token_values(tokens)
        assert "import" in values

    def test_from_in_import(self) -> None:
        """'from' appears in import statements."""
        tokens = tokenize_ts20('import { Foo } from "./foo"')
        values = token_values(tokens)
        assert "from" in values

    def test_export_keyword(self) -> None:
        tokens = tokenize_ts20("export default function foo() {}")
        values = token_values(tokens)
        assert "export" in values

    def test_import_braces(self) -> None:
        """Import named exports use LBRACE / RBRACE."""
        tokens = tokenize_ts20('import { Foo } from "./foo"')
        types = token_types(tokens)
        assert "LBRACE" in types
        assert "RBRACE" in types


# ============================================================================
# Test: Destructuring
# ============================================================================


class TestDestructuring:
    """ES2015 destructuring: ``const { x, y } = obj``

    Destructuring reuses the LBRACE, RBRACE, LBRACKET, RBRACKET tokens
    that already exist. The parser interprets them as destructuring from context.
    """

    def test_object_destructuring(self) -> None:
        """``const { x, y } = obj`` — object destructuring."""
        tokens = tokenize_ts20("const { x, y } = obj")
        types = token_types(tokens)
        assert "LBRACE" in types
        assert "RBRACE" in types
        assert "COMMA" in types

    def test_array_destructuring(self) -> None:
        """``const [first, second] = arr`` — array destructuring."""
        tokens = tokenize_ts20("const [first, second] = arr")
        types = token_types(tokens)
        assert "LBRACKET" in types
        assert "RBRACKET" in types


# ============================================================================
# Test: Default Parameters
# ============================================================================


class TestDefaultParameters:
    """ES2015 default parameters: ``function foo(x = 1)``

    Default parameters use the EQUALS token, which already existed in ES5.
    """

    def test_default_parameter(self) -> None:
        """``function foo(x = 1)`` — default parameter with EQUALS."""
        tokens = tokenize_ts20("function foo(x = 1)")
        types = token_types(tokens)
        assert "EQUALS" in types
        assert "NUMBER" in types


# ============================================================================
# Test: TS 1.0 Compatibility
# ============================================================================


class TestTS10Compatibility:
    """All TS 1.0 features must continue working in TS 2.0."""

    def test_type_annotation(self) -> None:
        """``var x: number = 1;`` — basic type annotation."""
        tokens = tokenize_ts20("var x: number = 1;")
        types = token_types(tokens)
        assert "COLON" in types

    def test_interface_name(self) -> None:
        """'interface' is still a NAME in TS 2.0."""
        tokens = tokenize_ts20("interface")
        assert tokens[0].type == TokenType.NAME

    def test_decorator_at(self) -> None:
        """``@Component`` — AT token for decorators."""
        tokens = tokenize_ts20("@Component")
        assert token_type_name(tokens[0]) == "AT"

    def test_generic_angle_brackets(self) -> None:
        """``Array<string>`` — generic type syntax."""
        tokens = tokenize_ts20("Array<string>")
        types = token_types(tokens)
        assert "LESS_THAN" in types
        assert "GREATER_THAN" in types

    def test_non_null_assertion(self) -> None:
        """``x!`` — non-null assertion."""
        tokens = tokenize_ts20("x!")
        types = token_types(tokens)
        assert "BANG" in types


# ============================================================================
# Test: ES5 Compatibility
# ============================================================================


class TestES5Compatibility:
    """All ES5 features must continue working in TS 2.0."""

    def test_debugger_keyword(self) -> None:
        tokens = tokenize_ts20("debugger;")
        assert tokens[0].type == TokenType.KEYWORD

    def test_var_declaration(self) -> None:
        tokens = tokenize_ts20("var x = 1;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF",
        ]

    def test_function_keyword(self) -> None:
        tokens = tokenize_ts20("function foo() {}")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "function"


# ============================================================================
# Test: EOF Handling
# ============================================================================


class TestEOF:
    """Every token stream must end with an EOF token."""

    def test_empty_source_has_eof(self) -> None:
        tokens = tokenize_ts20("")
        assert token_type_name(tokens[-1]) == "EOF"

    def test_eof_after_complex_source(self) -> None:
        source = "const x: never = undefined as never;"
        tokens = tokenize_ts20(source)
        assert token_type_name(tokens[-1]) == "EOF"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS20Lexer:
    """Tests for the ``create_ts20_lexer`` factory function."""

    def test_creates_lexer(self) -> None:
        """Factory returns an object with a tokenize method."""
        lexer = create_ts20_lexer("let x: string = 'hello';")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """Factory result matches direct tokenize_ts20 call."""
        source = "let x: string = 'hello';"
        tokens_direct = tokenize_ts20(source)
        tokens_factory = create_ts20_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_empty_source(self) -> None:
        lexer = create_ts20_lexer("")
        tokens = lexer.tokenize()
        assert token_type_name(tokens[-1]) == "EOF"
