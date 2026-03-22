"""Tests for the XML Lexer.

These tests verify that the XML lexer correctly tokenizes XML documents
using pattern groups and the on-token callback. The tests cover:

1. **Basic elements** — open/close tags, text content
2. **Attributes** — single and double quoted values
3. **Self-closing tags** — ``<br/>``
4. **Comments** — ``<!-- ... -->``
5. **CDATA sections** — ``<![CDATA[ ... ]]>``
6. **Processing instructions** — ``<?xml ... ?>``
7. **Entity references** — ``&amp;``, ``&#65;``, ``&#x41;``
8. **Nested structures** — tags within tags
9. **Mixed content** — text interspersed with elements
10. **Edge cases** — empty elements, whitespace handling
"""

from __future__ import annotations

from xml_lexer import tokenize_xml

# ---------------------------------------------------------------------------
# Helper — extract (type_name, value) pairs, filtering EOF
# ---------------------------------------------------------------------------


def token_pairs(source: str) -> list[tuple[str, str]]:
    """Tokenize XML and return (type_name, value) pairs without EOF."""
    tokens = tokenize_xml(source)
    result = []
    for t in tokens:
        name = t.type if isinstance(t.type, str) else t.type.name
        if name == "EOF":
            continue
        result.append((name, t.value))
    return result


def token_types(source: str) -> list[str]:
    """Tokenize XML and return just the type names without EOF."""
    return [name for name, _ in token_pairs(source)]


# ===========================================================================
# Basic Tags
# ===========================================================================


class TestBasicTags:
    """Test tokenization of simple XML tags and text content."""

    def test_simple_element(self) -> None:
        """A simple element: <p>text</p>."""
        pairs = token_pairs("<p>text</p>")
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "p"),
            ("TAG_CLOSE", ">"),
            ("TEXT", "text"),
            ("CLOSE_TAG_START", "</"),
            ("TAG_NAME", "p"),
            ("TAG_CLOSE", ">"),
        ]

    def test_element_with_namespace(self) -> None:
        """Tags with XML namespace prefixes: <ns:tag>."""
        types = token_types("<ns:tag>content</ns:tag>")
        assert types == [
            "OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
            "TEXT",
            "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE",
        ]
        pairs = token_pairs("<ns:tag>content</ns:tag>")
        assert pairs[1] == ("TAG_NAME", "ns:tag")

    def test_empty_element_explicit(self) -> None:
        """An explicitly empty element: <div></div>."""
        pairs = token_pairs("<div></div>")
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "div"),
            ("TAG_CLOSE", ">"),
            ("CLOSE_TAG_START", "</"),
            ("TAG_NAME", "div"),
            ("TAG_CLOSE", ">"),
        ]

    def test_self_closing_tag(self) -> None:
        """Self-closing tag: <br/>."""
        pairs = token_pairs("<br/>")
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "br"),
            ("SELF_CLOSE", "/>"),
        ]

    def test_self_closing_with_space(self) -> None:
        """Self-closing with space: <br />."""
        pairs = token_pairs("<br />")
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "br"),
            ("SELF_CLOSE", "/>"),
        ]


# ===========================================================================
# Attributes
# ===========================================================================


class TestAttributes:
    """Test tokenization of tag attributes."""

    def test_double_quoted_attribute(self) -> None:
        """Attribute with double quotes: class="main"."""
        pairs = token_pairs('<div class="main">')
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "div"),
            ("TAG_NAME", "class"),
            ("ATTR_EQUALS", "="),
            ("ATTR_VALUE", '"main"'),
            ("TAG_CLOSE", ">"),
        ]

    def test_single_quoted_attribute(self) -> None:
        """Attribute with single quotes: class='main'."""
        pairs = token_pairs("<div class='main'>")
        assert pairs == [
            ("OPEN_TAG_START", "<"),
            ("TAG_NAME", "div"),
            ("TAG_NAME", "class"),
            ("ATTR_EQUALS", "="),
            ("ATTR_VALUE", "'main'"),
            ("TAG_CLOSE", ">"),
        ]

    def test_multiple_attributes(self) -> None:
        """Multiple attributes on one tag."""
        pairs = token_pairs('<a href="url" target="_blank">')
        tag_names = [v for t, v in pairs if t == "TAG_NAME"]
        assert tag_names == ["a", "href", "target"]
        attr_values = [v for t, v in pairs if t == "ATTR_VALUE"]
        assert attr_values == ['"url"', '"_blank"']

    def test_attribute_on_self_closing(self) -> None:
        """Attribute on a self-closing tag."""
        types = token_types('<img src="photo.jpg"/>')
        assert "SELF_CLOSE" in types
        assert "ATTR_VALUE" in types


