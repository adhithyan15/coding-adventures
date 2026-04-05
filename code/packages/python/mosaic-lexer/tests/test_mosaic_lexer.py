"""Tests for the Mosaic Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
embedded Mosaic token grammar, correctly tokenizes .mosaic source code.

The Mosaic token grammar exercises several features:

1. **Keywords** — ``component``, ``slot``, ``import``, ``from``, ``as``,
   ``text``, ``number``, ``bool``, ``image``, ``color``, ``node``, ``list``,
   ``true``, ``false``, ``when``, ``each``.

2. **Multi-token identifiers** — Names may contain hyphens, e.g. ``avatar-url``.
   The grammar uses ``[a-zA-Z_][a-zA-Z0-9_-]*`` to handle this.

3. **Dimensions before numbers** — ``DIMENSION`` patterns must be tried before
   ``NUMBER`` because ``16dp`` must not tokenize as ``NUMBER(16)`` + ``NAME(dp)``.

4. **Color hex** — ``#2563eb``, ``#fff``, ``#ff000080`` are distinct from
   ``NUMBER`` and ``DIMENSION``.

5. **String literals** — Double-quoted strings with optional escape sequences.

6. **Comments** — Both ``//`` line comments and ``/* ... */`` block comments
   are skipped (not included in the token stream).

7. **Operators and delimiters** — ``{``, ``}``, ``<``, ``>``, ``:``, ``;``,
   ``,``, ``.``, ``=``, ``@``.

Test Organisation
-----------------

1. Basic token types — NAME, KEYWORD, COLON, SEMICOLON
2. Component declaration structure
3. Slot declarations with various types
4. Import declarations
5. Property assignments — STRING, NUMBER, DIMENSION, COLOR_HEX
6. Node tree with nested elements
7. Slot references using @
8. When and each blocks
9. List types with angle brackets
10. Comment skipping
11. String literals with escape sequences
12. create_lexer factory function
13. EOF token always present
14. Hyphenated identifiers
15. Enum/dotted property values
16. Full component example
"""

from __future__ import annotations

from mosaic_lexer import TOKEN_GRAMMAR, create_lexer, tokenize


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------

def token_types(source: str) -> list[str]:
    """Return only the token type names (excluding EOF) for brevity."""
    tokens = tokenize(source)
    return [t.type_name for t in tokens if t.type_name != "EOF"]


def token_values(source: str) -> list[str]:
    """Return all token values (excluding EOF)."""
    tokens = tokenize(source)
    return [t.value for t in tokens if t.type_name != "EOF"]


# ---------------------------------------------------------------------------
# 1. Basic token types
# ---------------------------------------------------------------------------

class TestBasicTokenTypes:
    """Verify individual token types tokenize correctly."""

    def test_keyword_component(self) -> None:
        tokens = tokenize("component")
        assert tokens[0].type_name == "KEYWORD"
        assert tokens[0].value == "component"

    def test_keyword_slot(self) -> None:
        tokens = tokenize("slot")
        assert tokens[0].type_name == "KEYWORD"
        assert tokens[0].value == "slot"

    def test_name(self) -> None:
        tokens = tokenize("Label")
        assert tokens[0].type_name == "NAME"
        assert tokens[0].value == "Label"

    def test_colon(self) -> None:
        assert "COLON" in token_types(":")

    def test_semicolon(self) -> None:
        assert "SEMICOLON" in token_types(";")

    def test_lbrace_rbrace(self) -> None:
        types = token_types("{}")
        assert types == ["LBRACE", "RBRACE"]

    def test_langle_rangle(self) -> None:
        types = token_types("<>")
        assert types == ["LANGLE", "RANGLE"]

    def test_at(self) -> None:
        assert "AT" in token_types("@title")

    def test_equals(self) -> None:
        assert "EQUALS" in token_types("=")

    def test_dot(self) -> None:
        assert "DOT" in token_types(".")

    def test_comma(self) -> None:
        assert "COMMA" in token_types(",")


# ---------------------------------------------------------------------------
# 2. Component declaration
# ---------------------------------------------------------------------------

class TestComponentDeclaration:
    """Verify a minimal component declaration tokenizes correctly."""

    def test_minimal_component(self) -> None:
        src = "component Label {}"
        types = token_types(src)
        assert types == ["KEYWORD", "NAME", "LBRACE", "RBRACE"]

    def test_component_name_is_name_token(self) -> None:
        tokens = tokenize("component ProfileCard {}")
        name_token = next(t for t in tokens if t.value == "ProfileCard")
        assert name_token.type_name == "NAME"

    def test_component_keyword(self) -> None:
        tokens = tokenize("component X {}")
        assert tokens[0].value == "component"
        assert tokens[0].type_name == "KEYWORD"


# ---------------------------------------------------------------------------
# 3. Slot declarations
# ---------------------------------------------------------------------------

