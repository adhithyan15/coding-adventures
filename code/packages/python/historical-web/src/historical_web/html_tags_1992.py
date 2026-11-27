"""A tree builder for the November 1992 CERN HTML Tags snapshot."""

from __future__ import annotations

import re
from collections.abc import Iterator, Set
from dataclasses import dataclass, field


@dataclass(slots=True)
class Element:
    """A historical element and its children, in document order."""

    tag: str
    attributes: dict[str, str | None] = field(default_factory=dict)
    children: list[Node] = field(default_factory=list)


type Node = Element | str

_SPACE = r"[ \t\n\r\f]"
_NAME = r"[A-Za-z][A-Za-z0-9-]*"
# Quoted attribute values may contain `>`. A fresh `<` starts a new
# candidate, so malformed markup cannot swallow every later tag.
_TAG = re.compile(rf"""<(/?)({_NAME})({_SPACE}(?:[^<>"']|"[^"<]*"|'[^'<]*')*)?>""")
_ATTRIBUTE = re.compile(
    rf"{_SPACE}+({_NAME})(?:{_SPACE}*={_SPACE}*"
    r"""(?:"([^"]*)"|'([^']*)'|([^ \t\n\r\f"'=<>]+)))?"""
)
_ENTITIES = {"&lt;": "<", "&gt;": ">", "&amp;": "&"}
_ENTITY = re.compile(r"&(?:lt|gt|amp);")
_HEADINGS = frozenset({"h1", "h2", "h3", "h4", "h5", "h6"})
_LISTS = frozenset({"ul", "menu", "dir"})
_TERMS = frozenset({"dt", "dd"})
_EMPTY = frozenset({"p", "isindex"})
_END_LITERAL = {
    name: re.compile(f"</{name}>", re.ASCII | re.IGNORECASE)
    for name in ("xmp", "listing")
}
_KNOWN = (
    _HEADINGS
    | _LISTS
    | _TERMS
    | _EMPTY
    | {
        "title",
        "address",
        "a",
        "dl",
        "li",
        "xmp",
        "listing",
        "plaintext",
    }
)


@dataclass(slots=True)
class _Tag:
    name: str
    closing: bool
    attributes: str


def _decode(text: str) -> str:
    # One replacement pass keeps `&amp;lt;` as literal `&lt;`.
    return _ENTITY.sub(lambda match: _ENTITIES[match[0]], text)


def _tokens(source: str) -> Iterator[_Tag | str]:
    """Yield tags and text. The literal modes bypass entity expansion."""
    cursor = 0
    while cursor < len(source):
        start = source.find("<", cursor)
        if start == -1:
            yield _decode(source[cursor:])
            return
        if start > cursor:
            yield _decode(source[cursor:start])

        if source.startswith("<!--", start):
            end = source.find("-->", start + 4)
            if end == -1:
                return
            cursor = end + 3
            continue

        match = _TAG.match(source, start)
        if match is None:
            yield "<"
            cursor = start + 1
            continue

        closing, name, attributes = match.groups()
        name = name.lower()
        yield _Tag(name, bool(closing), attributes or "")
        cursor = match.end()
        if closing:
            continue
        if name == "plaintext":
            yield source[cursor:]
            return
        if name in _END_LITERAL:
            end_tag = _END_LITERAL[name].search(source, cursor)
            if end_tag is None:
                yield source[cursor:]
                return
            yield source[cursor : end_tag.start()]
            yield _Tag(name, True, "")
            cursor = end_tag.end()


def _anchor_attributes(source: str) -> dict[str, str | None]:
    attributes: dict[str, str | None] = {}
    cursor = 0
    while (match := _ATTRIBUTE.match(source, cursor)) is not None:
        name, *forms = match.groups()
        name = name.lower()
        if name in {"href", "name", "type"} and name not in attributes:
            value = next((form for form in forms if form is not None), None)
            attributes[name] = _decode(value) if value is not None else None
        cursor = match.end()
    return attributes


def _closing_index(
    positions: dict[str, list[int]], names: Set[str], boundaries: Set[str] = frozenset()
) -> int | None:
    """Find the newest matching stack slot without rescanning the stack.

    The parser frequently has to reject unmatched end tags.  Keeping each
    tag's currently-open indexes makes a deeply nested malformed document
    linear in the number of tokens rather than quadratic in stack depth.
    Boundary tags still have their old scope semantics: a match is valid only
    when it is newer than the newest boundary.
    """
    match = max(
        (indexes[-1] for name in names if (indexes := positions.get(name))),
        default=-1,
    )
    if match < 0:
        return None
    boundary = max(
        (indexes[-1] for name in boundaries if (indexes := positions.get(name))),
        default=-1,
    )
    return match if match > boundary else None


def parse_html_tags_1992(source: str) -> list[Node]:
    """Build the teaching profile's tree using tag omission and a stack."""
    root: list[Node] = []
    stack: list[Element] = []
    positions: dict[str, list[int]] = {}
    text: list[str] = []
    for token in _tokens(source.replace("\r\n", "\n")):
        if isinstance(token, str):
            if token:
                text.append(token)
            continue
        name = token.name
        if name not in _KNOWN or (token.closing and name in _EMPTY):
            continue
        close_at = None
        if token.closing:
            close_at = _closing_index(positions, {name})
            if close_at is None:
                continue
        elif name == "li":
            close_at = _closing_index(positions, {"li"}, _LISTS)
        elif name in _TERMS:
            close_at = _closing_index(positions, _TERMS, {"dl"})
        elif name in _HEADINGS:
            close_at = _closing_index(positions, _HEADINGS)
        # Text belongs to the old parent. An implied close can change the
        # parent for the new element, as with the second <LI> in a list.
        if text:
            children = stack[-1].children if stack else root
            children.append("".join(text))
            text.clear()
        if close_at is not None:
            while len(stack) > close_at:
                popped = stack.pop()
                positions[popped.tag].pop()
        if token.closing:
            continue
        attributes = _anchor_attributes(token.attributes) if name == "a" else {}
        element = Element(name, attributes)
        children = stack[-1].children if stack else root
        children.append(element)
        if name not in _EMPTY:
            positions.setdefault(name, []).append(len(stack))
            stack.append(element)
    if text:
        children = stack[-1].children if stack else root
        children.append("".join(text))
    return root
