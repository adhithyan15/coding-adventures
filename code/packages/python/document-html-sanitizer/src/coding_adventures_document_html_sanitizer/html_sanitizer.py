"""Regex-based HTML string sanitizer.

This module implements sanitizeHtml() — a string-in, string-out transformation
that removes dangerous HTML without requiring a DOM parser.

## Why regex-based?

The HTML sanitizer is designed to be portable across all target languages
(Python, Go, Rust, Ruby, Elixir, Lua, JavaScript, TypeScript). None of those
environments have a guaranteed DOM parser. Regex-based sanitization is the
common denominator.

The trade-off is that regex HTML parsing is inherently imperfect — a
sufficiently malformed HTML string can fool the pattern matcher. For this
reason:
  1. We are conservative: when in doubt, the pattern drops content.
  2. For truly adversarial input, run BOTH the AST sanitizer (Stage 1)
     AND the HTML sanitizer (Stage 2) as belt-and-suspenders.

## Processing order

The sanitizer applies transformations in this order:
  1. Drop comments  (<!-- ... -->)
  2. Drop dangerous elements including their content
     (<script>...</script>, <style>...</style>, etc.)
  3. Strip dangerous attributes from all remaining elements
     (on*, srcdoc, formaction, plus any caller-specified names)
  4. Sanitize href and src URL values (check scheme)
  5. Strip dangerous style attributes (expression(), url(non-http))

The order matters: we drop comments FIRST so comment-hidden scripts don't
survive to step 2. We drop elements before stripping attributes so we don't
accidentally sanitize content that will be dropped anyway.

## Known limitations

- Nested elements of the same type in drop_elements may not be fully removed
  if the regex can't correctly identify matching close tags. For the named
  default drop elements (script, style, etc.) this is acceptable — authors
  should not have legitimate nested scripts.
- Very long attributes or deeply nested tags may cause performance issues
  with backtracking. The patterns use non-greedy matching to mitigate this.
- Does not handle HTML entities in attribute values (e.g.
  href="&#x6A;avascript:"). For adversarial entity-encoded payloads, use
  the DOM adapter or run after an HTML decoder.

Spec: TE02 — Document Sanitization, section "Stage 2"
"""

from __future__ import annotations

import re

from coding_adventures_document_html_sanitizer.policy import HtmlSanitizationPolicy
from coding_adventures_document_html_sanitizer.url_utils import is_url_allowed

# ─── Compiled regex patterns ──────────────────────────────────────────────────
#
# We compile all patterns at module load time. The patterns are:
#   - re.IGNORECASE because HTML tag names and attribute names are case-insensitive.
#   - re.DOTALL where content can span newlines (script blocks, comments).
#
# The patterns are intentionally conservative — they may match more than a
# spec-compliant parser would, which is the safe direction for a sanitizer.

# HTML comment: <!-- anything -->
# re.DOTALL makes . match newlines inside the comment.
_COMMENT_PATTERN = re.compile(r"<!--.*?-->", re.DOTALL)

# Attribute in a tag: captures name and value.
# Handles: attr="value", attr='value', attr=value, attr (boolean)
# We use a non-greedy .*? for the value to avoid crossing tag boundaries.
_ATTR_PATTERN = re.compile(
    r"""(?:
        (?P<name>[^\s=/<>]+)          # attribute name
        \s*=\s*                        # optional spaces around =
        (?:
            "(?P<dq_value>[^"]*)"     # double-quoted value
          | '(?P<sq_value>[^']*)'     # single-quoted value
          | (?P<uq_value>[^\s>]*)     # unquoted value
        )
      | (?P<bool_name>[^\s=/<>]+)     # boolean attribute (no value)
    )""",
    re.VERBOSE | re.IGNORECASE,
)

# Style attribute value that is dangerous:
# - Anything containing expression(...)  (IE CSS expression() — JavaScript)
# - url(...) with a non-http/https argument
_DANGEROUS_EXPRESSION = re.compile(r"expression\s*\(", re.IGNORECASE)

# Match url(...) in style values. Captures the URL argument.
_CSS_URL = re.compile(r"url\s*\(\s*['\"]?([^'\")\s]+)['\"]?\s*\)", re.IGNORECASE)


# ─── Public API ───────────────────────────────────────────────────────────────


