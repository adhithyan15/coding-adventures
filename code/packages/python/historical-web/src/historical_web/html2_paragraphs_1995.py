"""A small, strict text-only paragraph slice of HTML 2.0 (RFC 1866)."""

from __future__ import annotations

from .html_tags_1992 import Element, Node
from .html_tags_1992 import _decode as _decode_archived_text

_OUTSIDE_SPACE = frozenset(" \t\n\r\f")


def _simple_tag_at(source: str, start: int) -> tuple[str, bool, int] | None:
    """Recognize one of the four fixed tags without accepting attributes."""
    for spelling, tag, closing in (
        ("<P>", "p", False),
        ("</P>", "p", True),
        ("<H1>", "h1", False),
        ("</H1>", "h1", True),
    ):
        end = start + len(spelling)
        if end <= len(source):
            candidate = source[start:end]
            if candidate.isascii() and candidate.lower() == spelling.lower():
                return tag, closing, end
    return None


def parse_html2_paragraphs_1995(source: str) -> list[Node]:
    """Parse the text-only P/H1 slice versioned as ``html2-paragraphs-1995``.

    P has an optional end tag: a new P, H1, explicit ``</P>``, or EOF closes
    it. H1 must close explicitly. Unsupported markup is rejected instead of
    being guessed at as general SGML or modern HTML.
    """
    if not isinstance(source, str):
        raise TypeError("source must be a str")

    source = str.replace(source, "\r\n", "\n")
    roots: list[Node] = []
    active: Element | None = None
    text: list[str] = []
    cursor = 0

    def flush_text(*, outside: bool = False) -> None:
        if not text:
            return
        value = _decode_archived_text("".join(text))
        text.clear()
        if outside:
            if any(character not in _OUTSIDE_SPACE for character in value):
                raise ValueError("text outside P or H1 is unsupported")
            return
        if value and active is not None:
            active.children.append(value)

    while cursor < len(source):
        opening = source.find("<", cursor)
        if opening < 0:
            text.append(source[cursor:])
            cursor = len(source)
            break
        if opening > cursor:
            text.append(source[cursor:opening])
        flush_text(outside=active is None)

        parsed = _simple_tag_at(source, opening)
        if parsed is None:
            raise ValueError(f"unsupported markup at character {opening}")
        tag, closing, cursor = parsed

        if closing:
            if active is None or active.tag != tag:
                raise ValueError(f"unmatched </{tag.upper()}>")
            active = None
            continue

        if tag == "p":
            if active is not None and active.tag == "h1":
                raise ValueError("P cannot start inside H1")
            # A second P starts a new paragraph, implying the prior P's end.
            element = Element("p")
        else:
            if active is not None and active.tag == "h1":
                raise ValueError("H1 elements cannot nest")
            # A heading is not permitted by P's %text content model.
            element = Element("h1")

        roots.append(element)
        active = element

    flush_text(outside=active is None)
    if active is not None and active.tag == "h1":
        raise ValueError("H1 requires an explicit closing tag")
    return roots
