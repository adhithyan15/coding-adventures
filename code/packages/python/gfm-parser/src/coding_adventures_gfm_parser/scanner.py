"""String Scanner and Character Classification.

A cursor-based scanner over a string. Used by both the block parser
(to scan individual lines) and the inline parser (to scan inline
content character by character).

=== Design ===

The scanner maintains a position `pos` into the string. All read
operations advance `pos`. The scanner never backtracks on its own —
callers must save and restore `pos` explicitly when lookahead fails.

This is the same pattern used by hand-rolled recursive descent parsers
everywhere: try to match, if it fails, restore the saved position.

    saved = scanner.pos
    if not scanner.match("```"):
        scanner.pos = saved  # backtrack

=== Character classification ===

GFM cares about several Unicode character categories:
  - ASCII punctuation: !"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~
  - Unicode punctuation (for emphasis rules)
  - ASCII whitespace: space, tab, CR, LF, FF
  - Unicode whitespace
"""

from __future__ import annotations

import re
import unicodedata

# ─── Scanner ──────────────────────────────────────────────────────────────────


class Scanner:
    """Cursor-based string scanner.

    Maintains a `pos` cursor into `source`. All reads advance `pos`.
    Callers save and restore `pos` for backtracking.

    Example usage:
        s = Scanner("hello world")
        assert s.peek() == "h"
        assert s.advance() == "h"
        assert s.pos == 1
        saved = s.pos
        if not s.match("xyz"):
            s.pos = saved  # backtrack
    """

    __slots__ = ("source", "pos")

    def __init__(self, source: str, start: int = 0) -> None:
        self.source = source
        self.pos = start

    @property
    def done(self) -> bool:
        """True if the scanner has consumed all input."""
        return self.pos >= len(self.source)

    @property
    def remaining(self) -> int:
        """Number of characters remaining."""
        return len(self.source) - self.pos

    def peek(self, offset: int = 0) -> str:
        """Peek at the character at pos+offset without advancing.

        Returns empty string if out of bounds.
        """
        idx = self.pos + offset
        if idx < len(self.source):
            return self.source[idx]
        return ""

    def peek_slice(self, n: int) -> str:
        """Peek at n characters starting at pos without advancing."""
        return self.source[self.pos : self.pos + n]

    def advance(self) -> str:
        """Advance pos by one and return the consumed character.

        Returns empty string if already at end.
        """
        if self.pos < len(self.source):
            ch = self.source[self.pos]
            self.pos += 1
            return ch
        return ""

    def skip(self, n: int) -> None:
        """Advance pos by n characters, clamped to end of string."""
        self.pos = min(self.pos + n, len(self.source))

    def match(self, s: str) -> bool:
        """If the next characters exactly match `s`, advance past them and return True.

        Otherwise leave pos unchanged and return False.
        """
        if self.source.startswith(s, self.pos):
            self.pos += len(s)
            return True
        return False

    def match_regex(self, pattern: re.Pattern[str]) -> str | None:
        """If the regex matches at current pos, advance past the match and return the matched string.

        Otherwise return None and leave pos unchanged.

        The regex is applied with re.match (anchored at current pos via slicing).
        """
        m = pattern.match(self.source, self.pos)
        if m is None:
            return None
        self.pos = m.end()
        return m.group(0)

    def consume_while(self, pred: Callable[[str], bool]) -> str:
        """Consume characters while the predicate returns True.

        Returns the consumed string.
        """
        start = self.pos
        while self.pos < len(self.source) and pred(self.source[self.pos]):
            self.pos += 1
        return self.source[start : self.pos]

    def consume_line(self) -> str:
        """Consume the rest of the line (up to but not including the newline)."""
        start = self.pos
        while self.pos < len(self.source) and self.source[self.pos] != "\n":
            self.pos += 1
        return self.source[start : self.pos]

    def rest(self) -> str:
        """Return the rest of the input from current pos without advancing."""
        return self.source[self.pos :]

    def slice_from(self, start: int) -> str:
        """Return a slice of source from `start` to current pos."""
        return self.source[start : self.pos]

    def skip_spaces(self) -> int:
        """Skip ASCII spaces and tabs. Returns number of characters skipped."""
        start = self.pos
        while self.pos < len(self.source) and self.source[self.pos] in (" ", "\t"):
            self.pos += 1
        return self.pos - start

    def count_indent(self) -> int:
        """Count leading virtual spaces (expanding tabs to 4-column stops) without advancing."""
        indent = 0
        i = self.pos
        while i < len(self.source):
            ch = self.source[i]
            if ch == " ":
                indent += 1
                i += 1
            elif ch == "\t":
                indent += 4 - (indent % 4)
                i += 1
            else:
                break
        return indent


# ─── Type hints ───────────────────────────────────────────────────────────────

from collections.abc import Callable  # noqa: E402

# ─── Character Classification ─────────────────────────────────────────────────

