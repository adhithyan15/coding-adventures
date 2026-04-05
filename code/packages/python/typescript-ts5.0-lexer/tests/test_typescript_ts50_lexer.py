"""Tests for the TypeScript 5.0 (2023) Lexer.

TypeScript 5.0 targets the ES2022 baseline and adds standard TC39 decorators,
const type parameters, the ``accessor`` keyword, and ``satisfies`` operator.
The ``using`` context keyword is lexically recognized for TS 5.2 compatibility.
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts50_lexer import create_ts50_lexer, tokenize_ts50

# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    return [t.value for t in tokens]


# ============================================================================
# Test: Standard Decorators (NEW in TS 5.0)
# ============================================================================


class TestStandardDecorators:
    """TS 5.0 adopts the TC39 Stage-3 decorator proposal by default.

    The ``@`` token was already in TS 4.x via --experimentalDecorators.
    TS 5.0 makes the standard proposal the default without the flag.
    """

    def test_at_token(self) -> None:
        """The ``@`` sign lexes as the AT token."""
        tokens = tokenize_ts50("@decorator")
        # AT token, then the decorator name as NAME
        types = token_types(tokens)
        assert "AT" in types or types[0] in ("AT",)

    def test_class_decorator(self) -> None:
        """``@sealed class Foo {}`` — decorator on a class."""
        tokens = tokenize_ts50("@sealed class Foo {}")
        types = token_types(tokens)
        assert "AT" in types
        assert "KEYWORD" in types  # class is a keyword
        assert tokens[-1].type == TokenType.EOF

    def test_field_decorator_with_accessor(self) -> None:
        """``accessor`` is a context keyword used with decorators."""
        tokens = tokenize_ts50("accessor name = '';")
        values = token_values(tokens)
        assert "accessor" in values

    def test_decorator_with_arguments(self) -> None:
        """``@Component({selector: 'app-root'})`` — decorator with args."""
        tokens = tokenize_ts50("@Component({selector: 'app-root'})")
        types = token_types(tokens)
        assert types[0] == "AT"
        assert "LPAREN" in types
        assert "RBRACE" in types


# ============================================================================
# Test: Private Class Fields (ES2022 Baseline)
# ============================================================================


class TestPrivateClassFields:
    """ES2022 private names start with ``#``.

    These are TRULY private (runtime enforced), unlike TypeScript's ``private``
    modifier which is only enforced at compile-time.
    """

    def test_private_field_name(self) -> None:
        """``#count`` lexes as PRIVATE_NAME."""
        tokens = tokenize_ts50("#count")
        assert token_type_name(tokens[0]) == "PRIVATE_NAME"
        assert tokens[0].value == "#count"

    def test_private_field_in_class(self) -> None:
        """``this.#count`` accesses a private field through ``this``."""
        tokens = tokenize_ts50("this.#count")
        types = token_types(tokens)
        assert "PRIVATE_NAME" in types

    def test_private_name_dollar(self) -> None:
        """Private names can include ``$`` like regular identifiers."""
        tokens = tokenize_ts50("#$field")
        assert token_type_name(tokens[0]) == "PRIVATE_NAME"

    def test_private_method_name(self) -> None:
        """``#validate()`` — private method call."""
        tokens = tokenize_ts50("#validate()")
        assert token_type_name(tokens[0]) == "PRIVATE_NAME"
        assert tokens[0].value == "#validate"


# ============================================================================
# Test: Class Static Blocks (ES2022 Baseline)
# ============================================================================


class TestClassStaticBlocks:
    """Static initialization blocks run once when the class is defined.

    They allow complex initialization logic including try/catch, loops,
    and access to private static members.
    """

    def test_static_keyword(self) -> None:
        """``static`` is a context keyword in TS 5.0."""
        tokens = tokenize_ts50("static")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "static"

    def test_static_block_tokens(self) -> None:
        """``static { init(); }`` — static init block token sequence."""
        tokens = tokenize_ts50("static { init(); }")
        values = token_values(tokens)
        assert "static" in values
        assert "{" in values
        assert "}" in values


# ============================================================================
# Test: ``using`` Resource Management Context Keyword (TS 5.2)
# ============================================================================


class TestUsingKeyword:
    """``using`` is a context keyword for explicit resource management.

    Introduced in TS 5.2, it's lexically recognized in the TS 5.0 token grammar
    for forward compatibility. Resources implement ``Symbol.dispose``.
    """

    def test_using_is_name(self) -> None:
        """``using`` is a context keyword — lexes as NAME."""
        tokens = tokenize_ts50("using")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "using"

    def test_using_declaration(self) -> None:
        """``using x = getResource();`` — resource declaration."""
        tokens = tokenize_ts50("using x = getResource();")
        values = token_values(tokens)
        assert "using" in values
        assert "x" in values

    def test_await_using(self) -> None:
        """``await using`` — async resource management."""
        tokens = tokenize_ts50("await using db = connect();")
        values = token_values(tokens)
        assert "await" in values
        assert "using" in values


# ============================================================================
# Test: Template Literals with Expressions
# ============================================================================


class TestTemplateLiterals:
    """Template literals support embedded expressions via ``${...}``."""

    def test_simple_template(self) -> None:
        """Simple template string with no substitutions."""
        tokens = tokenize_ts50("`hello world`")
        assert token_type_name(tokens[0]) == "TEMPLATE_NO_SUB"

    def test_template_with_expression(self) -> None:
        r"""``\`Hello, ${name}!\`` — template with substitution."""
        tokens = tokenize_ts50("`Hello, ${name}!`")
        # Should produce TEMPLATE_HEAD token (the part before ${)
        assert token_type_name(tokens[0]) == "TEMPLATE_HEAD"