# ===========================================================================
# Comments
# ===========================================================================


class TestComments:
    """Test tokenization of XML comments."""

    def test_simple_comment(self) -> None:
        """A simple comment: <!-- text -->."""
        pairs = token_pairs("<!-- hello -->")
        assert pairs == [
            ("COMMENT_START", "<!--"),
            ("COMMENT_TEXT", " hello "),
            ("COMMENT_END", "-->"),
        ]

    def test_comment_preserves_whitespace(self) -> None:
        """Whitespace inside comments is preserved (skip disabled)."""
        pairs = token_pairs("<!--  spaces  and\ttabs  -->")
        text = [v for t, v in pairs if t == "COMMENT_TEXT"]
        assert text == ["  spaces  and\ttabs  "]

    def test_comment_with_dashes(self) -> None:
        """Comments can contain single dashes (but not --)."""
        pairs = token_pairs("<!-- a-b-c -->")
        text = [v for t, v in pairs if t == "COMMENT_TEXT"]
        assert text == [" a-b-c "]

    def test_comment_between_elements(self) -> None:
        """Comment between two elements."""
        types = token_types("<a/><!-- mid --><b/>")
        assert "COMMENT_START" in types
        assert "COMMENT_END" in types


# ===========================================================================
# CDATA Sections
# ===========================================================================


class TestCDATA:
    """Test tokenization of CDATA sections."""

    def test_simple_cdata(self) -> None:
        """A simple CDATA section."""
        pairs = token_pairs("<![CDATA[raw text]]>")
        assert pairs == [
            ("CDATA_START", "<![CDATA["),
            ("CDATA_TEXT", "raw text"),
            ("CDATA_END", "]]>"),
        ]

    def test_cdata_with_angle_brackets(self) -> None:
        """CDATA can contain < and > which are normally special."""
        pairs = token_pairs("<![CDATA[<not a tag>]]>")
        text = [v for t, v in pairs if t == "CDATA_TEXT"]
        assert text == ["<not a tag>"]

    def test_cdata_preserves_whitespace(self) -> None:
        """Whitespace in CDATA is preserved."""
        pairs = token_pairs("<![CDATA[  hello\n  world  ]]>")
        text = [v for t, v in pairs if t == "CDATA_TEXT"]
        assert text == ["  hello\n  world  "]

    def test_cdata_with_single_bracket(self) -> None:
        """CDATA can contain ] without ending (needs ]]>)."""
        pairs = token_pairs("<![CDATA[a]b]]>")
        text = [v for t, v in pairs if t == "CDATA_TEXT"]
        assert text == ["a]b"]


# ===========================================================================
# Processing Instructions
# ===========================================================================


class TestProcessingInstructions:
    """Test tokenization of processing instructions."""

    def test_xml_declaration(self) -> None:
        """The XML declaration: <?xml version="1.0"?>."""
        pairs = token_pairs('<?xml version="1.0"?>')
        assert pairs == [
            ("PI_START", "<?"),
            ("PI_TARGET", "xml"),
            ("PI_TEXT", ' version="1.0"'),
            ("PI_END", "?>"),
        ]

    def test_stylesheet_pi(self) -> None:
        """A stylesheet processing instruction."""
        types = token_types('<?xml-stylesheet type="text/xsl"?>')
        assert types[0] == "PI_START"
        assert types[1] == "PI_TARGET"
        assert types[-1] == "PI_END"


# ===========================================================================
# Entity and Character References
# ===========================================================================


