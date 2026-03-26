"""Comprehensive test suite for the Document AST Sanitizer.

This test module covers:
  1. All policy options from the truth table in TE02
  2. Every XSS attack vector listed in the spec
  3. Edge cases (empty documents, nested structures, etc.)
  4. Immutability guarantees (input never mutated)
  5. PASSTHROUGH identity property

Test organisation mirrors the structure of sanitizer.py:
  - URL utilities
  - Heading level clamping
  - Raw block/inline handling
  - Link handling (drop + URL sanitization)
  - Image handling (drop, transform-to-text, URL sanitization)
  - Autolink handling
  - Code block/span handling
  - Blockquote handling
  - Empty children cleanup
  - Named presets (STRICT, RELAXED, PASSTHROUGH)
  - XSS attack vectors
"""

from __future__ import annotations

import copy
import dataclasses

import pytest
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
    StrikethroughNode,
    StrongNode,
    TableCellNode,
    TableNode,
    TableRowNode,
    TaskItemNode,
    TextNode,
    ThematicBreakNode,
)

from coding_adventures_document_ast_sanitizer import (
    PASSTHROUGH,
    RELAXED,
    STRICT,
    SanitizationPolicy,
    extract_scheme,
    is_scheme_allowed,
    sanitize,
    strip_control_chars,
)

# ─── Helpers: document construction ──────────────────────────────────────────

def doc(*children) -> DocumentNode:
    """Build a DocumentNode with the given block children."""
    return DocumentNode(type="document", children=list(children))


def para(*children) -> ParagraphNode:
    """Build a ParagraphNode with the given inline children."""
    return ParagraphNode(type="paragraph", children=list(children))


def heading(level: int, *children) -> HeadingNode:
    """Build a HeadingNode."""
    return HeadingNode(type="heading", level=level, children=list(children))


def text(value: str) -> TextNode:
    """Build a TextNode."""
    return TextNode(type="text", value=value)


def link(dest: str, *children, title=None) -> LinkNode:
    """Build a LinkNode."""
    return LinkNode(type="link", destination=dest, title=title, children=list(children))


def image(dest: str, alt: str = "", title=None) -> ImageNode:
    """Build an ImageNode."""
    return ImageNode(type="image", destination=dest, alt=alt, title=title)


def autolink(dest: str, is_email: bool = False) -> AutolinkNode:
    """Build an AutolinkNode."""
    return AutolinkNode(type="autolink", destination=dest, is_email=is_email)


def raw_block(fmt: str, value: str = "") -> RawBlockNode:
    """Build a RawBlockNode."""
    return RawBlockNode(type="raw_block", format=fmt, value=value)


def raw_inline(fmt: str, value: str = "") -> RawInlineNode:
    """Build a RawInlineNode."""
    return RawInlineNode(type="raw_inline", format=fmt, value=value)


def code_block(lang=None, value="print()") -> CodeBlockNode:
    """Build a CodeBlockNode."""
    return CodeBlockNode(type="code_block", language=lang, value=value)


def code_span(value: str) -> CodeSpanNode:
    """Build a CodeSpanNode."""
    return CodeSpanNode(type="code_span", value=value)


def emphasis(*children) -> EmphasisNode:
    """Build an EmphasisNode."""
    return EmphasisNode(type="emphasis", children=list(children))


def strong(*children) -> StrongNode:
    """Build a StrongNode."""
    return StrongNode(type="strong", children=list(children))


def strikethrough(*children) -> StrikethroughNode:
    """Build a StrikethroughNode."""
    return StrikethroughNode(type="strikethrough", children=list(children))


def blockquote(*children) -> BlockquoteNode:
    """Build a BlockquoteNode."""
    return BlockquoteNode(type="blockquote", children=list(children))


def list_node(ordered=False, start=None, tight=True, *items) -> ListNode:
    """Build a ListNode."""
    return ListNode(type="list", ordered=ordered, start=start, tight=tight, children=list(items))


def list_item(*children) -> ListItemNode:
    """Build a ListItemNode."""
    return ListItemNode(type="list_item", children=list(children))


def task_item(checked: bool, *children) -> TaskItemNode:
    return TaskItemNode(type="task_item", checked=checked, children=list(children))


def table_cell(*children) -> TableCellNode:
    return TableCellNode(type="table_cell", children=list(children))


def table_row(is_header: bool, *cells) -> TableRowNode:
    return TableRowNode(type="table_row", isHeader=is_header, children=list(cells))


def table(*rows, align=None) -> TableNode:
    return TableNode(type="table", align=list(align or []), children=list(rows))


def thematic_break() -> ThematicBreakNode:
    """Build a ThematicBreakNode."""
    return ThematicBreakNode(type="thematic_break")


