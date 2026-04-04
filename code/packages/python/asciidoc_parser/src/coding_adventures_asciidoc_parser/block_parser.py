"""AsciiDoc block parser — state machine implementation.

The block parser reads AsciiDoc source text line by line and emits Document AST
block nodes. It uses an explicit state machine to track which block type is
currently being accumulated.

=== States ===

    normal           — between blocks; each new line dispatches to a rule
    paragraph        — accumulating a paragraph's lines
    code_block       — inside a ---- fenced code block
    literal_block    — inside a .... literal block
    passthrough_block — inside a ++++ passthrough block
    quote_block      — inside a ____ quote/blockquote block
    unordered_list   — accumulating * list items
    ordered_list     — accumulating . list items

=== Line dispatch in normal state ===

Checked top-to-bottom:

    blank line        → stay normal (flush any pending list)
    // comment        → skip
    [source,lang]     → record pending_language
    = text            → HeadingNode(level=1)
    == text           → HeadingNode(level=2)  … up to level 6
    ''' (≥3)          → ThematicBreakNode
    ---- (≥4)         → enter code_block
    .... (≥4)         → enter literal_block
    ++++ (≥4)         → enter passthrough_block
    ____ (≥4)         → enter quote_block
    * text / ** text  → unordered list item (level = number of leading *)
    . text / .. text  → ordered list item   (level = number of leading .)
    other             → enter paragraph

=== List nesting ===

AsciiDoc uses repeated markers for nesting:

    * Level 1 item
    ** Level 2 item (nested inside the previous)
    *** Level 3 item

The parser collects (level, text) pairs and then builds a nested tree of
ListNode / ListItemNode values using the _build_nested_list() helper.

=== Quote blocks ===

Content inside ____ is recursively re-parsed by the block parser itself.
The result becomes the children of a BlockquoteNode.

=== Attribute lines ===

A line of the form ``[source,LANG]`` (case-insensitive key) sets
``pending_language`` to LANG. The very next code block (----) consumes
that language tag.
"""

from __future__ import annotations

import re

from coding_adventures_asciidoc_parser.inline_parser import parse_inline

# ── Regex helpers ─────────────────────────────────────────────────────────────

# Matches a heading line: one or more ``=`` signs followed by a space and text.
# Levels beyond 6 are clamped to 6 at construction time.
_HEADING_RE = re.compile(r"^(={1,})\s+(.*)")

# Matches a [source,lang] attribute line.
_SOURCE_ATTR_RE = re.compile(r"^\[source\s*,\s*(\S+?)\s*\]$", re.IGNORECASE)

# Matches a line comment.
_COMMENT_RE = re.compile(r"^//")

# Matches an unordered list item: one or more * followed by a space and text.
_ULIST_RE = re.compile(r"^(\*+)\s+(.*)")

# Matches an ordered list item: one or more . followed by a space and text.
_OLIST_RE = re.compile(r"^(\.+)\s+(.*)")


# ── Node constructor helpers ──────────────────────────────────────────────────
# We build plain dicts that are compatible with the Document AST TypedDicts.
# This avoids heavy imports and keeps the parser self-contained.


def _document(children: list) -> dict:
    return {"type": "document", "children": children}


def _heading(level: int, children: list) -> dict:
    return {"type": "heading", "level": level, "children": children}


def _paragraph(children: list) -> dict:
    return {"type": "paragraph", "children": children}


def _code_block(language: str | None, value: str) -> dict:
    return {"type": "code_block", "language": language, "value": value}


def _blockquote(children: list) -> dict:
    return {"type": "blockquote", "children": children}


def _list_node(ordered: bool, start: int | None, tight: bool, children: list) -> dict:
    return {"type": "list", "ordered": ordered, "start": start, "tight": tight, "children": children}


def _list_item(children: list) -> dict:
    return {"type": "list_item", "children": children}


def _thematic_break() -> dict:
    return {"type": "thematic_break"}


def _raw_block(format_: str, value: str) -> dict:
    return {"type": "raw_block", "format": format_, "value": value}


# ── List nesting builder ──────────────────────────────────────────────────────


