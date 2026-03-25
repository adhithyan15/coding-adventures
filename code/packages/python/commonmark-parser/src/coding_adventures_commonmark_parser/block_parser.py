"""Block-Level Parser.

Phase 1 of CommonMark parsing: split the input into block-level tokens
and build the structural skeleton of the document.

=== Two-Phase Overview ===

CommonMark parsing is inherently two-phase:

  Phase 1 (this file): Block structure
    Input text → lines → block tree with raw inline content strings

  Phase 2 (inline_parser.py): Inline content
    Each block's raw content → inline nodes (emphasis, links, etc.)

The phases cannot be merged because block structure determines where
inline content lives. A `*` that starts a list item is structural;
a `*` inside paragraph text may be emphasis.

=== Block Tree Construction ===

Container blocks (document, blockquote, list items) form a stack.
When a new line arrives, we walk down the stack checking continuations,
then add the line's content to the appropriate block.

=== Block Priority Order ===

When detecting a new block type on a line, CommonMark has a strict priority:
  1. Fenced code block opener
  2. ATX heading
  3. Thematic break (check before list marker to avoid --- confusion)
  4. Setext heading underline
  5. HTML block (types 1-7)
  6. Blockquote
  7. List item
  8. Indented code block
  9. Paragraph continuation or new paragraph
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from coding_adventures_document_ast import (
    BlockNode,
    BlockquoteNode,
    CodeBlockNode,
    DocumentNode,
    HeadingNode,
    ListItemNode,
    ListNode,
    ParagraphNode,
    RawBlockNode,
    ThematicBreakNode,
)

from coding_adventures_commonmark_parser.entities import decode_entities
from coding_adventures_commonmark_parser.scanner import (
    is_ascii_punctuation,
    normalize_link_label,
    normalize_url,
)


def _apply_backslash_escapes(s: str) -> str:
    """Apply backslash escapes — only for ASCII punctuation characters.

    Per CommonMark §2.4: any ASCII punctuation character may be backslash-escaped.
    All other characters preceded by a backslash are treated literally.

    Examples:
        "\\*"  → "*"   (punctuation escape)
        "\\\\" → "\\"  (backslash is punctuation)
        "\\b"  → "\\b" (not punctuation — kept as-is)
    """
    result = []
    i = 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            next_ch = s[i + 1]
            if is_ascii_punctuation(next_ch):
                result.append(next_ch)
                i += 2
            else:
                result.append("\\")
                i += 1
        else:
            result.append(s[i])
            i += 1
    return "".join(result)


# ─── Internal Block Representations ──────────────────────────────────────────
#
# During parsing we use mutable intermediate representations, then convert
# them into the final readonly AST types at the end.


@dataclass
class MutableDocument:
    kind: str = "document"
    children: list[MutableBlock] = field(default_factory=list)


@dataclass
class MutableBlockquote:
    kind: str = "blockquote"
    children: list[MutableBlock] = field(default_factory=list)


@dataclass
class MutableList:
    kind: str = "list"
    ordered: bool = False
    marker: str = "-"       # the marker character: - * + or ) .
    start: int = 1
    tight: bool = True
    items: list[MutableListItem] = field(default_factory=list)
    had_blank_line: bool = False  # track blank lines between items


@dataclass
class MutableListItem:
    kind: str = "list_item"
    marker: str = "-"
    marker_indent: int = 0    # indentation of the marker
    content_indent: int = 0   # how many spaces of indentation the content needs
    children: list[MutableBlock] = field(default_factory=list)
    had_blank_line: bool = False  # blank line inside this item


@dataclass
class MutableParagraph:
    kind: str = "paragraph"
    lines: list[str] = field(default_factory=list)  # raw lines, joined with \n before inline parsing


@dataclass
class MutableFencedCode:
    kind: str = "fenced_code"
    fence: str = "```"     # the opening fence characters (``` or ~~~)
    fence_len: int = 3     # length of the fence (>= 3)
    base_indent: int = 0   # indentation of opening fence (0-3), stripped from content lines
    info_string: str = ""
    lines: list[str] = field(default_factory=list)
    closed: bool = False


@dataclass
class MutableIndentedCode:
    kind: str = "indented_code"
    lines: list[str] = field(default_factory=list)  # each line with 4 leading spaces stripped


@dataclass
class MutableHtmlBlock:
    kind: str = "html_block"
    html_type: int = 1  # 1-7
    lines: list[str] = field(default_factory=list)
    closed: bool = False


@dataclass
class MutableHeading:
    kind: str = "heading"
    level: int = 1  # 1-6
    content: str = ""  # raw inline content


@dataclass
class MutableThematicBreak:
    kind: str = "thematic_break"


@dataclass
class MutableLinkDef:
    kind: str = "link_def"
    label: str = ""
    destination: str = ""
    title: str | None = None


MutableBlock = (
    MutableDocument
    | MutableBlockquote
    | MutableList
    | MutableListItem
    | MutableParagraph
    | MutableFencedCode
    | MutableIndentedCode
    | MutableHtmlBlock
    | MutableHeading
    | MutableThematicBreak
    | MutableLinkDef
)


# ─── Parser State ─────────────────────────────────────────────────────────────

# Tracks the current multi-line block parsing mode
_MODE_NORMAL = "normal"
_MODE_FENCED = "fenced"
_MODE_HTML = "html_block"


# ─── HTML Block Pattern Helpers ───────────────────────────────────────────────
#
# CommonMark defines 7 types of HTML blocks. Each has different opening
# and closing conditions. Types 1-5 end on specific closing tags/markers.
# Types 6-7 end on a blank line.

_HTML_BLOCK_1_OPEN = re.compile(r"^<(?:script|pre|textarea|style)(?:\s|>|$)", re.IGNORECASE)
_HTML_BLOCK_1_CLOSE = re.compile(r"</(?:script|pre|textarea|style)>", re.IGNORECASE)
_HTML_BLOCK_2_OPEN = re.compile(r"^<!--")
# CommonMark spec §4.6: type-2 HTML blocks end at --> (standard comment close).
# Also accept --!> per HTML5 §13.2.6.2, which some parsers treat as comment end.
_HTML_BLOCK_2_CLOSE = re.compile(r"--!?>")
_HTML_BLOCK_3_OPEN = re.compile(r"^<\?")
_HTML_BLOCK_3_CLOSE = re.compile(r"\?>")
_HTML_BLOCK_4_OPEN = re.compile(r"^<![A-Z]")
_HTML_BLOCK_4_CLOSE = re.compile(r">")
_HTML_BLOCK_5_OPEN = re.compile(r"^<!\[CDATA\[")
_HTML_BLOCK_5_CLOSE = re.compile(r"\]\]>")

# Type 6: open/close tag for block-level HTML elements
_HTML_BLOCK_6_TAGS = frozenset([
    "address", "article", "aside", "base", "basefont", "blockquote", "body",
    "caption", "center", "col", "colgroup", "dd", "details", "dialog", "dir",
    "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form",
    "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
    "hr", "html", "iframe", "legend", "li", "link", "main", "menu", "menuitem",
    "meta", "nav", "noframes", "ol", "optgroup", "option", "p", "param",
    "search", "section", "summary", "table", "tbody", "td", "tfoot", "th",
    "thead", "title", "tr", "track", "ul",
])

_HTML_BLOCK_6_OPEN = re.compile(
    r"^</?(?:" + "|".join(sorted(_HTML_BLOCK_6_TAGS)) + r")(?:\s|>|/>|$)",
    re.IGNORECASE,
)

# Type 7: a complete open tag, closing tag, or processing instruction
# that is NOT in the type 6 list. Ends on blank line.
_HTML_BLOCK_7_ATTR = r"""(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"'=<>`]+|'[^'\n]*'|"[^"\n]*"))?)"""
_HTML_BLOCK_7_OPEN_TAG = re.compile(
    r"^<[A-Za-z][A-Za-z0-9\-]*(" + _HTML_BLOCK_7_ATTR + r")*\s*/?>$"
)
_HTML_BLOCK_7_CLOSE_TAG = re.compile(r"^<\/[A-Za-z][A-Za-z0-9\-]*\s*>$")


def _detect_html_block_type(line: str) -> int | None:
    """Detect whether a line starts an HTML block (types 1-7). Returns the type or None."""
    stripped = line.lstrip()
    if _HTML_BLOCK_1_OPEN.match(stripped):
        return 1
    if _HTML_BLOCK_2_OPEN.match(stripped):
        return 2
    if _HTML_BLOCK_3_OPEN.match(stripped):
        return 3
    if _HTML_BLOCK_4_OPEN.match(stripped):
        return 4
    if _HTML_BLOCK_5_OPEN.match(stripped):
        return 5
    if _HTML_BLOCK_6_OPEN.match(stripped):
        return 6
    # Type 7: complete open or close tag with valid attribute syntax
    if _HTML_BLOCK_7_OPEN_TAG.match(stripped) or _HTML_BLOCK_7_CLOSE_TAG.match(stripped):
        return 7
    return None


def _html_block_ends(line: str, html_type: int) -> bool:
    """Return True if this line closes the HTML block of the given type."""
    if html_type == 1:
        return bool(_HTML_BLOCK_1_CLOSE.search(line))
    elif html_type == 2:
        return bool(_HTML_BLOCK_2_CLOSE.search(line))
    elif html_type == 3:
        return bool(_HTML_BLOCK_3_CLOSE.search(line))
    elif html_type == 4:
        return bool(_HTML_BLOCK_4_CLOSE.search(line))
    elif html_type == 5:
        return bool(_HTML_BLOCK_5_CLOSE.search(line))
    elif html_type in (6, 7):
        return bool(re.match(r"^\s*$", line))  # blank line ends types 6 and 7
    return False


# ─── Line Classification Helpers ─────────────────────────────────────────────

def _is_blank(line: str) -> bool:
    """True if the line is blank (empty or only whitespace)."""
    return not line or line.strip() == ""


def _indent_of(line: str, base_col: int = 0) -> int:
    """Count leading virtual spaces (expanding tabs to the next 4-column tab stop).

    `base_col` is the virtual column of `line[0]` in the original document —
    necessary after partial-tab stripping, where the string may start mid-tab.
    Returns the number of virtual indentation spaces (relative to base_col).

    Example: _indent_of("  \\tbar", 2) — the two spaces are at cols 2-3, the tab
    starts at col 4 and expands to col 8 (adds 4 virtual spaces). Returns 6.
    """
    col = base_col
    for ch in line:
        if ch == " ":
            col += 1
        elif ch == "\t":
            col += 4 - (col % 4)
        else:
            break
    return col - base_col


def _strip_indent(line: str, n: int, base_col: int = 0) -> tuple[str, int]:
    """Strip exactly `n` virtual spaces of leading indentation, expanding tabs correctly.

    Returns (stripped_line, next_base_col) where next_base_col is the virtual
    column of stripped_line[0].

    **Partial-tab handling**: When a tab spans the strip boundary (the tab
    would expand past n virtual spaces), we consume the tab character and
    prepend the leftover expansion spaces to the result.

    Example: _strip_indent("\\t\\tbar", 2, 0)
      First tab at col 0 → expands 4 spaces; we need 2 → leftover 2 spaces.
      Returns ("  \\tbar", 2)  (two spaces + second tab, base col = 2).
    """
    remaining = n
    col = base_col
    i = 0
    while remaining > 0 and i < len(line):
        if line[i] == " ":
            i += 1
            remaining -= 1
            col += 1
        elif line[i] == "\t":
            w = 4 - (col % 4)
            if w <= remaining:
                i += 1
                remaining -= w
                col += w
            else:
                # Partial tab: consume the tab character but prepend leftover
                leftover = w - remaining
                return " " * leftover + line[i + 1:], col + remaining
        else:
            break
    return line[i:], col


def _virtual_col_after(line: str, char_count: int, start_col: int = 0) -> int:
    """Compute the virtual column reached after consuming `char_count` characters from `line`.

    Used to find the virtual column of the list-marker separator character so
    that a tab separator can be partially consumed.
    """
    col = start_col
    for i in range(min(char_count, len(line))):
        if line[i] == "\t":
            col += 4 - (col % 4)
        else:
            col += 1
    return col


def _extract_info_string(line: str) -> str:
    """Extract the info string from a fenced code block opening line.

    Per CommonMark: only the first word is the language.
    Backslash escapes (punctuation only) and entity decoding are applied.
    """
    m = re.match(r"^[`~]+\s*(.*?)$", line)
    if not m:
        return ""
    raw = m.group(1).strip()
    # Only the first word
    parts = raw.split()
    if not parts:
        return ""
    first_word = parts[0]
    return decode_entities(_apply_backslash_escapes(first_word))


# ─── ATX Heading Detection ────────────────────────────────────────────────────

@dataclass
class _AtxHeading:
    level: int
    content: str


def _parse_atx_heading(line: str) -> _AtxHeading | None:
    """Parse an ATX heading line. Returns None if not a heading."""
    # Up to 3 spaces of indentation, then 1-6 # chars, then space or end-of-line
    m = re.match(r"^ {0,3}(#{1,6})([ \t]|$)(.*)", line)
    if not m:
        return None

    hashes = m.group(1)
    # m.group(3) is everything after the first space (or empty if heading was just hashes)
    content = (m.group(3) or "").rstrip()

    # Remove closing hash sequence: space/tab + one or more hashes + optional spaces
    content = re.sub(r"[ \t]+#+[ \t]*$", "", content)
    # If content is now purely hashes (e.g. `### ###` → content becomes `###`), it was
    # the closing sequence and the heading is empty.
    if re.match(r"^#+[ \t]*$", content):
        content = ""

    return _AtxHeading(level=len(hashes), content=content.strip())


# ─── Thematic Break Detection ─────────────────────────────────────────────────

def _is_thematic_break(line: str) -> bool:
    """Return True if the line is a thematic break (hr)."""
    # 0-3 spaces, then 3+ of *, -, or _ optionally separated by spaces/tabs
    return bool(re.match(r"^ {0,3}((?:\*[ \t]*){3,}|(?:-[ \t]*){3,}|(?:_[ \t]*){3,})\s*$", line))


# ─── List Item Detection ──────────────────────────────────────────────────────

@dataclass
class _ListMarker:
    ordered: bool
    start: int
    marker: str       # the delimiter character: - * + . )
    marker_len: int   # total characters consumed by marker + space
    space_after: int  # spaces/tab after marker (0 for end-of-line items)
    indent: int       # spaces before marker


def _parse_list_marker(line: str) -> _ListMarker | None:
    """Detect a list marker at the start of a line. Returns None if not found."""
    # Unordered: up to 3 spaces + (- * +) + (space, tab, or end-of-line)
    m = re.match(r"^( {0,3})([-*+])( +|\t|$)", line)
    if m:
        indent = len(m.group(1))
        marker = m.group(2)
        space = m.group(3)
        return _ListMarker(
            ordered=False,
            start=1,
            marker=marker,
            marker_len=indent + 1 + len(space),
            space_after=len(space),
            indent=indent,
        )

    # Ordered: up to 3 spaces + 1-9 digits + (. or )) + (space, tab, or end-of-line)
    m = re.match(r"^( {0,3})(\d{1,9})([.)])( +|\t|$)", line)
    if m:
        indent = len(m.group(1))
        num = int(m.group(2))
        delim = m.group(3)
        space = m.group(4)
        marker_width = len(m.group(2)) + 1  # digits + delimiter
        return _ListMarker(
            ordered=True,
            start=num,
            marker=delim,
            marker_len=indent + marker_width + len(space),
            space_after=len(space),
            indent=indent,
        )

    return None


# ─── Setext Heading Detection ─────────────────────────────────────────────────

def _is_setext_underline(line: str) -> int | None:
    """Return 1 or 2 if the line is a setext heading underline, else None."""
    if re.match(r"^ {0,3}=+\s*$", line):
        return 1
    if re.match(r"^ {0,3}-+\s*$", line):
        return 2
    return None


# ─── Link Reference Definition Parsing ───────────────────────────────────────

@dataclass
class _ParsedLinkDef:
    label: str
    destination: str
    title: str | None
    chars_consumed: int  # total characters consumed (including newlines)


def _parse_link_definition(text: str) -> _ParsedLinkDef | None:
    """Attempt to parse a link reference definition from the start of `text`.

    Link reference definitions have the form:
      [label]: destination "optional title"

    They can span multiple lines. The destination can be:
      - In angle brackets: <url>
      - Without angle brackets: any non-whitespace non-control sequence
    The title is optional and can be in "double quotes", 'single quotes', or (parens).
    """
    # Link label: up to 3 leading spaces + [...]:
    # Labels may NOT contain unescaped [ (spec §4.7).
    label_match = re.match(r"^ {0,3}\[([^\]\\\[]*(?:\\.[^\]\\\[]*)*)\]:", text)
    if not label_match:
        return None

    raw_label = label_match.group(1)
    if not raw_label.strip():
        return None  # empty label not allowed

    label = normalize_link_label(raw_label)
    pos = len(label_match.group(0))

    # Skip whitespace (including one newline)
    ws_match = re.match(r"^[ \t]*\n?[ \t]*", text[pos:])
    if ws_match:
        pos += len(ws_match.group(0))

    # Destination: either <...> or non-whitespace non-control chars
    destination = ""
    if pos < len(text) and text[pos] == "<":
        angle_match = re.match(r"^<([^<>\n\\]*(?:\\.[^<>\n\\]*)*)>", text[pos:])
        if not angle_match:
            return None
        destination = normalize_url(decode_entities(_apply_backslash_escapes(angle_match.group(1))))
        pos += len(angle_match.group(0))
    else:
        # Non-angle-bracket destination: no spaces, no control chars, balanced parens
        depth = 0
        start = pos
        while pos < len(text):
            ch = text[pos]
            if ch == "(":
                depth += 1
                pos += 1
            elif ch == ")":
                if depth == 0:
                    break
                depth -= 1
                pos += 1
            elif re.match(r"[\s\x00-\x1f]", ch):
                break
            elif ch == "\\":
                pos += 2  # skip \X pair
            else:
                pos += 1
        if pos == start:
            return None  # empty destination
        destination = normalize_url(decode_entities(_apply_backslash_escapes(text[start:pos])))

    # Optional title
    title: str | None = None
    before_title = pos
    spaces_match = re.match(r"^[ \t]*\n?[ \t]*", text[pos:])
    if spaces_match and len(spaces_match.group(0)) > 0:
        pos += len(spaces_match.group(0))
        if pos < len(text):
            title_char = text[pos]
            close_char = ""
            if title_char == '"':
                close_char = '"'
            elif title_char == "'":
                close_char = "'"
            elif title_char == "(":
                close_char = ")"

            if close_char:
                pos += 1  # skip open char
                title_start = pos
                escaped = False
                while pos < len(text):
                    ch = text[pos]
                    if escaped:
                        escaped = False
                        pos += 1
                        continue
                    if ch == "\\":
                        escaped = True
                        pos += 1
                        continue
                    if ch == close_char:
                        pos += 1
                        break
                    if ch == "\n" and close_char == ")":
                        break  # parens don't allow newlines
                    pos += 1

                if pos > 0 and text[pos - 1] == close_char:
                    title = decode_entities(_apply_backslash_escapes(text[title_start:pos - 1]))
                else:
                    # Failed to parse title — restore position
                    pos = before_title
                    title = None
            else:
                pos = before_title
    else:
        if spaces_match:
            pass  # empty match — don't change pos

    # Must be followed by only whitespace on the rest of the line
    eol_match = re.match(r"^[ \t]*(?:\n|$)", text[pos:])
    if not eol_match:
        # If we had a title parse attempt, maybe the title was not present
        if title is not None:
            pos = before_title
            title = None
            eol_match2 = re.match(r"^[ \t]*(?:\n|$)", text[pos:])
            if not eol_match2:
                return None
            pos += len(eol_match2.group(0))
        else:
            return None
    else:
        pos += len(eol_match.group(0))

    return _ParsedLinkDef(
        label=label,
        destination=destination,
        title=title,
        chars_consumed=pos,
    )


# ─── Main Block Parser ────────────────────────────────────────────────────────

class LinkReference:
    """A resolved link reference definition."""

    def __init__(self, destination: str, title: str | None) -> None:
        self.destination = destination
        self.title = title


LinkRefMap = dict[str, LinkReference]


def parse_blocks(input_text: str) -> tuple[MutableDocument, LinkRefMap]:
    """Parse a CommonMark document into a block-level tree (Phase 1).

    Returns both the tree and the link reference map, which Phase 2
    (inline parsing) uses to resolve [text][label] links.

    The algorithm is a single-pass line-by-line scan:
    1. Normalize line endings to LF
    2. For each line, check container continuation (blockquotes, list items)
    3. Detect the type of new block to start (or continue an existing block)
    4. Accumulate multi-line blocks (fenced code, HTML blocks, paragraphs)
    5. At end-of-input, finalize any open leaf blocks
    """
    # Normalize line endings to LF
    normalized = input_text.replace("\r\n", "\n").replace("\r", "\n")
    raw_lines = normalized.split("\n")
    # The trailing newline at end of input produces a spurious empty string after
    # splitting. Remove it — it is not a real line to process.
    if raw_lines and raw_lines[-1] == "":
        raw_lines.pop()

    link_refs: LinkRefMap = {}
    root = MutableDocument()

    # Container block stack. The innermost open container is at the end.
    # We always start with the document.
    open_containers: list[MutableBlock] = [root]

    # Track the current open leaf block (paragraph, code block, etc.)
    current_leaf: MutableBlock | None = None

    # Track blank lines for list tightness
    last_line_was_blank = False
    # Track the innermost container during the last blank line
    last_blank_inner_container: MutableBlock = root

    # Current parser mode (for multi-line blocks)
    mode = _MODE_NORMAL

    for raw_line in raw_lines:
        orig_blank = _is_blank(raw_line)

        # ── Container continuation ────────────────────────────────────────────
        #
        # Always walk the container stack FIRST so that fenced/html code blocks
        # inside containers get lineContent with container markers already stripped.

        line_content = raw_line
        line_base_col = 0
        new_containers: list[MutableBlock] = [root]
        lazy_paragraph_continuation = False

        container_idx = 1
        while container_idx < len(open_containers):
            container = open_containers[container_idx]

            if container.kind == "blockquote":
                # Strip the blockquote marker `> ` (up to 3 leading spaces, `>`, then
                # optionally 1 space). A tab after `>` is treated as the separator but
                # only 1 virtual space of it is consumed.
                bq_i = 0
                bq_col = line_base_col
                # Strip 0–3 leading spaces
                while bq_i < 3 and bq_i < len(line_content) and line_content[bq_i] == " ":
                    bq_i += 1
                    bq_col += 1
                if bq_i < len(line_content) and line_content[bq_i] == ">":
                    bq_i += 1
                    bq_col += 1
                    if bq_i < len(line_content):
                        if line_content[bq_i] == " ":
                            bq_i += 1
                            bq_col += 1
                        elif line_content[bq_i] == "\t":
                            # Tab at bqCol: expand to next tab stop, consume 1 virtual space
                            w = 4 - (bq_col % 4)
                            bq_i += 1
                            if w > 1:
                                # Partial tab: leftover (w-1) virtual spaces become content prefix
                                line_content = " " * (w - 1) + line_content[bq_i:]
                                line_base_col = bq_col + 1
                                new_containers.append(container)
                                container_idx += 1
                                continue
                            bq_col += w
                    line_content = line_content[bq_i:]
                    line_base_col = bq_col
                    new_containers.append(container)
                    container_idx += 1
                elif (current_leaf and current_leaf.kind == "paragraph"
                      and not orig_blank
                      and not _is_thematic_break(line_content)
                      and not (_indent_of(line_content, line_base_col) < 4
                                and re.match(r"(`{3,}|~{3,})", line_content.lstrip()))
                      and not _parse_atx_heading(line_content)):
                    # Lazy continuation of paragraph inside blockquote
                    lm = _parse_list_marker(line_content)
                    lm_blank_start = lm and _is_blank(line_content[lm.marker_len:]) if lm else False
                    if not lm or lm_blank_start:
                        new_containers.append(container)
                        container_idx += 1
                        lazy_paragraph_continuation = True
                        break
                    break
                else:
                    break

            elif container.kind == "list":
                # Lists themselves pass through — continuation determined by list items
                new_containers.append(container)
                container_idx += 1

            elif container.kind == "list_item":
                item = container  # MutableListItem
                effective_blank = orig_blank or _is_blank(line_content)
                indent = _indent_of(line_content, line_base_col)
                if not effective_blank and indent >= item.content_indent:
                    line_content, line_base_col = _strip_indent(line_content, item.content_indent, line_base_col)
                    new_containers.append(container)
                    container_idx += 1
                elif effective_blank:
                    if item.children or (current_leaf is not None and item is open_containers[container_idx]):
                        new_containers.append(container)
                        container_idx += 1
                    else:
                        break
                elif (current_leaf and current_leaf.kind == "paragraph"
                      and not orig_blank
                      and not _is_thematic_break(line_content)
                      and not _parse_list_marker(line_content)
                      and not (_indent_of(line_content, line_base_col) < 4
                                and re.match(r"(`{3,}|~{3,})", line_content.lstrip()))
                      and not _parse_atx_heading(line_content)):
                    new_containers.append(container)
                    container_idx += 1
                    lazy_paragraph_continuation = True
                    break
                else:
                    break
            else:
                break

        # Save the previous innermost container before updating
        prev_inner_container = open_containers[-1] if open_containers else root
        open_containers = new_containers

        # After stripping container markers, re-check blank status
        blank = orig_blank
        if not blank and _is_blank(line_content):
            blank = True

        # Container exit cleanup: if innermost container changed, finalize any open leaf block
        current_inner_after_continuation = open_containers[-1] if open_containers else root

        # ── Multi-line block continuation ──────────────────────────────────────
        #
        # Fenced code and HTML blocks are handled AFTER container continuation so
        # that lineContent already has container markers stripped.

        # If we're inside a fenced code block, accumulate lines
        if mode == _MODE_FENCED and current_leaf and current_leaf.kind == "fenced_code":
            fence = current_leaf  # MutableFencedCode
            if current_inner_after_continuation is not prev_inner_container:
                # The fenced code's container was dropped — force-close the fence
                fence.closed = True
                mode = _MODE_NORMAL
                current_leaf = None
                # Fall through to normal block processing below
            else:
                stripped = line_content.lstrip()
                # Does this line close the fence?
                fence_char = fence.fence[0]
                closing_fence_re = re.compile(
                    r"^" + re.escape(fence_char) + "{" + str(fence.fence_len) + r",}\s*$"
                )
                other_char = "~" if fence_char == "`" else "`"
                if (_indent_of(line_content, line_base_col) < 4
                        and closing_fence_re.match(stripped)
                        and not stripped.startswith(other_char)):
                    fence.closed = True
                    mode = _MODE_NORMAL
                    current_leaf = None
                else:
                    # Strip the fence's base indentation from each content line
                    fence_line, _ = _strip_indent(line_content, fence.base_indent, line_base_col)
                    fence.lines.append(fence_line)
                last_line_was_blank = orig_blank
                continue

        # If we're inside an HTML block, accumulate lines
        if mode == _MODE_HTML and current_leaf and current_leaf.kind == "html_block":
            html_block = current_leaf  # MutableHtmlBlock
            if current_inner_after_continuation is not prev_inner_container:
                # Container was dropped — force-close the HTML block
                html_block.closed = True
                mode = _MODE_NORMAL
                current_leaf = None
                # Fall through to normal block processing below
            else:
                html_block.lines.append(line_content)
                if _html_block_ends(line_content, html_block.html_type):
                    html_block.closed = True
                    mode = _MODE_NORMAL
                    current_leaf = None
                last_line_was_blank = orig_blank
                continue

        # Finalize the current leaf if we left its container
        if (current_inner_after_continuation is not prev_inner_container
                and current_leaf is not None
                and not lazy_paragraph_continuation):
            _finalize_block(current_leaf, prev_inner_container, link_refs)
            current_leaf = None

        # ── Lazy paragraph continuation ─────────────────────────────────────
        if lazy_paragraph_continuation and current_leaf and current_leaf.kind == "paragraph":
            current_leaf.lines.append(line_content)
            last_line_was_blank = False
            continue

        # If the innermost container is a list (without a currently open item) and
        # this is not a blank line, check whether the line will start a new item.
        # Exception: thematic breaks (e.g. `* * *`) should close the list even if
        # they match the list's marker character.
        while (not blank and len(open_containers) > 1
               and open_containers[-1].kind == "list"):
            lst = open_containers[-1]  # MutableList
            marker = _parse_list_marker(line_content)
            if (marker and lst.ordered == marker.ordered and lst.marker == marker.marker
                    and not _is_thematic_break(line_content)):
                break  # will add a new item to this list
            open_containers.pop()

        # Get the innermost container
        inner_container = open_containers[-1] if open_containers else root

        # ── Blank line handling ───────────────────────────────────────────────
        if blank:
            if current_leaf and current_leaf.kind == "paragraph":
                _finalize_block(current_leaf, inner_container, link_refs)
                current_leaf = None
            elif current_leaf and current_leaf.kind == "indented_code":
                # Blank lines inside indented code are preserved with stripped indentation
                blank_code_line, _ = _strip_indent(raw_line, 4)
                current_leaf.lines.append(blank_code_line)

            if inner_container.kind == "list_item":
                inner_container.had_blank_line = True
            if inner_container.kind == "list":
                inner_container.had_blank_line = True

            last_line_was_blank = True
            last_blank_inner_container = inner_container
            continue

        # ── New block detection ───────────────────────────────────────────────
        #
        # We use a while-True loop so that blockquote detection can update
        # inner_container and re-dispatch without using recursion or goto.

        while True:  # block detect loop

            # After a blank line in a list, any new content makes the list loose.
            if (last_line_was_blank and inner_container.kind == "list"
                    and (last_blank_inner_container.kind == "list"
                         or last_blank_inner_container.kind == "list_item")):
                inner_container.tight = False

            # When content resumes after a blank line inside a list item
            if last_line_was_blank and inner_container.kind == "list_item":
                inner_container.had_blank_line = True

            indent = _indent_of(line_content, line_base_col)

            # 1. Fenced code block opener
            fence_match = re.match(r"(`{3,}|~{3,})", line_content.lstrip())
            if fence_match and indent < 4:
                fence_char = fence_match.group(1)[0]
                fence_len = len(fence_match.group(1))
                info_string = _extract_info_string(line_content)

                # Backtick fences cannot have backticks in info string
                info_line_raw = line_content.lstrip()[fence_len:]
                if fence_char == "`" and "`" in info_line_raw:
                    pass  # fall through to paragraph handling
                else:
                    _close_paragraph(current_leaf, inner_container, link_refs)
                    current_leaf = None

                    fenced_block = MutableFencedCode(
                        fence=fence_char * fence_len,
                        fence_len=fence_len,
                        base_indent=indent,
                        info_string=info_string,
                    )
                    _add_child(inner_container, fenced_block)
                    current_leaf = fenced_block
                    mode = _MODE_FENCED
                    last_line_was_blank = False
                    break

            # 2. ATX heading
            if indent < 4:
                heading = _parse_atx_heading(line_content)
                if heading:
                    _close_paragraph(current_leaf, inner_container, link_refs)
                    current_leaf = None

                    heading_block = MutableHeading(level=heading.level, content=heading.content)
                    _add_child(inner_container, heading_block)
                    current_leaf = None  # headings are single-line
                    last_line_was_blank = False
                    break

            # 3. Thematic break (must check before list marker to avoid --- confusion)
            if indent < 4 and _is_thematic_break(line_content):
                # BUT: if we're in a paragraph, --- might be a setext heading underline
                if current_leaf and current_leaf.kind == "paragraph":
                    level = _is_setext_underline(line_content)
                    if level is not None:
                        para = current_leaf
                        _finalize_block(para, inner_container, link_refs)
                        if para.lines:
                            heading_block = MutableHeading(
                                level=level,
                                content="\n".join(para.lines).strip(),
                            )
                            _remove_last_child(inner_container)
                            _add_child(inner_container, heading_block)
                            current_leaf = None
                            last_line_was_blank = False
                            break
                        # All content was link defs — para is now empty. Fall through to thematic break.
                        _remove_last_child(inner_container)
                        current_leaf = None

                _close_paragraph(current_leaf, inner_container, link_refs)
                current_leaf = None
                _add_child(inner_container, MutableThematicBreak())
                last_line_was_blank = False
                break

            # 4. Setext heading underline (when no thematic break matched)
            if indent < 4 and current_leaf and current_leaf.kind == "paragraph":
                level = _is_setext_underline(line_content)
                if level is not None:
                    para = current_leaf
                    _finalize_block(para, inner_container, link_refs)
                    if para.lines:
                        heading_block = MutableHeading(
                            level=level,
                            content="\n".join(para.lines).strip(),
                        )
                        _remove_last_child(inner_container)
                        _add_child(inner_container, heading_block)
                        current_leaf = None
                        last_line_was_blank = False
                        break
                    # All content was link defs — para is empty. Fall through to new para.
                    _remove_last_child(inner_container)
                    current_leaf = None

            # 5. HTML block
            if indent < 4:
                html_type = _detect_html_block_type(line_content)
                if html_type is not None:
                    # Type 7 cannot interrupt a paragraph
                    if html_type != 7 or not (current_leaf and current_leaf.kind == "paragraph"):
                        _close_paragraph(current_leaf, inner_container, link_refs)
                        current_leaf = None

                        html_block = MutableHtmlBlock(
                            html_type=html_type,
                            lines=[line_content],
                            closed=_html_block_ends(line_content, html_type),
                        )
                        _add_child(inner_container, html_block)

                        if not html_block.closed:
                            current_leaf = html_block
                            mode = _MODE_HTML

                        last_line_was_blank = False
                        break

            # 6. Blockquote
            if indent < 4 and line_content.lstrip().startswith(">"):
                _close_paragraph(current_leaf, inner_container, link_refs)
                current_leaf = None

                # Continue an existing blockquote only if no blank line intervened
                bq_last = _last_child(inner_container)
                if bq_last and bq_last.kind == "blockquote" and not last_line_was_blank:
                    bq = bq_last
                else:
                    bq = MutableBlockquote()
                    _add_child(inner_container, bq)

                open_containers.append(bq)
                # Strip the > marker with tab-aware virtual-column arithmetic
                bq_i = 0
                bq_col = line_base_col
                while bq_i < len(line_content) and line_content[bq_i] == " " and (bq_i - 0) < 3:
                    bq_i += 1
                    bq_col += 1
                if bq_i < len(line_content) and line_content[bq_i] == ">":
                    bq_i += 1
                    bq_col += 1
                    if bq_i < len(line_content):
                        if line_content[bq_i] == " ":
                            bq_i += 1
                            bq_col += 1
                        elif line_content[bq_i] == "\t":
                            w = 4 - (bq_col % 4)
                            bq_i += 1
                            if w > 1:
                                line_content = " " * (w - 1) + line_content[bq_i:]
                                line_base_col = bq_col + 1
                                inner_container = bq
                                if _is_blank(line_content):
                                    break
                                continue  # re-dispatch in blockquote context
                            bq_col += w
                line_content = line_content[bq_i:]
                line_base_col = bq_col
                inner_container = bq

                if _is_blank(line_content):
                    last_line_was_blank = False
                    break

                # Re-dispatch block detection within the blockquote context
                continue  # re-enter the while True loop

            # 7. List item
            if indent < 4:
                marker = _parse_list_marker(line_content)
                if marker is not None:
                    # Check if this continues an existing list or starts a new one.
                    lst: MutableList | None = None

                    if inner_container.kind == "list":
                        existing_list = inner_container
                        if existing_list.ordered == marker.ordered and existing_list.marker == marker.marker:
                            lst = existing_list

                    if lst is None:
                        list_last = _last_child(inner_container)
                        if list_last and list_last.kind == "list":
                            existing_list = list_last
                            if existing_list.ordered == marker.ordered and existing_list.marker == marker.marker:
                                lst = existing_list

                    # Compute the virtual column of itemContent[0]
                    new_line_base_col = _virtual_col_after(line_content, marker.marker_len, line_base_col)
                    item_content = line_content[marker.marker_len:]

                    # Handle tab separator (CommonMark §2.1)
                    if marker.space_after == 1:
                        sep_char = line_content[marker.marker_len - 1] if marker.marker_len <= len(line_content) else ""
                        if sep_char == "\t":
                            sep_col = _virtual_col_after(line_content, marker.marker_len - 1, line_base_col)
                            w = 4 - (sep_col % 4)
                            if w > 1:
                                item_content = " " * (w - 1) + item_content
                                new_line_base_col = sep_col + 1

                    blank_start = _is_blank(item_content)

                    # Empty list items (blank start) cannot interrupt a paragraph to start
                    # a NEW list. Continuing an existing list is always allowed.
                    # Also: ordered lists starting != 1 cannot interrupt to start a new list.
                    para_in_current_container = (
                        current_leaf and current_leaf.kind == "paragraph"
                        and _last_child(inner_container) is current_leaf
                    )
                    can_interrupt_para = (
                        (not marker.ordered or marker.start == 1 or lst is not None)
                        and (not blank_start or not para_in_current_container)
                    )

                    if not (current_leaf and current_leaf.kind == "paragraph") or can_interrupt_para:
                        if lst is None:
                            _close_paragraph(current_leaf, inner_container, link_refs)
                            current_leaf = None
                            lst = MutableList(
                                ordered=marker.ordered,
                                marker=marker.marker,
                                start=marker.start,
                                tight=True,
                                had_blank_line=False,
                            )
                            _add_child(inner_container, lst)
                        else:
                            _close_paragraph(current_leaf, inner_container, link_refs)
                            current_leaf = None
                            # If there was a blank line between items, the list is loose.
                            if (lst.had_blank_line
                                    or (last_line_was_blank
                                        and (last_blank_inner_container.kind == "list"
                                             or last_blank_inner_container.kind == "list_item"))):
                                lst.tight = False
                            lst.had_blank_line = False

                        # Compute content indent (W+1 rule)
                        normal_indent = marker.marker_len
                        reduced_indent = marker.marker_len - marker.space_after + 1
                        content_indent = reduced_indent if (blank_start or marker.space_after >= 5) else normal_indent

                        item = MutableListItem(
                            marker=marker.marker,
                            marker_indent=marker.indent,
                            content_indent=content_indent,
                            had_blank_line=False,
                        )
                        lst.items.append(item)
                        # Push the list only if it's not already the inner container
                        if inner_container is not lst:
                            open_containers.append(lst)
                        open_containers.append(item)

                        if not blank_start:
                            inner_container = item
                            if marker.space_after >= 5:
                                # W+1 rule: virtual col starts at marker end minus the extra spaces
                                line_base_col = _virtual_col_after(
                                    line_content, marker.marker_len - marker.space_after + 1, line_base_col
                                )
                                line_content = " " * (marker.space_after - 1) + item_content
                            else:
                                line_base_col = new_line_base_col
                                line_content = item_content
                            continue  # re-dispatch in item context

                        current_leaf = None
                        last_line_was_blank = False
                        break

            # 8. Indented code block (4+ spaces, but NOT inside a paragraph)
            if indent >= 4 and not (current_leaf and current_leaf.kind == "paragraph"):
                stripped, _ = _strip_indent(line_content, 4, line_base_col)
                if current_leaf and current_leaf.kind == "indented_code":
                    current_leaf.lines.append(stripped)
                else:
                    _close_paragraph(current_leaf, inner_container, link_refs)
                    icb = MutableIndentedCode(lines=[stripped])
                    _add_child(inner_container, icb)
                    current_leaf = icb
                last_line_was_blank = False
                break

            # 9. Paragraph continuation or new paragraph
            if current_leaf and current_leaf.kind == "paragraph":
                current_leaf.lines.append(line_content)
            else:
                _close_paragraph(current_leaf, inner_container, link_refs)
                para = MutableParagraph(lines=[line_content])
                _add_child(inner_container, para)
                current_leaf = para

            last_line_was_blank = False
            break

        # end block detect loop

    # Finalize any remaining open leaf block
    if current_leaf is not None:
        inner_container = open_containers[-1] if open_containers else root
        _finalize_block(current_leaf, inner_container, link_refs)

    return root, link_refs


# ─── Container Helpers ────────────────────────────────────────────────────────

def _last_child(container: MutableBlock) -> MutableBlock | None:
    """Return the last child of a container block, or None."""
    if container.kind == "document":
        return container.children[-1] if container.children else None
    if container.kind == "blockquote":
        return container.children[-1] if container.children else None
    if container.kind == "list_item":
        return container.children[-1] if container.children else None
    return None


def _add_child(container: MutableBlock, block: MutableBlock) -> None:
    """Add `block` as a child of `container`."""
    if container.kind == "document":
        container.children.append(block)
    elif container.kind == "blockquote":
        container.children.append(block)
    elif container.kind == "list_item":
        container.children.append(block)


def _remove_last_child(container: MutableBlock) -> None:
    """Remove the last child of a container block."""
    if container.kind == "document" and container.children:
        container.children.pop()
    elif container.kind == "blockquote" and container.children:
        container.children.pop()
    elif container.kind == "list_item" and container.children:
        container.children.pop()


def _close_paragraph(
    leaf: MutableBlock | None,
    container: MutableBlock,
    link_refs: LinkRefMap,
) -> None:
    """If `leaf` is an open paragraph, finalize it. If it's an indented code block,
    trim trailing blank lines.
    """
    if leaf and leaf.kind == "paragraph":
        _finalize_block(leaf, container, link_refs)
    elif leaf and leaf.kind == "indented_code":
        # Trim trailing blank/whitespace-only lines from indented code blocks
        while leaf.lines and re.match(r"^\s*$", leaf.lines[-1]):
            leaf.lines.pop()


def _finalize_block(
    block: MutableBlock,
    _container: MutableBlock,
    link_refs: LinkRefMap,
) -> None:
    """Finalize a leaf block.

    For paragraphs: extract any leading link reference definitions.
    For indented code blocks: trim trailing blank lines.
    """
    if block.kind == "paragraph":
        # Attempt to extract link reference definitions from the paragraph
        text = "\n".join(block.lines)
        while True:
            defn = _parse_link_definition(text)
            if not defn:
                break
            key = defn.label
            if key not in link_refs:
                link_refs[key] = LinkReference(destination=defn.destination, title=defn.title)
            text = text[defn.chars_consumed:]
        # Update paragraph lines with remaining text
        if not text.strip():
            block.lines = []
        else:
            # Preserve trailing spaces — they are significant for inline hard breaks.
            block.lines = text.split("\n")
            if block.lines:
                block.lines[-1] = block.lines[-1].rstrip()
    elif block.kind == "indented_code":
        # Trim trailing blank lines
        while block.lines and block.lines[-1] == "":
            block.lines.pop()


# ─── AST Conversion ───────────────────────────────────────────────────────────

class BlockParseResult:
    """Result of Phase 1 block parsing."""

    def __init__(
        self,
        document: DocumentNode,
        link_refs: LinkRefMap,
        raw_inline_content: dict[int, str],
    ) -> None:
        self.document = document
        self.link_refs = link_refs
        # Maps node id (id()) → raw inline content string
        self.raw_inline_content = raw_inline_content


def convert_to_ast(
    mutable_doc: MutableDocument,
    link_refs: LinkRefMap,
) -> BlockParseResult:
    """Convert the mutable intermediate document into the final AST.

    Inline content is NOT yet parsed — raw strings are stored in
    `raw_inline_content` for the inline parser to process.

    We use id() of the node dict as the key into raw_inline_content,
    similar to the TypeScript Symbol() approach.
    """
    raw_inline_content: dict[int, str] = {}

    def convert_block(block: MutableBlock) -> BlockNode | None:
        """Recursively convert a mutable block into a final AST node."""

        if block.kind == "document":
            doc_node: DocumentNode = {"type": "document", "children": []}
            children = [convert_block(c) for c in block.children]
            doc_node["children"] = [c for c in children if c is not None]
            return doc_node

        elif block.kind == "heading":
            heading_node: HeadingNode = {
                "type": "heading",
                "level": block.level,  # type: ignore[typeddict-item]
                "children": [],
            }
            # Store raw inline content with node id as key
            raw_inline_content[id(heading_node)] = block.content
            return heading_node

        elif block.kind == "paragraph":
            if not block.lines:
                return None
            # Strip leading whitespace from each line per the CommonMark spec.
            # Trailing spaces are preserved — they signal hard line breaks.
            content = "\n".join(line.lstrip() for line in block.lines)
            # Actually: the TypeScript impl uses lstrip() on each line to strip
            # leading whitespace, but preserve trailing spaces for hard breaks.
            # Let's match that exactly:
            content = "\n".join(re.sub(r"^[ \t]+", "", line) for line in block.lines)
            para_node: ParagraphNode = {"type": "paragraph", "children": []}
            raw_inline_content[id(para_node)] = content
            return para_node

        elif block.kind == "fenced_code":
            value = "\n".join(block.lines)
            if block.lines:
                value += "\n"
            code_node: CodeBlockNode = {
                "type": "code_block",
                "language": block.info_string if block.info_string else None,
                "value": value,
            }
            return code_node

        elif block.kind == "indented_code":
            code_node = {
                "type": "code_block",
                "language": None,
                "value": "\n".join(block.lines) + "\n",
            }
            return code_node

        elif block.kind == "blockquote":
            bq_node: BlockquoteNode = {"type": "blockquote", "children": []}
            children = [convert_block(c) for c in block.children]
            bq_node["children"] = [c for c in children if c is not None]
            return bq_node

        elif block.kind == "list":
            # A list is loose if:
            #   - blank lines appeared between items, OR
            #   - blank lines appeared between blocks within an item that has > 1 block.
            is_tight = (
                block.tight and not block.had_blank_line
                and not any(item.had_blank_line and len(item.children) > 1 for item in block.items)
            )
            list_node: ListNode = {
                "type": "list",
                "ordered": block.ordered,
                "start": block.start if block.ordered else None,
                "tight": is_tight,
                "children": [],
            }
            list_node["children"] = [convert_block(item) for item in block.items]  # type: ignore[list-item]
            return list_node

        elif block.kind == "list_item":
            item_node: ListItemNode = {"type": "list_item", "children": []}
            children = [convert_block(c) for c in block.children]
            item_node["children"] = [c for c in children if c is not None]
            return item_node

        elif block.kind == "thematic_break":
            tb_node: ThematicBreakNode = {"type": "thematic_break"}
            return tb_node

        elif block.kind == "html_block":
            # For type 6/7 blocks, a blank line terminates the block.
            # Trim trailing blank lines.
            lines = list(block.lines)
            while lines and lines[-1].strip() == "":
                lines.pop()
            raw_node: RawBlockNode = {
                "type": "raw_block",
                "format": "html",
                "value": "\n".join(lines) + "\n",
            }
            return raw_node

        elif block.kind == "link_def":
            # Link definitions are resolved into link_refs and NOT emitted into the AST
            return None

        return None

    document = convert_block(mutable_doc)
    if document is None:
        document = {"type": "document", "children": []}

    return BlockParseResult(
        document=document,  # type: ignore[arg-type]
        link_refs=link_refs,
        raw_inline_content=raw_inline_content,
    )
