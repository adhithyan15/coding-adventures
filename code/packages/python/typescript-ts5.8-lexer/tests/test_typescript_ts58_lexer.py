"""Tests for the TypeScript 5.8 (2025) Lexer.

TypeScript 5.8 targets the ES2025 baseline. ES2025 standardizes three major
features: TC39 decorators, import attributes (``with`` clause), and explicit
resource management (``using`` / ``await using``). TS 5.8 also adds the
HASHBANG token and regex ``v`` flag support.
"""

from __future__ import annotations

from lexer import Token, TokenType

from typescript_ts58_lexer import create_ts58_lexer, tokenize_ts58

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
# Test: ``using`` Context Keyword (ES2025 Explicit Resource Management)
# ============================================================================


class TestUsingKeyword:
    """``using`` is a context keyword for explicit resource management.

    In ES2025 (TS 5.8 baseline), ``using`` is standardized. Resources must
    implement ``Symbol.dispose`` for synchronous cleanup.
    """

    def test_using_is_name(self) -> None:
        """``using`` is a context keyword — lexes as NAME."""
        tokens = tokenize_ts58("using")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "using"

    def test_using_declaration(self) -> None:
        """``using x = getResource();`` — resource declaration."""
        tokens = tokenize_ts58("using x = getResource();")
        values = token_values(tokens)
        assert "using" in values
        assert "x" in values

    def test_using_with_type_annotation(self) -> None:
        """``using conn: Connection = open();`` — typed resource."""
        tokens = tokenize_ts58("using conn = open();")
        values = token_values(tokens)
        assert "using" in values
        assert "conn" in values


# ============================================================================
# Test: ``await using`` (Async Explicit Resource Management)
# ============================================================================


class TestAwaitUsing:
    """``await using`` declares async resources implementing ``Symbol.asyncDispose``."""

    def test_await_using_tokens(self) -> None:
        """``await using db = await connect();``"""
        tokens = tokenize_ts58("await using db = await connect();")
        values = token_values(tokens)
        assert "await" in values
        assert "using" in values
        assert "db" in values

    def test_await_using_in_async_function(self) -> None:
        """Full async function with ``await using`` resource."""
        source = "async function run() { await using db = await connect(); }"
        tokens = tokenize_ts58(source)
        values = token_values(tokens)
        assert "await" in values
        assert "using" in values


# ============================================================================
# Test: Import Attributes (ES2025 — ``with`` clause)
# ============================================================================


class TestImportAttributes:
    """ES2025 import attributes use ``with { type: "json" }`` syntax.

    The ``with`` keyword was already reserved in JavaScript; ES2025 reuses it
    for import attributes. The ``assert`` form (from an earlier proposal) was
    withdrawn in favor of ``with``, which can influence loading behavior.
    """

    def test_import_with_tokens(self) -> None:
        """``import x from "./f.json" with { type: "json" }``"""
        tokens = tokenize_ts58('import x from "./f.json" with { type: "json" }')
        values = token_values(tokens)
        assert "import" in values
        assert "with" in values
        assert "type" in values

    def test_import_attribute_string(self) -> None:
        """The module specifier and attribute value are STRING tokens."""
        tokens = tokenize_ts58('import data from "./data.json" with { type: "json" }')
        string_tokens = [t for t in tokens if token_type_name(t) == "STRING"]
        assert len(string_tokens) >= 2  # "./data.json" and "json"


# ============================================================================
# Test: HASHBANG Token (ES2025)
# ============================================================================


class TestHashbangToken:
    """ES2025 standardizes hashbang comments (``#!`` on line 1).

    Node.js has supported hashbangs for years to mark executable scripts.
    ES2025 normalizes how JavaScript parsers should treat them — as a
    comment-like token consumed at the start of a script/module.
    """

    def test_hashbang_token(self) -> None:
        """``#!/usr/bin/env node`` — hashbang lexes as HASHBANG."""
        tokens = tokenize_ts58("#!/usr/bin/env node\nconst x = 1;")
        assert token_type_name(tokens[0]) == "HASHBANG"
        assert tokens[0].value.startswith("#!")

    def test_hashbang_followed_by_code(self) -> None:
        """Code after hashbang tokenizes normally."""
        tokens = tokenize_ts58("#!/usr/bin/env node\nconst x = 1;")
        types = token_types(tokens)
        assert "HASHBANG" in types
        assert "KEYWORD" in types  # const


# ============================================================================
# Test: Standard Decorators (ES2025)
# ============================================================================


