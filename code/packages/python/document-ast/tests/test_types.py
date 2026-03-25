"""Tests for Document AST type definitions.

These tests verify that the TypedDict structures are correctly defined
and can be constructed with valid data.
"""

from coding_adventures_document_ast import (
    AutolinkNode,
    BlockquoteNode,
    CodeBlockNode,
    CodeSpanNode,
    DocumentNode,
    EmphasisNode,
    HardBreakNode,
    HeadingNode,
    ImageNode,
    LinkNode,
    ListItemNode,
    ListNode,
    ParagraphNode,
    RawBlockNode,
    RawInlineNode,
    SoftBreakNode,
    StrongNode,
    TextNode,
    ThematicBreakNode,
)


class TestDocumentNode:
    def test_empty_document(self) -> None:
        doc: DocumentNode = {"type": "document", "children": []}
        assert doc["type"] == "document"
        assert doc["children"] == []

    def test_document_with_children(self) -> None:
        para: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "text", "value": "Hello"}],
        }
        doc: DocumentNode = {"type": "document", "children": [para]}
        assert len(doc["children"]) == 1
        assert doc["children"][0]["type"] == "paragraph"


class TestHeadingNode:
    def test_heading_level_1(self) -> None:
        node: HeadingNode = {
            "type": "heading",
            "level": 1,
            "children": [{"type": "text", "value": "Title"}],
        }
        assert node["type"] == "heading"
        assert node["level"] == 1

    def test_heading_levels_1_through_6(self) -> None:
        for level in [1, 2, 3, 4, 5, 6]:
            node: HeadingNode = {"type": "heading", "level": level, "children": []}  # type: ignore[typeddict-item]
            assert node["level"] == level


class TestParagraphNode:
    def test_paragraph_with_text(self) -> None:
        node: ParagraphNode = {
            "type": "paragraph",
            "children": [{"type": "text", "value": "Hello world"}],
        }
        assert node["type"] == "paragraph"
        assert len(node["children"]) == 1


class TestCodeBlockNode:
    def test_code_block_with_language(self) -> None:
        node: CodeBlockNode = {
            "type": "code_block",
            "language": "python",
            "value": "print('hello')\n",
        }
        assert node["type"] == "code_block"
        assert node["language"] == "python"

    def test_code_block_without_language(self) -> None:
        node: CodeBlockNode = {
            "type": "code_block",
            "language": None,
            "value": "some code\n",
        }
        assert node["language"] is None


class TestBlockquoteNode:
    def test_blockquote(self) -> None:
        node: BlockquoteNode = {"type": "blockquote", "children": []}
        assert node["type"] == "blockquote"
        assert node["children"] == []


class TestListNode:
    def test_unordered_list(self) -> None:
        node: ListNode = {
            "type": "list",
            "ordered": False,
            "start": None,
            "tight": True,
            "children": [],
        }
        assert node["type"] == "list"
        assert node["ordered"] is False
        assert node["start"] is None
        assert node["tight"] is True

    def test_ordered_list_with_start(self) -> None:
        node: ListNode = {
            "type": "list",
            "ordered": True,
            "start": 5,
            "tight": False,
            "children": [],
        }
        assert node["ordered"] is True
        assert node["start"] == 5


class TestListItemNode:
    def test_list_item(self) -> None:
        node: ListItemNode = {"type": "list_item", "children": []}
        assert node["type"] == "list_item"


class TestThematicBreakNode:
    def test_thematic_break(self) -> None:
        node: ThematicBreakNode = {"type": "thematic_break"}
        assert node["type"] == "thematic_break"


class TestRawBlockNode:
    def test_raw_html_block(self) -> None:
        node: RawBlockNode = {
            "type": "raw_block",
            "format": "html",
            "value": "<div>raw</div>\n",
        }
        assert node["type"] == "raw_block"
        assert node["format"] == "html"

    def test_raw_latex_block(self) -> None:
        node: RawBlockNode = {
            "type": "raw_block",
            "format": "latex",
            "value": "\\textbf{x}\n",
        }
        assert node["format"] == "latex"