def hard_break() -> HardBreakNode:
    return HardBreakNode(type="hard_break")


def soft_break() -> SoftBreakNode:
    return SoftBreakNode(type="soft_break")


# ─── URL utilities ─────────────────────────────────────────────────────────────


class TestStripControlChars:
    def test_noop_on_clean_url(self):
        assert strip_control_chars("https://example.com") == "https://example.com"

    def test_strips_null_byte(self):
        assert strip_control_chars("java\x00script:alert(1)") == "javascript:alert(1)"

    def test_strips_cr(self):
        assert strip_control_chars("java\rscript:alert(1)") == "javascript:alert(1)"

    def test_strips_lf(self):
        assert strip_control_chars("java\nscript:alert(1)") == "javascript:alert(1)"

    def test_strips_tab(self):
        assert strip_control_chars("java\tscript:alert(1)") == "javascript:alert(1)"

    def test_strips_zero_width_space(self):
        assert strip_control_chars("\u200bjavascript:alert(1)") == "javascript:alert(1)"

    def test_strips_zero_width_non_joiner(self):
        assert strip_control_chars("\u200cjavascript:alert(1)") == "javascript:alert(1)"

    def test_strips_zero_width_joiner(self):
        assert strip_control_chars("\u200djavascript:alert(1)") == "javascript:alert(1)"

    def test_strips_word_joiner(self):
        assert strip_control_chars("\u2060javascript:alert(1)") == "javascript:alert(1)"

    def test_strips_bom(self):
        assert strip_control_chars("\ufeffjavascript:alert(1)") == "javascript:alert(1)"

    def test_strips_multiple_invisible(self):
        assert strip_control_chars("j\x00a\u200bv\x01a\ufeffscript:") == "javascript:"

    def test_empty_string(self):
        assert strip_control_chars("") == ""

    def test_relative_url_unchanged(self):
        assert strip_control_chars("relative/path") == "relative/path"


class TestExtractScheme:
    def test_https(self):
        assert extract_scheme("https://example.com") == "https"

    def test_http(self):
        assert extract_scheme("http://example.com") == "http"

    def test_mailto(self):
        assert extract_scheme("mailto:user@example.com") == "mailto"

    def test_javascript(self):
        assert extract_scheme("javascript:alert(1)") == "javascript"

    def test_uppercase_normalized_to_lower(self):
        assert extract_scheme("JAVASCRIPT:alert(1)") == "javascript"

    def test_mixed_case(self):
        assert extract_scheme("JavaSCRIPT:x") == "javascript"

    def test_data_scheme(self):
        assert extract_scheme("data:text/html,<h1>x</h1>") == "data"

    def test_no_scheme_relative(self):
        assert extract_scheme("relative/path") is None

    def test_no_scheme_absolute(self):
        assert extract_scheme("/absolute/path") is None

    def test_no_scheme_double_dot(self):
        assert extract_scheme("../relative") is None

    def test_colon_in_query_string(self):
        # The colon is after a ?, so it's not a scheme.
        assert extract_scheme("path?query=value:here") is None

    def test_colon_in_path(self):
        # The colon is after a /, so it's not a scheme.
        assert extract_scheme("/path:with:colons") is None

    def test_ftp(self):
        assert extract_scheme("ftp://files.example.com") == "ftp"

    def test_blob(self):
        assert extract_scheme("blob:https://origin/uuid") == "blob"

    def test_vbscript(self):
        assert extract_scheme("vbscript:MsgBox(1)") == "vbscript"

    def test_empty_string(self):
        assert extract_scheme("") is None


class TestIsSchemeAllowed:
    def test_https_allowed(self):
        assert is_scheme_allowed("https://example.com", ("http", "https", "mailto"))

    def test_javascript_blocked(self):
        assert not is_scheme_allowed("javascript:alert(1)", ("http", "https", "mailto"))

    def test_relative_always_allowed(self):
        assert is_scheme_allowed("relative/path", ("http", "https"))

    def test_absolute_path_always_allowed(self):
        assert is_scheme_allowed("/absolute/path", ("http", "https"))

    def test_data_blocked(self):
        assert not is_scheme_allowed("data:text/html,<h1>x</h1>", ("http", "https"))

    def test_uppercase_normalized(self):
        assert is_scheme_allowed("HTTPS://example.com", ("http", "https"))

    def test_null_allows_any(self):
        assert is_scheme_allowed("javascript:alert(1)", None)
        assert is_scheme_allowed("data:text/html,x", None)
        assert is_scheme_allowed("vbscript:MsgBox(1)", None)

    def test_control_char_bypass_blocked(self):
        # java\x00script: → javascript: after stripping → blocked
        assert not is_scheme_allowed("java\x00script:alert(1)", ("http", "https"))

    def test_zero_width_bypass_blocked(self):
        assert not is_scheme_allowed("\u200bjavascript:alert(1)", ("http", "https"))

    def test_blob_blocked(self):
        assert not is_scheme_allowed("blob:https://origin/uuid", ("http", "https"))

    def test_mailto_allowed(self):
        assert is_scheme_allowed("mailto:user@example.com", ("http", "https", "mailto"))

    def test_ftp_allowed(self):
        assert is_scheme_allowed("ftp://files.example.com", ("http", "https", "mailto", "ftp"))

    def test_vbscript_blocked(self):
        assert not is_scheme_allowed("vbscript:MsgBox(1)", ("http", "https", "mailto"))


