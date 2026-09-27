"""Pure, bounded, language-neutral glob matching for repository paths.

``match_path()`` performs no filesystem, process, environment, or platform
access. It implements the build-tool portable glob grammar directly instead
of delegating character classes to a host regular-expression engine.

``*`` matches zero or more Unicode scalar values within one path segment,
``?`` matches exactly one scalar, and ``**`` as a complete segment matches
zero or more path segments. Character classes support leading ``!``
negation, ascending ranges, literal leading or trailing ``-``, literal leading
``]``, and an unmatched ``[`` as a literal. Ambiguous class operators and
descending ranges are rejected before matching.

Both segment matching and path matching use bottom-up dynamic programming.
Their work is bounded by the pattern/value state grids, so adversarial
globstar near misses cannot cause recursive suffix revisits or stack growth.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import cast

__all__ = ["GlobPatternError", "match_path", "validate_pattern"]


class GlobPatternError(ValueError):
    """Raised when a glob uses a rejected portable character class."""


@dataclass(frozen=True, slots=True)
class _Literal:
    value: str


@dataclass(frozen=True, slots=True)
class _Star:
    pass


@dataclass(frozen=True, slots=True)
class _Question:
    pass


_ClassMember = str | tuple[str, str]


@dataclass(frozen=True, slots=True)
class _CharacterClass:
    negated: bool
    members: tuple[_ClassMember, ...]


_Token = _Literal | _Star | _Question | _CharacterClass
_SingleValueToken = _Literal | _CharacterClass
_STAR = _Star()
_QUESTION = _Question()
_INVALID_CLASS_MESSAGE = "ambiguous or descending character class in glob pattern"


def match_path(pattern: str, path: str) -> bool:
    """Return whether *path* matches *pattern* without host-system access.

    Empty components caused by leading, trailing, or repeated slashes are
    ignored for compatibility with the existing Python build-tool callers.
    Validated portable inputs never contain those components.

    Raises
    ------
    GlobPatternError
        If a character class contains a descending range or one of the
        ambiguous operators ``--``, ``&&``, ``~~``, or ``||``.
    """
    matched, _ = _match_path_with_state_count(pattern, path)
    return matched


def validate_pattern(pattern: str) -> None:
    """Reject host-ambiguous or descending classes before candidate matching."""
    _compile_pattern(pattern)


def _match_path_with_state_count(pattern: str, path: str) -> tuple[bool, int]:
    """Match and return the deterministic number of path-DP states visited."""
    path_segments = [segment for segment in path.split("/") if segment]
    compiled = _compile_pattern(pattern)

    path_count = len(path_segments)
    next_row = [False] * (path_count + 1)
    next_row[path_count] = True
    visited = path_count + 1

    for segment in reversed(compiled):
        row = [False] * (path_count + 1)
        visited += path_count + 1
        if segment is None:
            row[path_count] = next_row[path_count]
            for path_index in range(path_count - 1, -1, -1):
                row[path_index] = next_row[path_index] or row[path_index + 1]
        else:
            for path_index in range(path_count - 1, -1, -1):
                row[path_index] = next_row[path_index + 1] and _match_segment(
                    segment,
                    path_segments[path_index],
                )
        next_row = row

    return next_row[0], visited


def _compile_pattern(pattern: str) -> list[tuple[_Token, ...] | None]:
    pattern_segments = [segment for segment in pattern.split("/") if segment]
    return [
        None if segment == "**" else _parse_segment(segment)
        for segment in pattern_segments
    ]


def _parse_segment(segment: str) -> tuple[_Token, ...]:
    """Compile one slash-free glob segment into host-independent tokens."""
    tokens, _ = _parse_segment_with_state_count(segment)
    return tokens


def _parse_segment_with_state_count(segment: str) -> tuple[tuple[_Token, ...], int]:
    """Compile a segment and report bounded parser states for regression tests."""
    next_closing_bracket: list[int | None] = [None] * (len(segment) + 1)
    next_closing: int | None = None
    for position in range(len(segment) - 1, -1, -1):
        if segment[position] == "]":
            next_closing = position
        next_closing_bracket[position] = next_closing

    tokens: list[_Token] = []
    index = 0
    visited = len(segment)
    while index < len(segment):
        visited += 1
        character = segment[index]
        if character == "*":
            if not tokens or tokens[-1] is not _STAR:
                tokens.append(_STAR)
            index += 1
            continue
        if character == "?":
            tokens.append(_QUESTION)
            index += 1
            continue
        if character != "[":
            tokens.append(_Literal(character))
            index += 1
            continue

        parsed = _parse_character_class(segment, index, next_closing_bracket)
        if parsed is None:
            tokens.append(_Literal("["))
            index += 1
            continue
        token, index = parsed
        tokens.append(token)

    return tuple(tokens), visited


def _parse_character_class(
    segment: str,
    opening: int,
    next_closing_bracket: list[int | None],
) -> tuple[_CharacterClass, int] | None:
    cursor = opening + 1
    negated = cursor < len(segment) and segment[cursor] == "!"
    if negated:
        cursor += 1

    closing = next_closing_bracket[cursor]
    if closing == cursor:
        closing = next_closing_bracket[cursor + 1]
    if closing is None:
        return None

    body = segment[cursor:closing]
    if any(operator in body for operator in ("--", "&&", "~~", "||")):
        raise GlobPatternError(_INVALID_CLASS_MESSAGE)

    members: list[_ClassMember] = []
    member = cursor
    while member < closing:
        if member + 2 < closing and segment[member + 1] == "-":
            start = segment[member]
            end = segment[member + 2]
            if ord(start) > ord(end):
                raise GlobPatternError(_INVALID_CLASS_MESSAGE)
            members.append((start, end))
            member += 3
        else:
            members.append(segment[member])
            member += 1

    return _CharacterClass(negated, tuple(members)), closing + 1


def _match_segment(tokens: tuple[_Token, ...], value: str) -> bool:
    """Match one slash-free value with bounded bottom-up dynamic programming."""
    value_count = len(value)
    next_row = [False] * (value_count + 1)
    next_row[value_count] = True

    for token in reversed(tokens):
        row = [False] * (value_count + 1)
        if token is _STAR:
            row[value_count] = next_row[value_count]
            for value_index in range(value_count - 1, -1, -1):
                row[value_index] = next_row[value_index] or row[value_index + 1]
        elif token is _QUESTION:
            for value_index in range(value_count):
                row[value_index] = next_row[value_index + 1]
        else:
            single_value_token = cast(_SingleValueToken, token)
            for value_index in range(value_count):
                row[value_index] = next_row[value_index + 1] and _token_matches(
                    single_value_token,
                    value[value_index],
                )
        next_row = row

    return next_row[0]


def _token_matches(token: _SingleValueToken, value: str) -> bool:
    if isinstance(token, _Literal):
        return token.value == value

    matched = False
    for member in token.members:
        if isinstance(member, str):
            matched = matched or member == value
        else:
            start, end = member
            matched = matched or start <= value <= end
    return not matched if token.negated else matched