def sanitize_html(html: str, policy: HtmlSanitizationPolicy) -> str:
    """Sanitize an HTML string by applying *policy*.

    Returns a new string with dangerous content removed. The input is not
    mutated (strings are immutable in Python).

    Processing steps (applied in order):
      1. Strip comments (if policy.drop_comments)
      2. Drop dangerous elements including their content
      3. Strip dangerous attributes from remaining tags
      4. Sanitize href/src URL schemes
      5. Strip dangerous style attribute values

    Args:
        html:   An HTML string. May contain unsafe content.
        policy: The HtmlSanitizationPolicy to apply.

    Returns:
        A sanitized HTML string.

    Example:
        >>> from coding_adventures_document_html_sanitizer import sanitize_html, HTML_STRICT
        >>> sanitize_html('<p>Safe</p><script>alert(1)</script>', HTML_STRICT)
        '<p>Safe</p>'
        >>> sanitize_html('<a href="javascript:alert(1)">click</a>', HTML_STRICT)
        '<a href="">click</a>'
    """
    result = html

    # Step 1: Drop HTML comments.
    if policy.drop_comments:
        result = _strip_comments(result)

    # Step 2: Drop dangerous elements (including their nested content).
    if policy.drop_elements:
        result = _drop_elements(result, policy.drop_elements)

    # Steps 3–5: Process all remaining tags to strip attributes.
    needs_attr_pass = (
        policy.drop_attributes
        or policy.allowed_url_schemes is not None
        or policy.sanitize_style_attributes
    )
    if needs_attr_pass:
        result = _sanitize_attributes(result, policy)

    return result


# ─── Step 1: Comment removal ──────────────────────────────────────────────────


def _strip_comments(html: str) -> str:
    """Remove all HTML comments from *html*.

    HTML comments (<!-- ... -->) can contain:
      - IE conditional comments: <!--[if IE]><script>...</script><![endif]-->
      - Hidden payloads: <!--<img src=x onerror=alert(1)>-->

    We use re.DOTALL so the pattern matches multi-line comments.
    """
    return _COMMENT_PATTERN.sub("", html)


# ─── Step 2: Element dropping ─────────────────────────────────────────────────


def _drop_elements(html: str, drop_elements: tuple[str, ...]) -> str:
    """Drop all elements in *drop_elements* along with their content.

    For each element name in the drop list, we build a pattern that matches:
      <tag_name ...>...</tag_name>   (opening tag, content, closing tag)
    OR:
      <tag_name ... />               (self-closing tag)

    The key design choice is to use re.DOTALL so the pattern matches content
    that spans newlines (e.g. multi-line scripts).

    We use non-greedy (.*?) to avoid consuming too much content, but in
    the worst case (e.g. two <script> blocks), the non-greedy match will
    still correctly match each one.

    For elements like <script> and <style>, we also need to handle the case
    where the opening tag has attributes:
      <script type="text/javascript">...</script>
      <script src="evil.js"></script>

    Pattern explanation:
      <           - opening bracket
      \\s*        - optional whitespace (some parsers accept < script>)
      tag         - the element name (case-insensitive)
      [^>]*       - any attributes (everything until the close of the opening tag)
      >           - close of opening tag
      .*?         - content (non-greedy, DOTALL)
      </          - start of closing tag
      \\s*        - optional whitespace
      tag         - element name again
      \\s*        - optional whitespace
      >           - close of closing tag
    """
    result = html
    for tag in drop_elements:
        # Pattern for paired tags: <script ...>...</script>
        # re.escape ensures tag names with special chars are handled (none in
        # practice, but defensive).
        escaped_tag = re.escape(tag)
        paired_pattern = re.compile(
            r"<\s*" + escaped_tag + r"(?:\s[^>]*)?>.*?</\s*" + escaped_tag + r"\s*>",
            re.IGNORECASE | re.DOTALL,
        )
        result = paired_pattern.sub("", result)

        # Also drop self-closing tags: <embed src="evil.swf" />
        self_closing_pattern = re.compile(
            r"<\s*" + escaped_tag + r"(?:\s[^>]*)?\s*/?>",
            re.IGNORECASE,
        )
        result = self_closing_pattern.sub("", result)

    return result


# ─── Steps 3–5: Attribute processing ─────────────────────────────────────────


