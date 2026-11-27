"""A tree builder for the November 1992 CERN HTML Tags snapshot."""

from __future__ import annotations

from collections.abc import Iterator, Set
from dataclasses import dataclass, field


@dataclass(slots=True)
class Element:
    """A historical element and its children, in document order."""

    tag: str
    attributes: dict[str, str | None] = field(default_factory=dict)
    children: list[Node] = field(default_factory=list)


type Node = Element | str

_ENTITIES = {"&lt;": "<", "&gt;": ">", "&amp;": "&"}
_HEADINGS = frozenset({"h1", "h2", "h3", "h4", "h5", "h6"})
_LISTS = frozenset({"ul", "menu", "dir"})
_TERMS = frozenset({"dt", "dd"})
_EMPTY = frozenset({"p", "isindex"})
_END_LITERAL = frozenset(("xmp", "listing"))
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
    result: list[str] = []
    cursor = 0
    while cursor < len(text):
        start = text.find("&", cursor)
        if start < 0:
            result.append(text[cursor:])
            break
        result.append(text[cursor:start])
        for entity, replacement in _ENTITIES.items():
            if text.startswith(entity, start):
                result.append(replacement)
                cursor = start + len(entity)
                break
        else:
            result.append("&")
            cursor = start + 1
    return "".join(result)


def _is_space(character: str) -> bool:
    return character in " \t\n\r\f"


def _is_name_start(character: str) -> bool:
    return "A" <= character <= "Z" or "a" <= character <= "z"


def _is_name_character(character: str) -> bool:
    return _is_name_start(character) or "0" <= character <= "9" or character == "-"


def _tag_at(source: str, start: int) -> tuple[bool, str, str, int] | None:
    """Parse one tag with the snapshot's deliberately small grammar."""
    cursor = start + 1
    closing = cursor < len(source) and source[cursor] == "/"
    cursor += closing
    if cursor >= len(source) or not _is_name_start(source[cursor]):
        return None
    name_start = cursor
    cursor += 1
    while cursor < len(source) and _is_name_character(source[cursor]):
        cursor += 1
    name = source[name_start:cursor]
    if cursor == len(source) or source[cursor] == "<":
        return None
    if source[cursor] == ">":
        return closing, name, "", cursor + 1
    if not _is_space(source[cursor]):
        return None
    attributes_start = cursor
    while cursor < len(source):
        character = source[cursor]
        if character == ">":
            return closing, name, source[attributes_start:cursor], cursor + 1
        if character == "<":
            return None
        if character in "\"'":
            quote = character
            cursor += 1
            while cursor < len(source) and source[cursor] not in (quote, "<"):
                cursor += 1
            if cursor >= len(source) or source[cursor] != quote:
                return None
        cursor += 1
    return None


def _raw_end(source: str, cursor: int, name: str) -> tuple[int, int] | None:
    """Find an ASCII-case-insensitive ``</name>`` without regex backtracking."""
    needle = "</" + name + ">"
    while True:
        start = source.find("</", cursor)
        if start < 0:
            return None
        candidate = source[start : start + len(needle)]
        if len(candidate) == len(needle) and all(
            left == right or ("A" <= left <= "Z" and left.lower() == right)
            for left, right in zip(candidate, needle, strict=True)
        ):
            return start, start + len(needle)
        cursor = start + 2


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

        match = _tag_at(source, start)
        if match is None:
            yield "<"
            cursor = start + 1
            continue

        closing, name, attributes, end = match
        name = name.lower()
        yield _Tag(name, bool(closing), attributes or "")
        cursor = end
        if closing:
            continue
        if name == "plaintext":
            yield source[cursor:]
            return
        if name in _END_LITERAL:
            end_tag = _raw_end(source, cursor, name)
            if end_tag is None:
                yield source[cursor:]
                return
            yield source[cursor : end_tag[0]]
            yield _Tag(name, True, "")
            cursor = end_tag[1]


def _anchor_attributes(source: str) -> dict[str, str | None]:
    attributes: dict[str, str | None] = {}
    cursor = 0
    while (match := _attribute_at(source, cursor)) is not None:
        name, value, cursor = match
        name = name.lower()
        if name in {"href", "name", "type"} and name not in attributes:
            attributes[name] = _decode(value) if value is not None else None
    return attributes


def _attribute_at(source: str, cursor: int) -> tuple[str, str | None, int] | None:
    start = cursor
    while cursor < len(source) and _is_space(source[cursor]):
        cursor += 1
    if cursor == start or cursor >= len(source) or not _is_name_start(source[cursor]):
        return None
    name_start = cursor
    cursor += 1
    while cursor < len(source) and _is_name_character(source[cursor]):
        cursor += 1
    name = source[name_start:cursor]
    value_start = cursor
    while cursor < len(source) and _is_space(source[cursor]):
        cursor += 1
    if cursor >= len(source) or source[cursor] != "=":
        return name, None, value_start
    cursor += 1
    while cursor < len(source) and _is_space(source[cursor]):
        cursor += 1
    if cursor >= len(source):
        return name, None, value_start
    if source[cursor] in "\"'":
        quote = source[cursor]
        cursor += 1
        value_start = cursor
        while cursor < len(source) and source[cursor] != quote:
            cursor += 1
        if cursor >= len(source):
            return name, None, value_start
        return name, source[value_start:cursor], cursor + 1
    value_start = cursor
    while (
        cursor < len(source)
        and not _is_space(source[cursor])
        and source[cursor] not in "\"'=<>"
    ):
        cursor += 1
    if cursor == value_start:
        return name, None, value_start
    return name, source[value_start:cursor], cursor


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
