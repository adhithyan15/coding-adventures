"""URL scheme extraction and control-character sanitization.

Browsers silently ignore certain "invisible" characters when parsing URLs.
Attackers exploit this to sneak schemes past naive scheme-checkers:

    java\x00script:alert(1)   → browser sees: javascript:alert(1)
    \u200bjavascript:alert(1) → browser sees: javascript:alert(1)
    java\rscript:alert(1)     → browser sees: javascript:alert(1)

This module strips those invisible characters BEFORE scheme extraction,
closing the bypass.

Characters stripped (from the URL string before scheme analysis):
  C0 control characters   U+0000–U+001F  (includes NUL, CR, LF, TAB)
  Zero-width space        U+200B
  Zero-width non-joiner   U+200C
  Zero-width joiner       U+200D
  Word joiner             U+2060
  BOM / zero-width no-break space  U+FEFF

These are the same characters stripped by the OWASP HTML Sanitizer and
the CommonMark spec's link destination parsing.

Spec: TE02 § "URL Scheme Sanitization"
"""

from __future__ import annotations

import re

# ─── Invisible-character strip pattern ────────────────────────────────────────
#
# We build a single compiled regex for efficiency. The character class includes:
#   \x00-\x1f  — C0 control characters (NUL through US)
#   \u200b     — zero-width space
#   \u200c     — zero-width non-joiner
#   \u200d     — zero-width joiner
#   \u2060     — word joiner
#   \ufeff     — BOM / zero-width no-break space
#
# Using re.compile() at module level amortises the compilation cost across all
# calls. The pattern is simple enough that the compile overhead is negligible.
_INVISIBLE_CHARS = re.compile(
    r"[\x00-\x1f\u200b\u200c\u200d\u2060\ufeff]"
)


def strip_control_chars(url: str) -> str:
    """Remove C0 control characters and zero-width Unicode from *url*.

    This is the FIRST step of URL sanitization — always call this before
    extracting or checking the URL scheme.

    Examples:
        >>> strip_control_chars("java\\x00script:alert(1)")
        'javascript:alert(1)'

        >>> strip_control_chars("\\u200bjavascript:alert(1)")
        'javascript:alert(1)'

        >>> strip_control_chars("https://example.com")
        'https://example.com'

        >>> strip_control_chars("relative/path")
        'relative/path'
    """
    return _INVISIBLE_CHARS.sub("", url)


def extract_scheme(url: str) -> str | None:
    """Extract the URL scheme from *url* (everything before the first ``:``) .

    Returns the scheme as a lowercase string, or ``None`` if the URL is
    relative (no scheme separator found, or separator appears after ``/`` or ``?``).

    A relative URL is one where:
      - There is no ``:`` at all.
      - The ``:`` appears after a ``/`` or ``?`` (path-relative, not a scheme).
        Example: ``/path?a=b:c`` — the ``:`` is in the query string, not a scheme.

    Examples:
        >>> extract_scheme("https://example.com")
        'https'

        >>> extract_scheme("JAVASCRIPT:alert(1)")
        'javascript'

        >>> extract_scheme("mailto:user@example.com")
        'mailto'

        >>> extract_scheme("relative/path")
        None

        >>> extract_scheme("/absolute/path")
        None

        >>> extract_scheme("../relative")
        None

        >>> extract_scheme("path?query=value:here")
        None
    """
    # Find the first colon in the URL.
    colon_index = url.find(":")
    if colon_index == -1:
        # No colon → definitely relative.
        return None

    # Check if the colon is preceded by a slash or question mark.
    # If so, the colon is in the path or query string, not in a scheme.
    #
    #   /absolute/path:with:colons   → slash before first colon → relative
    #   ?query=a:b                   → question mark → relative (unusual, but safe)
    #
    # We scan the URL up to the colon for / or ?.
    before_colon = url[:colon_index]
    if "/" in before_colon or "?" in before_colon:
        return None

    # Everything before the colon is the scheme. Lowercase for comparison.
    return before_colon.lower()


def is_scheme_allowed(url: str, allowed_schemes: tuple[str, ...] | None) -> bool:
    """Return True if *url* is safe under *allowed_schemes*.

    This is the complete URL safety check:
      1. Strip invisible/control characters.
      2. Extract scheme.
      3. If allowed_schemes is None → always safe (passthrough).
      4. If no scheme found → relative URL → always safe.
      5. Check extracted scheme against allowed_schemes (case-insensitive).

    Args:
        url:             The raw URL string from a link/image destination.
        allowed_schemes: Tuple of lowercase scheme strings (e.g. ("http","https")),
                         or None to allow any scheme.

    Returns:
        True  → URL is safe; keep the destination as-is.
        False → URL has a disallowed scheme; caller should set destination="" .

    Examples:
        >>> is_scheme_allowed("https://example.com", ("http", "https", "mailto"))
        True

        >>> is_scheme_allowed("javascript:alert(1)", ("http", "https", "mailto"))
        False

        >>> is_scheme_allowed("relative/path", ("http", "https"))
        True

        >>> is_scheme_allowed("data:text/html,<h1>x</h1>", ("http", "https"))
        False

        >>> is_scheme_allowed("HTTPS://example.com", ("http", "https"))
        True

        >>> is_scheme_allowed("any://scheme", None)
        True
    """
    # Step 1: strip control chars before scheme extraction.
    clean_url = strip_control_chars(url)

    # Step 2: allowed_schemes=None means "allow everything."
    if allowed_schemes is None:
        return True

    # Step 3: extract scheme.
    scheme = extract_scheme(clean_url)

    # Step 4: no scheme → relative URL → always allowed.
    if scheme is None:
        return True

    # Step 5: check against allowlist.
    return scheme in allowed_schemes