class TestReferences:
    """Test tokenization of entity and character references."""

    def test_named_entity(self) -> None:
        """Named entity reference: &amp;."""
        pairs = token_pairs("a&amp;b")
        assert pairs == [
            ("TEXT", "a"),
            ("ENTITY_REF", "&amp;"),
            ("TEXT", "b"),
        ]

    def test_decimal_char_ref(self) -> None:
        """Decimal character reference: &#65;."""
        pairs = token_pairs("&#65;")
        assert pairs == [("CHAR_REF", "&#65;")]

    def test_hex_char_ref(self) -> None:
        """Hexadecimal character reference: &#x41;."""
        pairs = token_pairs("&#x41;")
        assert pairs == [("CHAR_REF", "&#x41;")]

    def test_multiple_entities(self) -> None:
        """Multiple entity references in text."""
        types = token_types("&lt;hello&gt;")
        assert types == ["ENTITY_REF", "TEXT", "ENTITY_REF"]


# ===========================================================================
# Nested and Mixed Content
# ===========================================================================


class TestNestedContent:
    """Test tokenization of nested and mixed XML content."""

    def test_nested_elements(self) -> None:
        """Nested elements: <a><b>text</b></a>."""
        types = token_types("<a><b>text</b></a>")
        # Should have two OPEN_TAG_START and two CLOSE_TAG_START
        assert types.count("OPEN_TAG_START") == 2
        assert types.count("CLOSE_TAG_START") == 2

    def test_mixed_content(self) -> None:
        """Text mixed with child elements."""
        pairs = token_pairs("<p>Hello <b>world</b>!</p>")
        texts = [v for t, v in pairs if t == "TEXT"]
        assert texts == ["Hello ", "world", "!"]

    def test_full_document(self) -> None:
        """A small but complete XML document."""
        source = (
            '<?xml version="1.0"?>'
            "<!-- A greeting -->"
            '<root lang="en">'
            "<greeting>Hello &amp; welcome</greeting>"
            "</root>"
        )
        tokens = tokenize_xml(source)
        types = [
            t.type if isinstance(t.type, str) else t.type.name
            for t in tokens
        ]

        # PI present
        assert "PI_START" in types
        assert "PI_END" in types

        # Comment present
        assert "COMMENT_START" in types
        assert "COMMENT_END" in types

        # Tags present
        assert types.count("OPEN_TAG_START") == 2  # root + greeting
        assert types.count("CLOSE_TAG_START") == 2

        # Entity ref present
        assert "ENTITY_REF" in types

        # Ends with EOF
        assert types[-1] == "EOF"

    def test_cdata_inside_element(self) -> None:
        """CDATA section inside an element."""
        source = "<script><![CDATA[x < y]]></script>"
        types = token_types(source)
        assert "CDATA_START" in types
        assert "CDATA_TEXT" in types
        assert "CDATA_END" in types


# ===========================================================================
# Edge Cases
# ===========================================================================


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_empty_string(self) -> None:
        """Empty input produces only EOF."""
        tokens = tokenize_xml("")
        assert len(tokens) == 1
        name = (
            tokens[0].type
            if isinstance(tokens[0].type, str)
            else tokens[0].type.name
        )
        assert name == "EOF"

    def test_text_only(self) -> None:
        """Plain text with no tags."""
        pairs = token_pairs("just text")
        assert pairs == [("TEXT", "just text")]

    def test_whitespace_between_tags_skipped(self) -> None:
        """Whitespace between tags is consumed by skip patterns.

        The XML grammar has a skip pattern for whitespace, so spaces
        between tags are silently consumed — no TEXT tokens are emitted
        for inter-tag whitespace.
        """
        pairs = token_pairs("<a> <b> </b> </a>")
        texts = [v for t, v in pairs if t == "TEXT"]
        assert texts == []  # whitespace consumed by skip pattern

    def test_eof_always_last(self) -> None:
        """The last token is always EOF."""
        tokens = tokenize_xml("<root/>")
        name = (
            tokens[-1].type
            if isinstance(tokens[-1].type, str)
            else tokens[-1].type.name
        )
        assert name == "EOF"