# ASCII punctuation characters as defined by GFM.
# These are exactly: ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
ASCII_PUNCTUATION = frozenset(
    "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
)


def is_ascii_punctuation(ch: str) -> bool:
    """True if `ch` is an ASCII punctuation character (GFM definition).

    Used in the emphasis rules to determine flanking delimiter runs and in
    backslash escape handling (only ASCII punctuation can be backslash-escaped).

    The GFM ASCII punctuation set is:
      ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \\ ] ^ _ ` { | } ~
    """
    return ch in ASCII_PUNCTUATION


def is_unicode_punctuation(ch: str) -> bool:
    """True if `ch` is a Unicode punctuation character for GFM flanking.

    GFM defines this (per the cmark reference implementation) as any
    ASCII punctuation character OR any character in Unicode categories:
      Pc, Pd, Pe, Pf, Pi, Po, Ps (punctuation) or Sm, Sc, Sk, So (symbols).

    The symbol categories (S*) are included because cmark treats them as
    punctuation for delimiter flanking (e.g. £ U+00A3 Sc, € U+20AC Sc).

    We use unicodedata.category() for Python's Unicode category classification,
    which matches what the GFM spec requires.
    """
    if not ch:
        return False
    if ch in ASCII_PUNCTUATION:
        return True
    cat = unicodedata.category(ch)
    # P* categories: Pc Pd Pe Pf Pi Po Ps (punctuation)
    # S* categories: Sc Sk Sm So (symbols)
    return cat[0] in ("P", "S")


def is_ascii_whitespace(ch: str) -> bool:
    """True if `ch` is ASCII whitespace.

    ASCII whitespace: space (U+0020), tab (U+0009),
    newline (U+000A), form feed (U+000C), carriage return (U+000D).
    """
    return ch in (" ", "\t", "\n", "\r", "\f")


def is_unicode_whitespace(ch: str) -> bool:
    """True if `ch` is Unicode whitespace (any code point with Unicode property White_Space=yes).

    We check the standard whitespace categories as defined by GFM's
    reference implementation. Python's str.isspace() covers most but not all;
    we add NBSP and a few other characters to be precise.
    """
    if not ch:
        return False
    # Python's str.isspace() covers \\t \\n \\r \\f \\v \\x1c-\\x1f \\x85 \\xa0 and other Unicode spaces
    # The additional characters cmark checks are covered by this
    return ch.isspace() or ch in (
        "\u00A0",  # NO-BREAK SPACE
        "\u1680",  # OGHAM SPACE MARK
        "\u2000", "\u2001", "\u2002", "\u2003", "\u2004", "\u2005",
        "\u2006", "\u2007", "\u2008", "\u2009", "\u200A",  # EN QUAD through HAIR SPACE
        "\u202F",  # NARROW NO-BREAK SPACE
        "\u205F",  # MEDIUM MATHEMATICAL SPACE
        "\u3000",  # IDEOGRAPHIC SPACE
    )


def normalize_link_label(label: str) -> str:
    """Normalize a link label per GFM.

    Steps:
      1. Strip leading and trailing whitespace
      2. Collapse internal whitespace runs to a single space
      3. Fold to lowercase

    Two labels are equivalent if their normalized forms are equal.

    === Case folding ===

    JavaScript's toLowerCase() and Python's lower() both miss the Unicode
    *full* case fold for ß (U+00DF) and ẞ (U+1E9E), which both fold to "ss"
    in Unicode Full Case Folding. We post-process these to match cmark's
    behaviour.
    """
    result = label.strip()
    result = re.sub(r"\s+", " ", result)
    result = result.lower()
    # Apply the ß → ss full case fold that lower() misses
    result = result.replace("ß", "ss")
    return result


def normalize_url(url: str) -> str:
    """Normalize a URL: percent-encode characters that should not appear unencoded in HTML href/src attributes.

    Percent-encodes characters outside the unreserved set (A-Z a-z 0-9 - _ . ~)
    and the reserved set (:/?#@!$&'()*+,;=), plus % itself if not already encoded.

    Already-encoded sequences (like %20) are passed through unchanged.
    """
    result = []
    i = 0
    while i < len(url):
        ch = url[i]
        # Characters safe in HTML href attributes
        if ch in _URL_SAFE or (ch == "%" and i + 2 < len(url) and _HEX_RE.match(url, i + 1)):
            result.append(ch)
        else:
            # Percent-encode the character using UTF-8 encoding
            encoded = ch.encode("utf-8")
            result.extend(f"%{b:02X}" for b in encoded)
        i += 1
    return "".join(result)


# Characters that are safe in HTML href attributes without encoding.
# This is the union of unreserved and reserved URI characters plus
# some additional characters that cmark preserves.
_URL_SAFE = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    "0123456789-._~:/?#@!$&'()*+,;=%"
)

_HEX_RE = re.compile(r"[0-9A-Fa-f]{2}")