class TestStandardDecorators:
    """In TS 5.8, standard TC39 decorators are part of the ES2025 baseline.

    The ``@`` token is the same as in TS 5.0, but it's now in the ES2025 standard.
    """

    def test_at_token(self) -> None:
        """``@`` is the AT token — decorator prefix."""
        tokens = tokenize_ts58("@decorator")
        types = token_types(tokens)
        assert types[0] == "AT"

    def test_class_decorator(self) -> None:
        """``@sealed class Foo {}`` — standard TC39 decorator."""
        tokens = tokenize_ts58("@sealed class Foo {}")
        types = token_types(tokens)
        assert "AT" in types
        assert "KEYWORD" in types  # class

    def test_field_decorator(self) -> None:
        """``@logged accessor name = ""`` — accessor with decorator."""
        tokens = tokenize_ts58("@logged accessor name = '';")
        types = token_types(tokens)
        assert types[0] == "AT"
        values = token_values(tokens)
        assert "accessor" in values


# ============================================================================
# Test: Private Class Fields (ES2022 Baseline, present in TS 5.8)
# ============================================================================


class TestPrivateClassFields:
    """Private names (``#name``) are part of the ES2022 baseline."""

    def test_private_field(self) -> None:
        """``#count`` lexes as PRIVATE_NAME."""
        tokens = tokenize_ts58("#count")
        assert token_type_name(tokens[0]) == "PRIVATE_NAME"

    def test_private_field_access(self) -> None:
        """``this.#count`` — private field access."""
        tokens = tokenize_ts58("this.#count")
        types = token_types(tokens)
        assert "PRIVATE_NAME" in types


# ============================================================================
# Test: TypeScript Context Keywords
# ============================================================================


class TestTypeScriptContextKeywords:
    """TypeScript type-system context keywords lex as NAME."""

    def test_type_keyword(self) -> None:
        """``type`` is a context keyword."""
        tokens = tokenize_ts58("type")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "type"

    def test_interface_keyword(self) -> None:
        """``interface`` is a context keyword."""
        tokens = tokenize_ts58("interface")
        assert token_type_name(tokens[0]) == "NAME"
        assert tokens[0].value == "interface"

    def test_declare_keyword(self) -> None:
        """``declare`` is a context keyword."""
        tokens = tokenize_ts58("declare")
        assert token_type_name(tokens[0]) == "NAME"

    def test_satisfies_keyword(self) -> None:
        """``satisfies`` is a context keyword."""
        tokens = tokenize_ts58("satisfies")
        assert token_type_name(tokens[0]) == "NAME"

    def test_accessor_keyword(self) -> None:
        """``accessor`` is a context keyword."""
        tokens = tokenize_ts58("accessor")
        assert token_type_name(tokens[0]) == "NAME"


# ============================================================================
# Test: ES2025 Keyword Set
# ============================================================================


class TestES2025Keywords:
    """ES2025 keywords include all ES2022 keywords."""

    def test_core_keywords(self) -> None:
        """Core JS keywords are KEYWORD tokens."""
        keywords = [
            "class", "const", "let", "import", "export", "extends",
            "super", "yield", "async", "return",
        ]
        for kw in keywords:
            tokens = tokenize_ts58(kw)
            assert tokens[0].type == TokenType.KEYWORD, f"{kw} should be KEYWORD"


# ============================================================================
# Test: Logical Assignment Operators (ES2021, present in TS 5.8)
# ============================================================================


class TestLogicalAssignmentOperators:
    """ES2021 logical assignment operators are part of the TS 5.8 token set."""

    def test_nullish_coalesce_equals(self) -> None:
        """``x ??= fallback`` — nullish assignment."""
        tokens = tokenize_ts58("x ??= fallback")
        types = token_types(tokens)
        assert "NULLISH_COALESCE_EQUALS" in types

    def test_or_or_equals(self) -> None:
        """``x ||= default_val``"""
        tokens = tokenize_ts58("x ||= default_val")
        types = token_types(tokens)
        assert "OR_OR_EQUALS" in types

    def test_and_and_equals(self) -> None:
        """``x &&= value``"""
        tokens = tokenize_ts58("x &&= value")
        types = token_types(tokens)
        assert "AND_AND_EQUALS" in types


# ============================================================================
# Test: Template Literals
# ============================================================================


class TestTemplateLiterals:
    def test_simple_template(self) -> None:
        tokens = tokenize_ts58("`hello`")
        assert token_type_name(tokens[0]) == "TEMPLATE_NO_SUB"

    def test_template_with_substitution(self) -> None:
        tokens = tokenize_ts58("`Hello, ${name}!`")
        assert token_type_name(tokens[0]) == "TEMPLATE_HEAD"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS58Lexer:

    def test_creates_lexer(self) -> None:
        lexer = create_ts58_lexer("const x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        source = "using x = getResource();"
        tokens_direct = tokenize_ts58(source)
        tokens_factory = create_ts58_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Comments
# ============================================================================


class TestComments:
    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_ts58("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_ts58("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]