class TestSlotDeclarations:
    """Verify slot declarations tokenize correctly."""

    def test_slot_text(self) -> None:
        types = token_types("slot title: text;")
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]

    def test_slot_number(self) -> None:
        types = token_types("slot count: number;")
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]

    def test_slot_bool(self) -> None:
        types = token_types("slot visible: bool;")
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]

    def test_slot_with_default_number(self) -> None:
        types = token_types("slot count: number = 0;")
        assert types == [
            "KEYWORD", "NAME", "COLON", "KEYWORD",
            "EQUALS", "NUMBER", "SEMICOLON",
        ]

    def test_slot_with_default_true(self) -> None:
        values = token_values("slot visible: bool = true;")
        assert "true" in values

    def test_slot_with_default_false(self) -> None:
        values = token_values("slot visible: bool = false;")
        assert "false" in values

    def test_slot_image_type(self) -> None:
        types = token_types("slot avatar: image;")
        assert "KEYWORD" in types

    def test_slot_color_type(self) -> None:
        types = token_types("slot bg: color;")
        assert "KEYWORD" in types

    def test_slot_node_type(self) -> None:
        types = token_types("slot content: node;")
        assert "KEYWORD" in types


# ---------------------------------------------------------------------------
# 4. Import declarations
# ---------------------------------------------------------------------------

class TestImportDeclarations:
    """Verify import declaration tokenizes correctly."""

    def test_import_simple(self) -> None:
        src = 'import Button from "./button.mosaic";'
        types = token_types(src)
        assert types[0] == "KEYWORD"  # import
        assert types[1] == "NAME"     # Button
        assert "STRING" in types

    def test_import_with_alias(self) -> None:
        src = 'import Card as InfoCard from "./card.mosaic";'
        values = token_values(src)
        assert "Card" in values
        assert "as" in values
        assert "InfoCard" in values


# ---------------------------------------------------------------------------
# 5. Property assignments
# ---------------------------------------------------------------------------

class TestPropertyAssignments:
    """Verify property value tokens are correct."""

    def test_string_value(self) -> None:
        types = token_types('content: "Hello";')
        assert "STRING" in types

    def test_number_value(self) -> None:
        types = token_types("padding: 16;")
        assert "NUMBER" in types

    def test_dimension_value(self) -> None:
        types = token_types("padding: 16dp;")
        assert "DIMENSION" in types
        assert "NUMBER" not in types

    def test_dimension_percent(self) -> None:
        types = token_types("width: 100%;")
        assert "DIMENSION" in types

    def test_dimension_sp(self) -> None:
        types = token_types("font-size: 18sp;")
        assert "DIMENSION" in types

    def test_color_hex_6(self) -> None:
        types = token_types("background: #2563eb;")
        assert "COLOR_HEX" in types

    def test_color_hex_3(self) -> None:
        types = token_types("color: #fff;")
        assert "COLOR_HEX" in types

    def test_color_hex_8(self) -> None:
        types = token_types("border: #ff000080;")
        assert "COLOR_HEX" in types

    def test_name_as_value(self) -> None:
        types = token_types("align: center;")
        assert "NAME" in types


# ---------------------------------------------------------------------------
# 6. Nested node tree
# ---------------------------------------------------------------------------

class TestNodeTree:
    """Verify nested node structures tokenize correctly."""

    def test_simple_node(self) -> None:
        src = "Text { content: @title; }"
        types = token_types(src)
        assert types[0] == "NAME"   # Text
        assert "LBRACE" in types
        assert "RBRACE" in types

    def test_nested_nodes(self) -> None:
        src = "Column { Text { content: @title; } }"
        types = token_types(src)
        assert types.count("LBRACE") == 2
        assert types.count("RBRACE") == 2


# ---------------------------------------------------------------------------
# 7. Slot references
# ---------------------------------------------------------------------------

class TestSlotReferences:
    """Verify @slotName tokenizes as AT + NAME."""

    def test_at_name_pair(self) -> None:
        types = token_types("@title")
        assert types == ["AT", "NAME"]

    def test_at_hyphenated_name(self) -> None:
        types = token_types("@avatar-url")
        assert types == ["AT", "NAME"]
        values = token_values("@avatar-url")
        assert "avatar-url" in values


# ---------------------------------------------------------------------------
# 8. When and each blocks
# ---------------------------------------------------------------------------

class TestWhenEachBlocks:
    """Verify when and each keywords tokenize correctly."""

    def test_when_keyword(self) -> None:
        tokens = tokenize("when")
        assert tokens[0].type_name == "KEYWORD"
        assert tokens[0].value == "when"

    def test_each_keyword(self) -> None:
        tokens = tokenize("each")
        assert tokens[0].type_name == "KEYWORD"
        assert tokens[0].value == "each"

    def test_when_block_structure(self) -> None:
        src = "when @visible { Text {} }"
        types = token_types(src)
        assert types[0] == "KEYWORD"  # when
        assert types[1] == "AT"
        assert types[2] == "NAME"


# ---------------------------------------------------------------------------
# 9. List types with angle brackets
# ---------------------------------------------------------------------------