# ─── DocumentNode ──────────────────────────────────────────────────────────────


class TestDocumentNode:
    def test_empty_document_returned_as_empty(self):
        d = doc()
        result = sanitize(d, PASSTHROUGH)
        assert result["type"] == "document"
        assert result["children"] == []

    def test_document_with_children(self):
        d = doc(para(text("hello")))
        result = sanitize(d, PASSTHROUGH)
        assert len(result["children"]) == 1


# ─── Heading level clamping ────────────────────────────────────────────────────


class TestHeadingClamping:
    def test_passthrough_no_clamping(self):
        d = doc(heading(1, text("Title")))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["level"] == 1

    def test_max_heading_drop_removes_all_headings(self):
        d = doc(heading(1, text("A")), heading(3, text("B")), para(text("p")))
        policy = SanitizationPolicy(max_heading_level="drop")
        result = sanitize(d, policy)
        # headings dropped, paragraph kept
        assert len(result["children"]) == 1
        assert result["children"][0]["type"] == "paragraph"

    def test_min_heading_level_promotes_h1_to_h2(self):
        d = doc(heading(1, text("Title")))
        policy = SanitizationPolicy(min_heading_level=2)
        result = sanitize(d, policy)
        assert result["children"][0]["level"] == 2

    def test_min_heading_level_does_not_affect_deeper(self):
        d = doc(heading(3, text("Sub")))
        policy = SanitizationPolicy(min_heading_level=2)
        result = sanitize(d, policy)
        assert result["children"][0]["level"] == 3

    def test_max_heading_level_clamps_h5_to_h3(self):
        d = doc(heading(5, text("Deep")))
        policy = SanitizationPolicy(max_heading_level=3)
        result = sanitize(d, policy)
        assert result["children"][0]["level"] == 3

    def test_max_heading_level_does_not_affect_shallower(self):
        d = doc(heading(2, text("Sub")))
        policy = SanitizationPolicy(max_heading_level=3)
        result = sanitize(d, policy)
        assert result["children"][0]["level"] == 2

    def test_both_min_and_max_clamping(self):
        d = doc(
            heading(1, text("H1")),  # below min → promoted to 2
            heading(5, text("H5")),  # above max → clamped to 4
            heading(3, text("H3")),  # within range → unchanged
        )
        policy = SanitizationPolicy(min_heading_level=2, max_heading_level=4)
        result = sanitize(d, policy)
        assert result["children"][0]["level"] == 2
        assert result["children"][1]["level"] == 4
        assert result["children"][2]["level"] == 3

    def test_strict_preserves_h2(self):
        d = doc(heading(2, text("Section")))
        result = sanitize(d, STRICT)
        assert result["children"][0]["level"] == 2

    def test_strict_promotes_h1_to_h2(self):
        d = doc(heading(1, text("Title")))
        result = sanitize(d, STRICT)
        assert result["children"][0]["level"] == 2

    def test_heading_with_empty_children_dropped(self):
        # heading whose only child is a raw inline that gets dropped
        d = doc(heading(2, raw_inline("latex", "\\LaTeX{}")))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []


# ─── Raw block handling ────────────────────────────────────────────────────────


class TestRawBlockNode:
    def test_drop_all_drops_html(self):
        d = doc(raw_block("html", "<div>hi</div>"))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_drop_all_drops_latex(self):
        d = doc(raw_block("latex", "\\LaTeX"))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_passthrough_keeps_all(self):
        d = doc(raw_block("html", "<div>"), raw_block("latex", "\\x"))
        policy = SanitizationPolicy(allow_raw_block_formats="passthrough")
        result = sanitize(d, policy)
        assert len(result["children"]) == 2

    def test_allowlist_keeps_html_drops_latex(self):
        d = doc(raw_block("html", "<div>"), raw_block("latex", "\\x"))
        policy = SanitizationPolicy(allow_raw_block_formats=("html",))
        result = sanitize(d, policy)
        assert len(result["children"]) == 1
        assert result["children"][0]["format"] == "html"

    def test_allowlist_drops_unknown_format(self):
        d = doc(raw_block("rtf", "{\\rtf1}"))
        policy = SanitizationPolicy(allow_raw_block_formats=("html",))
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_strict_drops_raw_blocks(self):
        d = doc(raw_block("html", "<script>alert(1)</script>"))
        result = sanitize(d, STRICT)
        assert result["children"] == []

    def test_relaxed_keeps_html_raw_block(self):
        d = doc(raw_block("html", "<b>bold</b>"))
        result = sanitize(d, RELAXED)
        assert len(result["children"]) == 1

    def test_relaxed_drops_latex_raw_block(self):
        d = doc(raw_block("latex", "\\LaTeX"))
        result = sanitize(d, RELAXED)
        assert result["children"] == []