def _build_nested_list(items: list[tuple[int, str]], ordered: bool) -> dict:
    """Convert a flat list of (level, text) pairs into a nested ListNode tree.

    AsciiDoc uses repeated markers to indicate nesting depth:

        * Level 1         → level = 1
        ** Level 2        → level = 2
        *** Level 3       → level = 3

    Algorithm: we build the tree by tracking the current nesting stack.
    Each stack frame holds a list of children for one ListNode at a given
    level. When we encounter a deeper level, we push a new frame. When we
    encounter a shallower level, we pop and attach the completed sub-list.

    Example:
        items = [(1, "a"), (2, "b"), (2, "c"), (1, "d")]
        →  ListNode [
               ListItemNode [ParagraphNode("a"),
                             ListNode [
                               ListItemNode [ParagraphNode("b")],
                               ListItemNode [ParagraphNode("c")],
                             ]],
               ListItemNode [ParagraphNode("d")],
           ]

    @param items   List of (level, text) tuples from the parser.
    @param ordered True for ordered (``<ol>``), False for unordered (``<ul>``).
    @returns       A ListNode dict.
    """
    # Stack of (level, children_list) frames.
    # The bottom frame is for the top-level list.
    stack: list[tuple[int, list]] = [(1, [])]

    for level, text in items:
        inline = parse_inline(text)
        item_children: list = [_paragraph(inline)]

        # Pop frames that are deeper than the current level
        while len(stack) > 1 and stack[-1][0] > level:
            completed_level, completed_children = stack.pop()
            sub_list = _list_node(ordered, 1 if ordered else None, True, completed_children)
            # Attach the completed sub-list to the last item of the parent frame
            parent_items = stack[-1][1]
            if parent_items:
                last_item = parent_items[-1]
                last_item["children"].append(sub_list)

        # Push a deeper frame if needed
        if len(stack) == 0 or stack[-1][0] < level:
            stack.append((level, []))

        stack[-1][1].append(_list_item(item_children))

    # Collapse remaining stack frames from deepest to shallowest
    while len(stack) > 1:
        completed_level, completed_children = stack.pop()
        sub_list = _list_node(ordered, 1 if ordered else None, True, completed_children)
        parent_items = stack[-1][1]
        if parent_items:
            last_item = parent_items[-1]
            last_item["children"].append(sub_list)

    top_children = stack[0][1]
    start = 1 if ordered else None
    return _list_node(ordered, start, True, top_children)


# ── Block parser ──────────────────────────────────────────────────────────────


