"""AsciiDoc inline parser.

The inline parser processes a plain string left-to-right and emits a list of
Document AST inline nodes.

=== AsciiDoc inline constructs ===

AsciiDoc uses a different convention from CommonMark:

  - ``*bold*``   → StrongNode   (not EmphasisNode!)
  - ``_italic_`` → EmphasisNode
  - ``**bold**`` → StrongNode   (unconstrained form)
  - ``__italic__`` → EmphasisNode (unconstrained form)
  - backtick   → CodeSpanNode  (verbatim, no nested parsing)
  - link:url[text] → LinkNode
  - image:url[alt] → ImageNode
  - <<anchor,text>> → LinkNode { destination: "#anchor" }
  - https://url[text] → LinkNode; bare https://... → AutolinkNode
  - two trailing spaces + newline → HardBreakNode
  - backslash + newline → HardBreakNode
  - bare newline → SoftBreakNode

=== Priority order ===

Checked left-to-right, earlier rules have priority:
  1. Hard break (two spaces + \\n or backslash + \\n)
  2. Soft break (bare \\n)
  3. Backtick → CodeSpanNode
  4. ``**`` → StrongNode (unconstrained — check BEFORE ``*``)
  5. ``__`` → EmphasisNode (unconstrained — check BEFORE ``_``)
  6. ``*``  → StrongNode  (constrained)
  7. ``_``  → EmphasisNode (constrained)
  8. ``link:`` → LinkNode
  9. ``image:`` → ImageNode
 10. ``<<`` → cross-reference LinkNode
 11. ``https://`` / ``http://`` → LinkNode or AutolinkNode
 12. Anything else → TextNode (character accumulation)

=== Verbatim code spans ===

Content between backticks is never recursively parsed. The raw string
(with surrounding whitespace collapsed per AsciiDoc rules) becomes the
``value`` of a CodeSpanNode.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from coding_adventures_document_ast import InlineNode

# ── Node constructors ────────────────────────────────────────────────────────
# We build plain TypedDict-compatible dicts directly to avoid importing the
# heavier factory functions from document_ast (they are just constructors).


def _text(value: str) -> "InlineNode":
    """Create a text node."""
    return {"type": "text", "value": value}  # type: ignore[return-value]


def _soft_break() -> "InlineNode":
    return {"type": "soft_break"}  # type: ignore[return-value]


def _hard_break() -> "InlineNode":
    return {"type": "hard_break"}  # type: ignore[return-value]


def _code_span(value: str) -> "InlineNode":
    return {"type": "code_span", "value": value}  # type: ignore[return-value]


def _strong(children: list["InlineNode"]) -> "InlineNode":
    return {"type": "strong", "children": children}  # type: ignore[return-value]


def _emphasis(children: list["InlineNode"]) -> "InlineNode":
    return {"type": "emphasis", "children": children}  # type: ignore[return-value]


def _link(destination: str, title: str | None, children: list["InlineNode"]) -> "InlineNode":
    return {"type": "link", "destination": destination, "title": title, "children": children}  # type: ignore[return-value]


def _image(destination: str, title: str | None, alt: str) -> "InlineNode":
    return {"type": "image", "destination": destination, "title": title, "alt": alt}  # type: ignore[return-value]


def _autolink(destination: str, is_email: bool) -> "InlineNode":
    return {"type": "autolink", "destination": destination, "is_email": is_email}  # type: ignore[return-value]


# ── Helpers ──────────────────────────────────────────────────────────────────


def _flush_text(buf: list[str], out: list["InlineNode"]) -> None:
    """Flush accumulated text characters into a TextNode.

    This helper is called whenever the scanner encounters a special token.
    If the text buffer is non-empty, its contents are joined and appended
    as a TextNode before processing the special token.

    Example:
        buf = ['H', 'e', 'l', 'l', 'o', ' ']
        out = []
        _flush_text(buf, out)
        # out == [{"type": "text", "value": "Hello "}]
        # buf == []
    """
    if buf:
        out.append(_text("".join(buf)))
        buf.clear()


def _find_closing(text: str, start: int, delimiter: str) -> int:
    """Find the next occurrence of *delimiter* in *text* starting at *start*.

    Returns the index of the first character of the delimiter, or -1 if not
    found. This is used to detect closing markup tokens such as ``**`` or ``_``.
    """
    return text.find(delimiter, start)


# ── Main entry point ─────────────────────────────────────────────────────────


def parse_inline(text: str) -> list["InlineNode"]:
    """Parse an AsciiDoc inline string into a list of Document AST inline nodes.

    The parser is a greedy left-to-right character scanner. At each position
    it tries to match the highest-priority inline construct. If nothing matches,
    the character is added to the text accumulation buffer.

    === Character-by-character walk ===

    We use an integer index ``i`` that advances through the string. When we
    match a multi-character token (e.g. ``**``), we advance ``i`` by the token
    length plus the length of the content and closing delimiter.

    Example input: ``"Hello *world* and __italic__"``
    Produces:
        TextNode("Hello ")
        StrongNode([TextNode("world")])
        TextNode(" and ")
        EmphasisNode([TextNode("italic")])

    @param text  The inline AsciiDoc string to parse.
    @returns     A (possibly empty) list of inline AST nodes.
    """
    out: list[InlineNode] = []
    buf: list[str] = []
    i = 0
    n = len(text)

    while i < n:
        ch = text[i]

        # ── Rule 1: Hard break — two trailing spaces before \\n ──────────────
        # AsciiDoc: two spaces at the end of a line force a <br />.
        # We normalise this the same way CommonMark does.
        if ch == " " and text[i : i + 2] == "  " and i + 2 < n and text[i + 2] == "\n":
            _flush_text(buf, out)
            out.append(_hard_break())
            i += 3  # skip "  \n"
            continue

        # ── Rule 1b: Hard break — backslash + \\n ────────────────────────────
        if ch == "\\" and i + 1 < n and text[i + 1] == "\n":
            _flush_text(buf, out)
            out.append(_hard_break())
            i += 2
            continue

        # ── Rule 2: Soft break — bare \\n ───────────────────────────────────
        if ch == "\n":
            _flush_text(buf, out)
            out.append(_soft_break())
            i += 1
            continue

        # ── Rule 3: Code span — backtick ────────────────────────────────────
        # Content between backticks is verbatim — no nested inline parsing.
        # We search for the matching closing backtick. If none is found, the
        # opening backtick is treated as a literal character.
        if ch == "`":
            close = _find_closing(text, i + 1, "`")
            if close != -1:
                _flush_text(buf, out)
                out.append(_code_span(text[i + 1 : close]))
                i = close + 1
                continue

        # ── Rule 4: Strong — unconstrained ** (must come BEFORE single *) ───
        if text[i : i + 2] == "**":
            close = _find_closing(text, i + 2, "**")
            if close != -1:
                _flush_text(buf, out)
                inner = parse_inline(text[i + 2 : close])
                out.append(_strong(inner))
                i = close + 2
                continue

        # ── Rule 5: Emphasis — unconstrained __ (must come BEFORE single _) ─
        if text[i : i + 2] == "__":
            close = _find_closing(text, i + 2, "__")
            if close != -1:
                _flush_text(buf, out)
                inner = parse_inline(text[i + 2 : close])
                out.append(_emphasis(inner))
                i = close + 2
                continue

        # ── Rule 6: Strong — constrained * ──────────────────────────────────
        # In AsciiDoc, single-asterisk means STRONG (bold), not emphasis.
        if ch == "*":
            close = _find_closing(text, i + 1, "*")
            if close != -1:
                _flush_text(buf, out)
                inner = parse_inline(text[i + 1 : close])
                out.append(_strong(inner))
                i = close + 1
                continue

        # ── Rule 7: Emphasis — constrained _ ────────────────────────────────
        if ch == "_":
            close = _find_closing(text, i + 1, "_")
            if close != -1:
                _flush_text(buf, out)
                inner = parse_inline(text[i + 1 : close])
                out.append(_emphasis(inner))
                i = close + 1
                continue

        # ── Rule 8: link:url[text] ───────────────────────────────────────────
        # AsciiDoc link macro: ``link:https://example.com[Link text]``
        if text[i:].startswith("link:"):
            rest = text[i + 5 :]
            bracket_open = rest.find("[")
            if bracket_open != -1:
                bracket_close = rest.find("]", bracket_open)
                if bracket_close != -1:
                    url = rest[:bracket_open]
                    label = rest[bracket_open + 1 : bracket_close]
                    link_text = label if label else url
                    _flush_text(buf, out)
                    out.append(_link(url, None, [_text(link_text)]))
                    i += 5 + bracket_close + 1
                    continue

        # ── Rule 9: image:url[alt] ───────────────────────────────────────────
        # AsciiDoc image macro: ``image:cat.png[A cat]``
        if text[i:].startswith("image:"):
            rest = text[i + 6 :]
            bracket_open = rest.find("[")
            if bracket_open != -1:
                bracket_close = rest.find("]", bracket_open)
                if bracket_close != -1:
                    url = rest[:bracket_open]
                    alt = rest[bracket_open + 1 : bracket_close]
                    _flush_text(buf, out)
                    out.append(_image(url, None, alt))
                    i += 6 + bracket_close + 1
                    continue

        # ── Rule 10: <<anchor>> and <<anchor,text>> ──────────────────────────
        # AsciiDoc cross-reference: ``<<section-id,Section Title>>``
        # Renders as a link with destination ``#anchor``.
        if text[i : i + 2] == "<<":
            close = text.find(">>", i + 2)
            if close != -1:
                inner_text = text[i + 2 : close]
                if "," in inner_text:
                    anchor, label = inner_text.split(",", 1)
                    anchor = anchor.strip()
                    label = label.strip()
                else:
                    anchor = inner_text.strip()
                    label = anchor
                _flush_text(buf, out)
                out.append(_link(f"#{anchor}", None, [_text(label)]))
                i = close + 2
                continue

        # ── Rule 11: https:// and http:// URLs ──────────────────────────────
        # If followed by ``[text]``, produces a LinkNode.
        # Otherwise produces an AutolinkNode (bare URL).
        for scheme in ("https://", "http://"):
            if text[i:].startswith(scheme):
                # Scan to end of URL (stops at whitespace or ``[``)
                j = i + len(scheme)
                while j < n and text[j] not in (" ", "\t", "\n", "[", "]"):
                    j += 1
                url = text[i:j]
                if j < n and text[j] == "[":
                    # Explicit link text: https://url[text]
                    bracket_close = text.find("]", j + 1)
                    if bracket_close != -1:
                        label = text[j + 1 : bracket_close]
                        link_children = [_text(label)] if label else [_text(url)]
                        _flush_text(buf, out)
                        out.append(_link(url, None, link_children))
                        i = bracket_close + 1
                        break
                # Bare URL → AutolinkNode
                _flush_text(buf, out)
                out.append(_autolink(url, is_email=False))
                i = j
                break
        else:
            # ── Rule 12: Text accumulation ───────────────────────────────────
            # None of the special patterns matched; treat this character as
            # plain text.
            buf.append(ch)
            i += 1
            continue

        # If we hit a ``break`` from the scheme loop, continue the outer loop.
        continue

    # Flush any remaining text buffer.
    _flush_text(buf, out)
    return out