class TestListTypes:
    """Verify list<T> syntax tokenizes correctly."""

    def test_list_text(self) -> None:
        types = token_types("list<text>")
        assert types == ["KEYWORD", "LANGLE", "KEYWORD", "RANGLE"]

    def test_list_component(self) -> None:
        types = token_types("list<Button>")
        assert types == ["KEYWORD", "LANGLE", "NAME", "RANGLE"]


# ---------------------------------------------------------------------------
# 10. Comment skipping
# ---------------------------------------------------------------------------

class TestCommentSkipping:
    """Verify comments are not included in the token stream."""

    def test_line_comment_skipped(self) -> None:
        src = "// this is a comment\nslot x: text;"
        types = token_types(src)
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]

    def test_block_comment_skipped(self) -> None:
        src = "/* block comment */ slot x: text;"
        types = token_types(src)
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]

    def test_inline_block_comment(self) -> None:
        src = "slot /* mid */ x: text;"
        types = token_types(src)
        assert types == ["KEYWORD", "NAME", "COLON", "KEYWORD", "SEMICOLON"]


# ---------------------------------------------------------------------------
# 11. String literals
# ---------------------------------------------------------------------------

class TestStringLiterals:
    """Verify string literal tokenization."""

    def test_simple_string(self) -> None:
        tokens = tokenize('"hello"')
        assert tokens[0].type_name == "STRING"
        # The lexer strips surrounding quotes when escape_mode='standard'
        assert tokens[0].value == "hello"

    def test_empty_string(self) -> None:
        tokens = tokenize('""')
        assert tokens[0].type_name == "STRING"
        # Empty string — quotes stripped, value is empty
        assert tokens[0].value == ""

    def test_path_string(self) -> None:
        tokens = tokenize('"./button.mosaic"')
        assert tokens[0].type_name == "STRING"


# ---------------------------------------------------------------------------
# 12. create_lexer factory
# ---------------------------------------------------------------------------

class TestCreateLexer:
    """Verify create_lexer() factory function."""

    def test_create_lexer_produces_tokens(self) -> None:
        lexer = create_lexer("slot x: text;")
        tokens = lexer.tokenize()
        assert len(tokens) > 0

    def test_create_lexer_matches_tokenize(self) -> None:
        src = "component X { slot y: number; }"
        from_factory = [t.type_name for t in create_lexer(src).tokenize()]
        from_shortcut = [t.type_name for t in tokenize(src)]
        assert from_factory == from_shortcut


# ---------------------------------------------------------------------------
# 13. EOF token
# ---------------------------------------------------------------------------

class TestEOFToken:
    """Verify EOF is always the last token."""

    def test_eof_present_empty_source(self) -> None:
        tokens = tokenize("")
        assert tokens[-1].type_name == "EOF"

    def test_eof_present_nonempty_source(self) -> None:
        tokens = tokenize("component Label {}")
        assert tokens[-1].type_name == "EOF"


# ---------------------------------------------------------------------------
# 14. Hyphenated identifiers
# ---------------------------------------------------------------------------

class TestHyphenatedIdentifiers:
    """Verify that kebab-case names tokenize as a single NAME token."""

    def test_avatar_url(self) -> None:
        tokens = tokenize("avatar-url")
        assert tokens[0].type_name == "NAME"
        assert tokens[0].value == "avatar-url"

    def test_corner_radius(self) -> None:
        tokens = tokenize("corner-radius")
        assert tokens[0].type_name == "NAME"
        assert tokens[0].value == "corner-radius"

    def test_font_size(self) -> None:
        tokens = tokenize("font-size")
        assert tokens[0].type_name == "NAME"
        assert tokens[0].value == "font-size"


# ---------------------------------------------------------------------------
# 15. Enum/dotted property values
# ---------------------------------------------------------------------------

class TestEnumDottedValues:
    """Verify dotted enum values like 'heading.large' tokenize as NAME DOT NAME."""

    def test_enum_value(self) -> None:
        types = token_types("heading.large")
        assert types == ["NAME", "DOT", "NAME"]

    def test_enum_in_property(self) -> None:
        types = token_types("style: heading.small;")
        assert "DOT" in types


# ---------------------------------------------------------------------------
# 16. Full component example
# ---------------------------------------------------------------------------

class TestFullComponent:
    """Verify a realistic full component tokenizes without errors."""

    def test_full_component(self) -> None:
        src = """
        component ProfileCard {
            slot avatar-url: image;
            slot display-name: text;
            slot follower-count: number = 0;

            Column {
                Image { source: @avatar-url; }
                Text { content: @display-name; font-size: 18sp; }
            }
        }
        """
        tokens = tokenize(src)
        types = [t.type_name for t in tokens]
        assert "KEYWORD" in types
        assert "NAME" in types
        assert "COLON" in types
        assert "SEMICOLON" in types
        assert types[-1] == "EOF"

    def test_token_grammar_accessible(self) -> None:
        assert TOKEN_GRAMMAR is not None
        assert hasattr(TOKEN_GRAMMAR, "keywords")
        assert "component" in TOKEN_GRAMMAR.keywords