# ─── Raw inline handling ────────────────────────────────────────────────────────


class TestRawInlineNode:
    def test_drop_all_drops_html_inline(self):
        d = doc(para(text("a"), raw_inline("html", "<em>b</em>")))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert len(result["children"][0]["children"]) == 1
        assert result["children"][0]["children"][0]["value"] == "a"

    def test_passthrough_keeps_inline(self):
        d = doc(para(raw_inline("html", "<em>x</em>")))
        policy = SanitizationPolicy(allow_raw_inline_formats="passthrough")
        result = sanitize(d, policy)
        assert len(result["children"][0]["children"]) == 1

    def test_allowlist_keeps_html_drops_latex(self):
        d = doc(para(raw_inline("html", "<em>"), raw_inline("latex", "\\cmd")))
        policy = SanitizationPolicy(allow_raw_inline_formats=("html",))
        result = sanitize(d, policy)
        assert len(result["children"][0]["children"]) == 1
        assert result["children"][0]["children"][0]["format"] == "html"

    def test_para_dropped_when_only_raw_inline_dropped(self):
        # A paragraph containing only a dropped raw_inline → paragraph itself dropped.
        d = doc(para(raw_inline("html", "<script>alert(1)</script>")))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []


# ─── Link handling ─────────────────────────────────────────────────────────────