def parse_blocks(text: str) -> dict:
    """Parse AsciiDoc source text into a DocumentNode.

    This is the main entry point for the block parser. It reads the text line
    by line through a state machine and returns a complete DocumentNode.

    === State machine overview ===

    The parser maintains a ``state`` variable and a set of accumulation
    buffers. The outer loop processes one line per iteration; each state
    handler returns the new state.

    @param text  The full AsciiDoc source string.
    @returns     A DocumentNode dict containing all parsed block nodes.
    """
    lines = text.split("\n")
    # Ensure there is always a trailing empty line so that the final paragraph
    # or list gets flushed by the blank-line handler.
    if lines and lines[-1] != "":
        lines.append("")

    blocks: list = []

    # State variables
    state = "normal"
    para_lines: list[str] = []
    code_lines: list[str] = []
    pending_language: str | None = None
    # For list accumulation: list of (level, text) tuples
    list_items: list[tuple[int, str]] = []
    list_ordered: bool = False

    def flush_paragraph() -> None:
        """Emit a ParagraphNode from accumulated paragraph lines."""
        if para_lines:
            # Join lines with newlines so the inline parser sees them as
            # a single string with embedded soft breaks.
            joined = "\n".join(para_lines)
            inline = parse_inline(joined)
            blocks.append(_paragraph(inline))
            para_lines.clear()

    def flush_list() -> None:
        """Emit a ListNode from accumulated list items."""
        if list_items:
            blocks.append(_build_nested_list(list_items, list_ordered))
            list_items.clear()

    for line in lines:
        stripped = line.rstrip()

        # ── State: code_block ────────────────────────────────────────────────
        # We are inside a ---- delimited code block. Accumulate lines verbatim
        # until we see another ---- (≥4 dashes) line as the closing delimiter.
        if state == "code_block":
            if re.match(r"^-{4,}$", stripped):
                # Closing delimiter found — emit the code block
                value = "\n".join(code_lines) + ("\n" if code_lines else "")
                blocks.append(_code_block(pending_language, value))
                pending_language = None
                code_lines.clear()
                state = "normal"
            else:
                code_lines.append(line.rstrip("\n"))
            continue

        # ── State: literal_block ─────────────────────────────────────────────
        # Inside a .... delimited literal block. Same as code_block but uses
        # ``....`` as the closing delimiter. Language is always None.
        if state == "literal_block":
            if re.match(r"^\.{4,}$", stripped):
                value = "\n".join(code_lines) + ("\n" if code_lines else "")
                blocks.append(_code_block(None, value))
                code_lines.clear()
                state = "normal"
            else:
                code_lines.append(line.rstrip("\n"))
            continue

        # ── State: passthrough_block ─────────────────────────────────────────
        # Inside a ++++ passthrough block. Content is emitted as-is as a
        # RawBlockNode with format "html". This allows embedding raw HTML.
        if state == "passthrough_block":
            if re.match(r"^\+{4,}$", stripped):
                value = "\n".join(code_lines) + ("\n" if code_lines else "")
                blocks.append(_raw_block("html", value))
                code_lines.clear()
                state = "normal"
            else:
                code_lines.append(line.rstrip("\n"))
            continue

        # ── State: quote_block ───────────────────────────────────────────────
        # Inside a ____ quote block. Content is recursively parsed and wrapped
        # in a BlockquoteNode.
        if state == "quote_block":
            if re.match(r"^_{4,}$", stripped):
                inner_text = "\n".join(code_lines)
                inner_doc = parse_blocks(inner_text)
                blocks.append(_blockquote(inner_doc["children"]))
                code_lines.clear()
                state = "normal"
            else:
                code_lines.append(line.rstrip("\n"))
            continue

        # ── State: paragraph ─────────────────────────────────────────────────
        # We are accumulating a paragraph. A blank line ends it.
        if state == "paragraph":
            if stripped == "":
                flush_paragraph()
                state = "normal"
            else:
                # Check if this line actually starts a new block construct.
                # If so, flush the paragraph first and fall through to normal
                # dispatch. This handles cases like:
                #
                #     Some text
                #     = Heading (oops)
                #
                # We detect the most common openers here.
                is_new_block = (
                    bool(re.match(r"^={1,6}\s", stripped))
                    or re.match(r"^'{3,}$", stripped) is not None
                    or re.match(r"^-{4,}$", stripped) is not None
                    or re.match(r"^\.{4,}$", stripped) is not None
                    or re.match(r"^\+{4,}$", stripped) is not None
                    or re.match(r"^_{4,}$", stripped) is not None
                )
                if is_new_block:
                    flush_paragraph()
                    state = "normal"
                    # Fall through to normal dispatch below
                else:
                    para_lines.append(stripped)
                    continue

        # ── State: unordered_list ────────────────────────────────────────────
        if state == "unordered_list":
            if stripped == "":
                flush_list()
                state = "normal"
                continue
            m = _ULIST_RE.match(stripped)
            if m:
                list_items.append((len(m.group(1)), m.group(2)))
                continue
            # Non-list, non-blank line — flush the list and handle as normal
            flush_list()
            state = "normal"
            # Fall through to normal dispatch below

        # ── State: ordered_list ──────────────────────────────────────────────
        if state == "ordered_list":
            if stripped == "":
                flush_list()
                state = "normal"
                continue
            m = _OLIST_RE.match(stripped)
            if m:
                list_items.append((len(m.group(1)), m.group(2)))
                continue
            flush_list()
            state = "normal"
            # Fall through to normal dispatch below

        # ── Normal dispatch ───────────────────────────────────────────────────
        # We reach here either because state == "normal" or because a block
        # state was just exited (list/paragraph) and fell through.

        # Blank line — separator between blocks
        if stripped == "":
            continue

        # Line comment — skip silently
        if _COMMENT_RE.match(stripped):
            continue

        # Attribute line [source,lang] — sets pending_language
        m_attr = _SOURCE_ATTR_RE.match(stripped)
        if m_attr:
            pending_language = m_attr.group(1)
            continue

        # Heading: = text through ====== text
        m_heading = _HEADING_RE.match(stripped)
        if m_heading:
            level = min(len(m_heading.group(1)), 6)
            heading_text = m_heading.group(2)
            inline = parse_inline(heading_text)
            blocks.append(_heading(level, inline))
            state = "normal"
            continue

        # Thematic break: three or more single-quotes on a line by themselves
        if re.match(r"^'{3,}$", stripped):
            blocks.append(_thematic_break())
            state = "normal"
            continue

        # Code block: four or more dashes
        if re.match(r"^-{4,}$", stripped):
            state = "code_block"
            continue

        # Literal block: four or more dots
        if re.match(r"^\.{4,}$", stripped):
            state = "literal_block"
            continue

        # Passthrough block: four or more plus signs
        if re.match(r"^\+{4,}$", stripped):
            state = "passthrough_block"
            continue

        # Quote block: four or more underscores
        if re.match(r"^_{4,}$", stripped):
            state = "quote_block"
            continue

        # Unordered list item
        m_ul = _ULIST_RE.match(stripped)
        if m_ul:
            list_ordered = False
            list_items.append((len(m_ul.group(1)), m_ul.group(2)))
            state = "unordered_list"
            continue

        # Ordered list item
        m_ol = _OLIST_RE.match(stripped)
        if m_ol:
            list_ordered = True
            list_items.append((len(m_ol.group(1)), m_ol.group(2)))
            state = "ordered_list"
            continue

        # Plain text → start a paragraph
        para_lines.append(stripped)
        state = "paragraph"

    # ── End of input — flush any remaining state ──────────────────────────────
    flush_paragraph()
    flush_list()

    # Handle unclosed delimited blocks (lenient parsing):
    # emit whatever was accumulated as the appropriate block type.
    if state == "code_block" and code_lines:
        value = "\n".join(code_lines) + "\n"
        blocks.append(_code_block(pending_language, value))
    elif state == "literal_block" and code_lines:
        value = "\n".join(code_lines) + "\n"
        blocks.append(_code_block(None, value))
    elif state == "passthrough_block" and code_lines:
        value = "\n".join(code_lines) + "\n"
        blocks.append(_raw_block("html", value))
    elif state == "quote_block" and code_lines:
        inner_text = "\n".join(code_lines)
        inner_doc = parse_blocks(inner_text)
        blocks.append(_blockquote(inner_doc["children"]))

    return _document(blocks)