class TestTextNode:
    def test_text_node(self) -> None:
        node: TextNode = {"type": "text", "value": "Hello world"}
        assert node["type"] == "text"
        assert node["value"] == "Hello world"


class TestEmphasisNode:
    def test_emphasis(self) -> None:
        node: EmphasisNode = {
            "type": "emphasis",
            "children": [{"type": "text", "value": "hello"}],
        }
        assert node["type"] == "emphasis"


class TestStrongNode:
    def test_strong(self) -> None:
        node: StrongNode = {
            "type": "strong",
            "children": [{"type": "text", "value": "bold"}],
        }
        assert node["type"] == "strong"


class TestCodeSpanNode:
    def test_code_span(self) -> None:
        node: CodeSpanNode = {"type": "code_span", "value": "const x = 1"}
        assert node["type"] == "code_span"
        assert node["value"] == "const x = 1"


class TestLinkNode:
    def test_link_with_title(self) -> None:
        node: LinkNode = {
            "type": "link",
            "destination": "https://example.com",
            "title": "Example",
            "children": [{"type": "text", "value": "click here"}],
        }
        assert node["type"] == "link"
        assert node["destination"] == "https://example.com"
        assert node["title"] == "Example"

    def test_link_without_title(self) -> None:
        node: LinkNode = {
            "type": "link",
            "destination": "https://example.com",
            "title": None,
            "children": [],
        }
        assert node["title"] is None


class TestImageNode:
    def test_image(self) -> None:
        node: ImageNode = {
            "type": "image",
            "destination": "cat.png",
            "title": None,
            "alt": "a cat",
        }
        assert node["type"] == "image"
        assert node["alt"] == "a cat"


class TestAutolinkNode:
    def test_url_autolink(self) -> None:
        node: AutolinkNode = {
            "type": "autolink",
            "destination": "https://example.com",
            "is_email": False,
        }
        assert node["type"] == "autolink"
        assert node["is_email"] is False

    def test_email_autolink(self) -> None:
        node: AutolinkNode = {
            "type": "autolink",
            "destination": "user@example.com",
            "is_email": True,
        }
        assert node["is_email"] is True


class TestRawInlineNode:
    def test_html_inline(self) -> None:
        node: RawInlineNode = {
            "type": "raw_inline",
            "format": "html",
            "value": "<em>raw</em>",
        }
        assert node["type"] == "raw_inline"
        assert node["format"] == "html"


class TestHardBreakNode:
    def test_hard_break(self) -> None:
        node: HardBreakNode = {"type": "hard_break"}
        assert node["type"] == "hard_break"


class TestSoftBreakNode:
    def test_soft_break(self) -> None:
        node: SoftBreakNode = {"type": "soft_break"}
        assert node["type"] == "soft_break"


class TestNesting:
    def test_nested_structure(self) -> None:
        """Test that complex nested structures can be created and traversed."""
        doc: DocumentNode = {
            "type": "document",
            "children": [
                {
                    "type": "heading",
                    "level": 1,
                    "children": [{"type": "text", "value": "Title"}],
                },
                {
                    "type": "paragraph",
                    "children": [
                        {"type": "text", "value": "Hello "},
                        {
                            "type": "emphasis",
                            "children": [{"type": "text", "value": "world"}],
                        },
                    ],
                },
                {
                    "type": "list",
                    "ordered": False,
                    "start": None,
                    "tight": True,
                    "children": [
                        {
                            "type": "list_item",
                            "children": [
                                {
                                    "type": "paragraph",
                                    "children": [{"type": "text", "value": "item 1"}],
                                }
                            ],
                        }
                    ],
                },
            ],
        }
        assert doc["type"] == "document"
        assert len(doc["children"]) == 3
        assert doc["children"][0]["type"] == "heading"
        assert doc["children"][1]["type"] == "paragraph"
        assert doc["children"][2]["type"] == "list"