def _sanitize_attributes(html: str, policy: HtmlSanitizationPolicy) -> str:
    """Walk every HTML tag in *html* and sanitize its attributes.

    We use re.sub() with a callback function. The callback receives each
    opening tag match and returns a sanitized replacement.

    The pattern matches:
      <tag_name attrs...>   — opening tags only (not closing tags or comments)

    We do NOT process closing tags (</p>) because they have no attributes.
    We do NOT process DOCTYPE or processing instructions.
    """
    # Match any opening HTML tag (not a closing tag): <tagname ...>
    # We capture:
    #   group 1 (tag_name): the element name
    #   group 2 (attrs):    everything between the tag name and >
    tag_pattern = re.compile(
        r"<([A-Za-z][A-Za-z0-9\-]*)(\s[^>]*)?>",
        re.IGNORECASE,
    )

    def process_tag(match: re.Match) -> str:
        tag_name = match.group(1)
        attrs_str = match.group(2) or ""
        safe_attrs = _sanitize_attrs_string(attrs_str, policy)
        return f"<{tag_name}{safe_attrs}>"

    return tag_pattern.sub(process_tag, html)


def _sanitize_attrs_string(attrs_str: str, policy: HtmlSanitizationPolicy) -> str:
    """Sanitize the attribute string from a single HTML tag.

    Parse each attribute, apply the policy, and reconstruct the safe attrs.

    Returns a string starting with a space (if any safe attrs remain) or
    empty string (if all attrs were stripped). This preserves correct
    whitespace in the output tag.

    Attribute sanitization rules:
      1. on* attributes → always dropped (event handlers)
      2. Named attributes in policy.drop_attributes → dropped
      3. href attribute → URL scheme checked
      4. src attribute  → URL scheme checked
      5. style attribute → expression() and dangerous url() stripped
    """
    if not attrs_str.strip():
        return ""

    safe_attrs: list[str] = []

    for m in _ATTR_PATTERN.finditer(attrs_str):
        # Extract attribute name (handling both valued and boolean attrs).
        name = m.group("name") or m.group("bool_name") or ""
        if not name:
            continue

        name_lower = name.lower()

        # Rule 1: Drop all on* event handler attributes.
        # onclick, onload, onerror, onfocus, onmouseover, etc.
        if name_lower.startswith("on"):
            continue

        # Rule 2: Drop named attributes from the drop list.
        if name_lower in policy.drop_attributes:
            continue

        # Extract value (None for boolean attributes).
        raw_value: str | None
        if m.group("dq_value") is not None:
            raw_value = m.group("dq_value")
            quote = '"'
        elif m.group("sq_value") is not None:
            raw_value = m.group("sq_value")
            quote = "'"
        elif m.group("uq_value") is not None:
            raw_value = m.group("uq_value")
            quote = '"'  # normalise to double-quotes
        else:
            # Boolean attribute (no value).
            safe_attrs.append(f" {name}")
            continue

        # Rule 3 & 4: Sanitize href and src URL schemes.
        url_attrs = ("href", "src")
        if name_lower in url_attrs and not is_url_allowed(raw_value, policy.allowed_url_schemes):
            raw_value = ""

        # Rule 5: Strip dangerous style attributes.
        is_dangerous_style = (
            name_lower == "style"
            and policy.sanitize_style_attributes
            and _is_style_dangerous(raw_value)
        )
        if is_dangerous_style:
            continue  # Drop the entire style attribute

        safe_attrs.append(f' {name}={quote}{raw_value}{quote}')

    return "".join(safe_attrs)


def _is_style_dangerous(style_value: str) -> bool:
    """Return True if *style_value* contains dangerous CSS.

    Dangerous patterns:
      expression(...)      — IE-specific JavaScript execution in CSS
      url(javascript:...)  — javascript: URI in a CSS url()
      url(data:...)        — data: URI in a CSS url() (may execute scripts)

    We check:
      1. Any occurrence of expression(   → always dangerous
      2. url(...) where the URL is not http/https → potentially dangerous

    Returns True (dangerous, drop the style attribute) or
            False (appears safe, keep the style attribute).
    """
    # Check for CSS expression() — always dangerous.
    if _DANGEROUS_EXPRESSION.search(style_value):
        return True

    # Check for url() with non-http/https arguments.
    for url_match in _CSS_URL.finditer(style_value):
        url_arg = url_match.group(1)
        # We only allow http:// and https:// inside url() in style attrs.
        # Note: this is stricter than the href/src check — we don't allow
        # mailto: or ftp: inside CSS url() because they have no valid use
        # in a style context (you don't set background: url(mailto:...)).
        if not is_url_allowed(url_arg, ("http", "https")):
            return True

    return False