# ============================================================================
# Test: ``satisfies`` Operator (TS 4.9, fully integrated in 5.0)
# ============================================================================


class TestSatisfiesOperator:
    """``satisfies`` is a TS context keyword for type checking without widening.

    ``expr satisfies Type`` validates that ``expr`` is compatible with ``Type``
    but preserves the more specific inferred type of ``expr``.

    Example::

        const palette = {
            red: [255, 0, 0],
            green: "#00ff00",
        } satisfies Record<string, string | number[]>;
    """

    def test_satisfies_is_name(self) -> None:
        """``satisfies`` lexes as NAME (context keyword)."""
        tokens = tokenize_ts50("satisfies")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "satisfies"

    def test_satisfies_expression(self) -> None:
        """``x satisfies Type`` — validate type compatibility."""
        tokens = tokenize_ts50("x satisfies SomeType")
        values = token_values(tokens)
        assert "satisfies" in values


# ============================================================================
# Test: ``accessor`` Keyword (TS 5.0)
# ============================================================================


class TestAccessorKeyword:
    """``accessor`` is a context keyword for auto-accessor class members.

    An auto-accessor generates a getter/setter pair backed by a private
    storage field. Particularly useful with decorators.

    Example::

        class Foo {
            accessor name: string = "";
        }

    Compiles to a class with a private backing field and
    public ``get name()`` / ``set name()`` methods.
    """

    def test_accessor_is_name(self) -> None:
        """``accessor`` lexes as NAME (context keyword)."""
        tokens = tokenize_ts50("accessor")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "accessor"

    def test_accessor_field(self) -> None:
        """``accessor x = 1;`` — auto-accessor field declaration."""
        tokens = tokenize_ts50("accessor x = 1;")
        values = token_values(tokens)
        assert "accessor" in values
        assert "x" in values


# ============================================================================
# Test: Logical Assignment Operators (ES2021)
# ============================================================================


class TestLogicalAssignmentOperators:
    """ES2021 logical assignment operators combine a logical op with assignment.

    These are the three forms:
    - ``||=``  (OR_OR_EQUALS): assign only if left side is falsy
    - ``&&=``  (AND_AND_EQUALS): assign only if left side is truthy
    - ``??=``  (NULLISH_COALESCE_EQUALS): assign only if left side is nullish
    """

    def test_or_or_equals(self) -> None:
        """``x ||= default`` — assign if falsy."""
        tokens = tokenize_ts50("x ||= default_val")
        types = token_types(tokens)
        assert "OR_OR_EQUALS" in types

    def test_and_and_equals(self) -> None:
        """``x &&= value`` — assign if truthy."""
        tokens = tokenize_ts50("x &&= value")
        types = token_types(tokens)
        assert "AND_AND_EQUALS" in types

    def test_nullish_coalesce_equals(self) -> None:
        """``x ??= fallback`` — assign if nullish (null or undefined)."""
        tokens = tokenize_ts50("x ??= fallback")
        types = token_types(tokens)
        assert "NULLISH_COALESCE_EQUALS" in types


# ============================================================================
# Test: TypeScript Keywords
# ============================================================================


class TestTypeScriptKeywords:
    """TypeScript adds several ES2022 keywords on top of the baseline."""

    def test_es2022_keywords(self) -> None:
        """Core ES2022 keywords are recognized."""
        keywords = ["class", "const", "import", "export", "extends", "super", "let"]
        for kw in keywords:
            tokens = tokenize_ts50(kw)
            assert tokens[0].type == TokenType.KEYWORD, f"{kw} should be KEYWORD"

    def test_typescript_context_keywords(self) -> None:
        """TypeScript type-system context keywords lex as NAME."""
        context_kws = [
            "type", "interface", "namespace", "declare", "abstract",
            "readonly", "keyof", "infer", "never", "unknown", "override",
        ]
        for kw in context_kws:
            tokens = tokenize_ts50(kw)
            assert token_type_name(tokens[0]) == "NAME", f"{kw} should be NAME"
            assert tokens[0].value == kw

    def test_const_type_parameter_tokens(self) -> None:
        """``<const T>`` — const type parameter syntax."""
        tokens = tokenize_ts50("function id<const T>(x: T): T { return x; }")
        values = token_values(tokens)
        assert "const" in values


# ============================================================================
# Test: Numeric Separators (ES2021)
# ============================================================================


class TestNumericSeparators:
    """ES2021 allows ``_`` as a visual separator in numeric literals."""

    def test_integer_with_separator(self) -> None:
        """``1_000_000`` — thousand separator."""
        tokens = tokenize_ts50("1_000_000")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_hex_with_separator(self) -> None:
        """``0xFF_FF`` — hex with separator."""
        tokens = tokenize_ts50("0xFF_FF")
        assert token_type_name(tokens[0]) == "NUMBER"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS50Lexer:

    def test_creates_lexer(self) -> None:
        lexer = create_ts50_lexer("const x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        source = "const x: number = 1;"
        tokens_direct = tokenize_ts50(source)
        tokens_factory = create_ts50_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Comments
# ============================================================================


class TestComments:
    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_ts50("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_ts50("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]
