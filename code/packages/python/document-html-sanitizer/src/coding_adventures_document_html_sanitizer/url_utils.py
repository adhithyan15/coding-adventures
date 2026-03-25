"""URL scheme extraction and safety check for the HTML sanitizer.

This is an independent copy of the URL utilities from the AST sanitizer.
The HTML sanitizer has no dependency on the AST package, so we cannot share
code between them. Both packages must be independently deployable.

The logic is identical to coding_adventures_document_ast_sanitizer.url_utils:
  1. Strip C0 control chars and zero-width Unicode from the URL.
  2. Extract the scheme (everything before the first ':').
  3. Check the scheme against the allowed list.

See that module's docstring for the full rationale.

Spec: TE02 — Document Sanitization, section "URL Scheme Sanitization"
"""

from __future__ import annotations

import re

# Strip C0 control characters and zero-width Unicode before scheme extraction.
_INVISIBLE_CHARS = re.compile(
    r"[\x00-\x1f\u200b\u200c\u200d\u2060\ufeff]"
)


def strip_control_chars(url: str) -> str:
    """Remove invisible characters from *url* before scheme extraction."""
    return _INVISIBLE_CHARS.sub("", url)


def is_url_allowed(url: str, allowed_schemes: tuple[str, ...] | None) -> bool:
    """Return True if *url* is safe under *allowed_schemes*.

    Args:
        url:             The raw URL string from an href or src attribute.
        allowed_schemes: Tuple of lowercase scheme strings, or None for any.

    Returns:
        True  → URL is safe; keep the attribute value.
        False → URL has a disallowed scheme; caller should replace with "".
    """
    # Step 1: strip invisible characters.
    clean = strip_control_chars(url)

    # Step 2: None = allow any scheme.
    if allowed_schemes is None:
        return True

    # Step 3: find the scheme separator.
    colon_idx = clean.find(":")
    if colon_idx == -1:
        # No colon → relative URL → always safe.
        return True

    # Step 4: colon after / or ? → in path/query, not a scheme.
    before = clean[:colon_idx]
    if "/" in before or "?" in before:
        return True

    # Step 5: extract and check scheme.
    scheme = before.lower()
    return scheme in allowed_schemes