class TestLinkNode:
    def test_passthrough_keeps_link(self):
        d = doc(para(link("https://example.com", text("click"))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["type"] == "link"
        assert result["children"][0]["children"][0]["destination"] == "https://example.com"

    def test_drop_links_promotes_children(self):
        d = doc(para(link("https://example.com", text("click here"))))
        policy = SanitizationPolicy(drop_links=True)
        result = sanitize(d, policy)
        para_children = result["children"][0]["children"]
        # link gone, TextNode promoted
        assert len(para_children) == 1
        assert para_children[0]["type"] == "text"
        assert para_children[0]["value"] == "click here"

    def test_drop_links_multiple_children_all_promoted(self):
        d = doc(para(link("https://x.com", text("a"), text("b"))))
        policy = SanitizationPolicy(drop_links=True)
        result = sanitize(d, policy)
        para_children = result["children"][0]["children"]
        assert len(para_children) == 2
        assert para_children[0]["value"] == "a"
        assert para_children[1]["value"] == "b"

    def test_javascript_url_blocked_destination_emptied(self):
        d = doc(para(link("javascript:alert(1)", text("click"))))
        result = sanitize(d, STRICT)
        link_node = result["children"][0]["children"][0]
        assert link_node["type"] == "link"
        assert link_node["destination"] == ""

    def test_javascript_uppercase_blocked(self):
        d = doc(para(link("JAVASCRIPT:alert(1)", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_null_byte_bypass_blocked(self):
        d = doc(para(link("java\x00script:alert(1)", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_zero_width_bypass_blocked(self):
        d = doc(para(link("\u200bjavascript:alert(1)", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_data_url_blocked(self):
        d = doc(para(link("data:text/html,<script>alert(1)</script>", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_blob_url_blocked(self):
        d = doc(para(link("blob:https://origin/uuid", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_vbscript_blocked(self):
        d = doc(para(link("vbscript:MsgBox(1)", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_https_allowed(self):
        d = doc(para(link("https://example.com", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == "https://example.com"

    def test_relative_url_always_allowed(self):
        d = doc(para(link("relative/page.html", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == "relative/page.html"

    def test_absolute_path_always_allowed(self):
        d = doc(para(link("/absolute/page.html", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == "/absolute/page.html"

    def test_link_title_preserved(self):
        d = doc(para(link("https://example.com", text("click"), title="Hover text")))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["title"] == "Hover text"

    def test_null_allowed_schemes_passes_javascript(self):
        d = doc(para(link("javascript:alert(1)", text("click"))))
        result = sanitize(d, PASSTHROUGH)
        # PASSTHROUGH has allowed_url_schemes=None → any scheme passes
        assert result["children"][0]["children"][0]["destination"] == "javascript:alert(1)"


# ─── Image handling ─────────────────────────────────────────────────────────────


class TestImageNode:
    def test_passthrough_keeps_image(self):
        d = doc(para(image("cat.png", alt="a cat")))
        result = sanitize(d, PASSTHROUGH)
        img = result["children"][0]["children"][0]
        assert img["type"] == "image"
        assert img["destination"] == "cat.png"

    def test_drop_images_removes_entirely(self):
        d = doc(para(image("cat.png", alt="a cat")))
        policy = SanitizationPolicy(drop_images=True)
        result = sanitize(d, policy)
        # image gone, paragraph empty → paragraph dropped
        assert result["children"] == []

    def test_transform_image_to_text(self):
        d = doc(para(image("cat.png", alt="a cat")))
        policy = SanitizationPolicy(transform_image_to_text=True)
        result = sanitize(d, policy)
        node = result["children"][0]["children"][0]
        assert node["type"] == "text"
        assert node["value"] == "a cat"

    def test_drop_images_takes_precedence_over_transform(self):
        # When both are True, drop wins.
        d = doc(para(image("cat.png", alt="a cat")))
        policy = SanitizationPolicy(drop_images=True, transform_image_to_text=True)
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_image_javascript_url_blocked(self):
        # Use a policy that keeps images but restricts URL schemes
        # (STRICT converts images to text, so we need a custom policy here)
        d = doc(para(image("javascript:alert(1)", alt="x")))
        policy = SanitizationPolicy(
            drop_images=False,
            transform_image_to_text=False,
            allowed_url_schemes=("http", "https", "mailto"),
        )
        result = sanitize(d, policy)
        img = result["children"][0]["children"][0]
        assert img["type"] == "image"
        assert img["destination"] == ""

    def test_image_https_allowed(self):
        # Use a policy that keeps images and allows https
        d = doc(para(image("https://cdn.example.com/cat.png", alt="cat")))
        policy = SanitizationPolicy(
            drop_images=False,
            transform_image_to_text=False,
            allowed_url_schemes=("http", "https", "mailto"),
        )
        result = sanitize(d, policy)
        img = result["children"][0]["children"][0]
        assert img["destination"] == "https://cdn.example.com/cat.png"

    def test_strict_converts_image_to_text(self):
        # STRICT has transform_image_to_text=True
        d = doc(para(image("https://tracker.example.com/pixel.gif", alt="tracking pixel")))
        result = sanitize(d, STRICT)
        node = result["children"][0]["children"][0]
        assert node["type"] == "text"
        assert node["value"] == "tracking pixel"

    def test_image_empty_alt_becomes_empty_text(self):
        d = doc(para(image("cat.png", alt="")))
        policy = SanitizationPolicy(transform_image_to_text=True)
        result = sanitize(d, policy)
        # empty text node is kept (not dropped — TextNode is always kept)
        assert result["children"][0]["children"][0]["type"] == "text"
        assert result["children"][0]["children"][0]["value"] == ""


# ─── Autolink handling ─────────────────────────────────────────────────────────


class TestAutolinkNode:
    def test_https_autolink_kept(self):
        d = doc(para(autolink("https://example.com")))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["type"] == "autolink"

    def test_email_autolink_kept(self):
        d = doc(para(autolink("user@example.com", is_email=True)))
        result = sanitize(d, STRICT)
        node = result["children"][0]["children"][0]
        assert node["type"] == "autolink"

    def test_javascript_autolink_dropped(self):
        d = doc(para(autolink("javascript:alert(1)")))
        result = sanitize(d, STRICT)
        # autolink dropped, paragraph empty → paragraph dropped
        assert result["children"] == []

    def test_data_autolink_dropped(self):
        d = doc(para(autolink("data:text/html,<h1>x</h1>")))
        result = sanitize(d, STRICT)
        assert result["children"] == []

    def test_passthrough_keeps_any_scheme(self):
        d = doc(para(autolink("javascript:alert(1)")))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["type"] == "autolink"


# ─── Code blocks and code spans ────────────────────────────────────────────────


class TestCodeHandling:
    def test_passthrough_keeps_code_block(self):
        d = doc(code_block("python", "x = 1\n"))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["type"] == "code_block"

    def test_drop_code_blocks(self):
        d = doc(code_block("python", "os.system('rm -rf /')"))
        policy = SanitizationPolicy(drop_code_blocks=True)
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_code_block_not_dropped_by_strict(self):
        # STRICT doesn't drop code blocks
        d = doc(code_block("python", "print('hello')"))
        result = sanitize(d, STRICT)
        assert len(result["children"]) == 1

    def test_code_span_passthrough(self):
        d = doc(para(code_span("x = 1")))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["type"] == "code_span"

    def test_transform_code_span_to_text(self):
        d = doc(para(code_span("x = 1")))
        policy = SanitizationPolicy(transform_code_span_to_text=True)
        result = sanitize(d, policy)
        node = result["children"][0]["children"][0]
        assert node["type"] == "text"
        assert node["value"] == "x = 1"

    def test_transform_code_span_not_set_in_strict(self):
        d = doc(para(code_span("x = 1")))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["type"] == "code_span"


# ─── Blockquote handling ───────────────────────────────────────────────────────


class TestBlockquoteNode:
    def test_passthrough_keeps_blockquote(self):
        d = doc(blockquote(para(text("quote"))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["type"] == "blockquote"

    def test_drop_blockquotes_removes_subtree(self):
        # Children are NOT promoted (unlike drop_links).
        d = doc(blockquote(para(text("quote"))))
        policy = SanitizationPolicy(drop_blockquotes=True)
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_blockquote_recurse_drops_invalid_children(self):
        # Blockquote contains a raw block → dropped by policy → blockquote empty → dropped.
        d = doc(blockquote(raw_block("html", "<div>hi</div>")))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_blockquote_keeps_valid_children(self):
        d = doc(blockquote(para(text("safe")), raw_block("html", "<b>")))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        bq = result["children"][0]
        assert len(bq["children"]) == 1
        assert bq["children"][0]["type"] == "paragraph"


# ─── List handling ─────────────────────────────────────────────────────────────


class TestListNode:
    def test_list_preserved(self):
        d = doc(list_node(False, None, True, list_item(para(text("item")))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["type"] == "list"

    def test_empty_list_after_sanitization_dropped(self):
        # All list items become empty → list dropped.
        d = doc(list_node(False, None, True, list_item(raw_block("html", "<div>"))))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_list_metadata_preserved(self):
        d = doc(list_node(True, 3, False, list_item(para(text("a")))))
        result = sanitize(d, PASSTHROUGH)
        lst = result["children"][0]
        assert lst["ordered"] is True
        assert lst["start"] == 3
        assert lst["tight"] is False


# ─── ThematicBreak handling ────────────────────────────────────────────────────


class TestThematicBreakNode:
    def test_always_kept(self):
        d = doc(thematic_break())
        result = sanitize(d, STRICT)
        assert result["children"][0]["type"] == "thematic_break"


# ─── HardBreak and SoftBreak ───────────────────────────────────────────────────


class TestBreakNodes:
    def test_hard_break_always_kept(self):
        d = doc(para(text("a"), hard_break(), text("b")))
        result = sanitize(d, STRICT)
        inlines = result["children"][0]["children"]
        assert any(n["type"] == "hard_break" for n in inlines)

    def test_soft_break_always_kept(self):
        d = doc(para(text("a"), soft_break(), text("b")))
        result = sanitize(d, STRICT)
        inlines = result["children"][0]["children"]
        assert any(n["type"] == "soft_break" for n in inlines)


# ─── Emphasis and Strong handling ──────────────────────────────────────────────


class TestEmphasisStrong:
    def test_emphasis_preserved(self):
        d = doc(para(emphasis(text("em"))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["type"] == "emphasis"

    def test_strong_preserved(self):
        d = doc(para(strong(text("strong"))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["type"] == "strong"

    def test_empty_emphasis_dropped(self):
        d = doc(para(emphasis(raw_inline("html", "<em>"))))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        # emphasis children dropped → emphasis dropped → paragraph empty → dropped
        assert result["children"] == []

    def test_empty_strong_dropped(self):
        d = doc(para(strong(raw_inline("html", "<b>"))))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []


# ─── Empty children cleanup ────────────────────────────────────────────────────


class TestEmptyChildrenCleanup:
    def test_paragraph_dropped_when_all_children_dropped(self):
        d = doc(para(raw_inline("html", "<em>")))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []

    def test_document_never_dropped(self):
        d = doc(raw_block("html", "<div>"))
        policy = SanitizationPolicy(allow_raw_block_formats="drop-all")
        result = sanitize(d, policy)
        assert result["type"] == "document"
        assert result["children"] == []

    def test_nested_empty_cleanup(self):
        # Blockquote → paragraph → raw_inline → dropped.
        # paragraph empty → dropped. Blockquote empty → dropped.
        d = doc(blockquote(para(raw_inline("html", "<em>"))))
        policy = SanitizationPolicy(allow_raw_inline_formats="drop-all")
        result = sanitize(d, policy)
        assert result["children"] == []


# ─── Immutability ──────────────────────────────────────────────────────────────


class TestImmutability:
    def test_input_not_mutated_by_strict(self):
        d = doc(heading(1, text("Title")), para(link("javascript:alert(1)", text("x"))))
        original_deep_copy = copy.deepcopy(d)
        sanitize(d, STRICT)
        assert d == original_deep_copy

    def test_input_not_mutated_by_passthrough(self):
        d = doc(para(text("hello")))
        original = copy.deepcopy(d)
        sanitize(d, PASSTHROUGH)
        assert d == original

    def test_multiple_sanitizations_independent(self):
        d = doc(para(link("javascript:alert(1)", text("click"))))
        result1 = sanitize(d, STRICT)
        result2 = sanitize(d, RELAXED)
        # Both results are independent new objects.
        assert result1 is not result2
        # Original unchanged.
        assert d["children"][0]["children"][0]["destination"] == "javascript:alert(1)"


# ─── Named presets ─────────────────────────────────────────────────────────────


class TestNamedPresets:
    def test_strict_drops_raw_blocks(self):
        d = doc(raw_block("html", "<div>"))
        assert sanitize(d, STRICT)["children"] == []

    def test_strict_drops_raw_inlines(self):
        d = doc(para(raw_inline("html", "<em>")))
        assert sanitize(d, STRICT)["children"] == []

    def test_strict_allows_https(self):
        d = doc(para(link("https://example.com", text("x"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == "https://example.com"

    def test_strict_blocks_ftp(self):
        # STRICT allows only http, https, mailto (not ftp)
        d = doc(para(link("ftp://files.example.com", text("x"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_relaxed_keeps_html_raw_block(self):
        d = doc(raw_block("html", "<b>bold</b>"))
        result = sanitize(d, RELAXED)
        assert result["children"][0]["type"] == "raw_block"

    def test_relaxed_allows_ftp(self):
        d = doc(para(link("ftp://files.example.com", text("x"))))
        result = sanitize(d, RELAXED)
        assert result["children"][0]["children"][0]["destination"] == "ftp://files.example.com"

    def test_relaxed_keeps_images(self):
        d = doc(para(image("cat.png", alt="cat")))
        result = sanitize(d, RELAXED)
        assert result["children"][0]["children"][0]["type"] == "image"

    def test_passthrough_identity_for_simple_doc(self):
        d = doc(
            heading(1, text("Title")),
            para(text("Hello "), link("https://example.com", text("world"))),
            raw_block("html", "<div>"),
        )
        result = sanitize(d, PASSTHROUGH)
        # Document structure preserved
        assert len(result["children"]) == 3
        assert result["children"][0]["level"] == 1
        assert result["children"][1]["children"][1]["destination"] == "https://example.com"
        assert result["children"][2]["type"] == "raw_block"

    def test_passthrough_keeps_javascript_url(self):
        d = doc(para(link("javascript:alert(1)", text("x"))))
        result = sanitize(d, PASSTHROUGH)
        assert result["children"][0]["children"][0]["destination"] == "javascript:alert(1)"


# ─── XSS attack vectors ─────────────────────────────────────────────────────────


class TestXSSVectors:
    """Every XSS vector listed in TE02 spec testing strategy section."""

    # Script injection via raw blocks
    def test_script_via_raw_block(self):
        d = doc(raw_block("html", "<script>alert(1)</script>"))
        assert sanitize(d, STRICT)["children"] == []

    # JavaScript URL in link destination
    def test_javascript_url_link(self):
        d = doc(para(link("javascript:alert(1)", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_javascript_url_uppercase(self):
        d = doc(para(link("JAVASCRIPT:alert(1)", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_javascript_null_byte_bypass(self):
        # java\x00script: → javascript: after stripping
        d = doc(para(link("java\x00script:alert(1)", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_data_url_html_injection(self):
        d = doc(para(link("data:text/html,<script>alert(1)</script>", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_blob_url(self):
        d = doc(para(link("blob:https://origin/some-uuid", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_vbscript_url(self):
        d = doc(para(link("vbscript:MsgBox(1)", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_zero_width_bypass_in_link(self):
        d = doc(para(link("\u200bjavascript:alert(1)", text("click me"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_javascript_url_in_image(self):
        d = doc(para(image("javascript:alert(1)", alt="img")))
        # STRICT converts images to text, so destination="" wouldn't be tested
        # but let's test with a policy that keeps images but sanitizes URLs
        policy = SanitizationPolicy(
            drop_images=False,
            transform_image_to_text=False,
            allowed_url_schemes=("http", "https"),
        )
        result = sanitize(d, policy)
        img = result["children"][0]["children"][0]
        assert img["destination"] == ""

    def test_carriage_return_bypass(self):
        # java\rscript: → javascript: after stripping
        d = doc(para(link("java\rscript:alert(1)", text("click"))))
        result = sanitize(d, STRICT)
        assert result["children"][0]["children"][0]["destination"] == ""

    def test_html_raw_block_with_script_dropped_by_strict(self):
        # Even if the raw block format is "html", STRICT drops all raw blocks.
        d = doc(raw_block("html", "<script>alert(document.cookie)</script>"))
        result = sanitize(d, STRICT)
        assert result["children"] == []

    def test_autolink_javascript_dropped(self):
        d = doc(para(autolink("javascript:alert(1)")))
        result = sanitize(d, STRICT)
        assert result["children"] == []

    def test_complex_doc_strip_xss_keep_safe(self):
        """A realistic user post with mixed safe and dangerous content."""
        d = doc(
            heading(1, text("My Post")),
            para(
                text("Check out "),
                link("https://example.com", text("this site")),
                text(" or "),
                link("javascript:alert(1)", text("this one")),
            ),
            para(
                image("https://cdn.example.com/img.png", alt="safe image"),
            ),
            raw_block("html", "<script>alert(1)</script>"),
            para(code_span("safe code")),
        )
        result = sanitize(d, STRICT)

        # h1 → h2 (min_heading_level=2)
        assert result["children"][0]["level"] == 2

        # First paragraph: safe link kept, javascript link has destination=""
        para1 = result["children"][1]
        links = [n for n in para1["children"] if n["type"] == "link"]
        assert links[0]["destination"] == "https://example.com"
        assert links[1]["destination"] == ""

        # Second paragraph: image converted to text (STRICT has transform_image_to_text=True)
        para2 = result["children"][2]
        assert para2["children"][0]["type"] == "text"
        assert para2["children"][0]["value"] == "safe image"

        # raw_block dropped (STRICT has allow_raw_block_formats="drop-all")
        # Code span kept
        assert result["children"][3]["children"][0]["type"] == "code_span"


# ─── SanitizationPolicy dataclass ──────────────────────────────────────────────


class TestSanitizationPolicyDefaults:
    def test_default_policy_is_passthrough_like(self):
        """A default SanitizationPolicy has all-permissive settings."""
        p = SanitizationPolicy()
        assert p.allow_raw_block_formats == "passthrough"
        assert p.allow_raw_inline_formats == "passthrough"
        assert p.allowed_url_schemes == ("http", "https", "mailto", "ftp")
        assert p.drop_links is False
        assert p.drop_images is False
        assert p.transform_image_to_text is False
        assert p.max_heading_level == 6
        assert p.min_heading_level == 1
        assert p.drop_blockquotes is False
        assert p.drop_code_blocks is False
        assert p.transform_code_span_to_text is False

    def test_frozen_policy_cannot_be_mutated(self):
        p = SanitizationPolicy()
        with pytest.raises(dataclasses.FrozenInstanceError):
            p.drop_links = True  # type: ignore[misc]

    def test_custom_policy_via_constructor(self):
        p = SanitizationPolicy(
            allow_raw_block_formats="drop-all",
            min_heading_level=2,
            max_heading_level=4,
        )
        assert p.allow_raw_block_formats == "drop-all"
        assert p.min_heading_level == 2
        assert p.max_heading_level == 4

    def test_policy_spread_pattern(self):
        """Show how policies can be composed via dataclass.__dict__ spread."""
        custom = dataclasses.replace(RELAXED, min_heading_level=2)
        assert custom.min_heading_level == 2
        assert custom.allow_raw_block_formats == RELAXED.allow_raw_block_formats


class TestGfmNodes:
    def test_strikethrough_recurses(self):
        result = sanitize(doc(para(strikethrough(text("gone")))), STRICT)
        node = result["children"][0]["children"][0]
        assert node["type"] == "strikethrough"
        assert node["children"][0]["value"] == "gone"

    def test_empty_strikethrough_is_dropped(self):
        policy = dataclasses.replace(STRICT, allow_raw_inline_formats="drop-all")
        result = sanitize(doc(para(strikethrough(raw_inline("html", "<b>x</b>")))), policy)
        assert result["children"] == []

    def test_task_item_is_preserved(self):
        result = sanitize(
            doc(list_node(False, None, True, task_item(True, para(text("done"))))),
            STRICT,
        )
        item = result["children"][0]["children"][0]
        assert item["type"] == "task_item"
        assert item["checked"] is True

    def test_table_is_preserved(self):
        result = sanitize(
            doc(table(table_row(True, table_cell(text("A"))), align=["left"])),
            STRICT,
        )
        tbl = result["children"][0]
        assert tbl["type"] == "table"
        assert tbl["children"][0]["children"][0]["children"][0]["value"] == "A"

    def test_empty_table_cell_drops_row_and_table(self):
        policy = dataclasses.replace(STRICT, allow_raw_inline_formats="drop-all")
        result = sanitize(
            doc(table(table_row(True, table_cell(raw_inline("html", "<b>x</b>"))), align=[None])),
            policy,
        )
        assert result["children"] == []
