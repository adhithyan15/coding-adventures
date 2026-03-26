"""HTML Sanitization policy types and named presets.

The HTML sanitizer operates on an **opaque HTML string** — it has no knowledge
of where the HTML came from (Markdown renderer, CMS API, rich-text editor paste).
The policy controls which elements are dropped, which attributes are stripped,
and which URL schemes are allowed in href/src attributes.

Unlike the AST sanitizer (which operates on structured data), the HTML sanitizer
uses pattern-based string operations. This makes it portable across all
environments (browser, Node.js, Deno, Go, Python, Ruby, Rust, Elixir, Lua)
without requiring a DOM parser.

Design choice: drop elements INCLUDING their inner content. This is the safe
default for dangerous elements like <script>, <iframe>, <style>. We cannot
simply remove the tag and keep the content because the content of those elements
is itself executable/injected code.

Spec: TE02 — Document Sanitization, section "Stage 2 — HtmlSanitizationPolicy"
"""

from __future__ import annotations

from dataclasses import dataclass

# ─── Default drop lists ────────────────────────────────────────────────────────
#
# These defaults represent the consensus on which HTML elements and attributes
# are inherently dangerous regardless of their content.

# Elements that are dropped including all their nested content.
# We use tuple (not list) for immutability.
DEFAULT_DROP_ELEMENTS: tuple[str, ...] = (
    "script",    # Direct JavaScript execution — the most dangerous element
    "style",     # CSS expression() attacks, data: URL exfiltration
    "iframe",    # Framing attacks, clickjacking
    "object",    # Plugin execution (Flash, Java applets)
    "embed",     # Same as <object> — plugin execution
    "applet",    # Java applet execution (legacy, but still processed by some parsers)
    "form",      # CSRF attacks, credential phishing via fake login forms
    "input",     # Data capture, autofill attacks
    "button",    # Can submit forms
    "select",    # Can submit forms
    "textarea",  # Data capture
    "noscript",  # Can be abused in certain parser contexts (IE conditional comments)
    "meta",      # Redirect via http-equiv="refresh", charset attacks
    "link",      # CSS import, DNS prefetch exfiltration
    "base",      # Base URL hijacking — breaks ALL relative links in the page
)

# Attributes that are always stripped from every element.
# on* attributes are handled separately by a pattern match.
DEFAULT_DROP_ATTRIBUTES: tuple[str, ...] = (
    "srcdoc",     # Inline HTML frame content — iframe srcdoc XSS
    "formaction", # Overrides the form's action URL — CSRF vector
)


@dataclass(frozen=True)
class HtmlSanitizationPolicy:
    """Policy controlling how the HTML sanitizer transforms an HTML string.

    All fields have sensible security defaults. For most use cases, you should
    start from HTML_STRICT or HTML_RELAXED and override specific fields.

    The sanitizer applies transformations in this order:
      1. Drop comments (if drop_comments=True)
      2. Drop dangerous elements including their content (drop_elements)
      3. Strip dangerous attributes (drop_attributes + all on* attributes)
      4. Sanitize href/src URL schemes (allowed_url_schemes)
      5. Strip dangerous style attributes (sanitize_style_attributes)

    Field details:

    drop_elements:
        Element names (lowercase) that are removed including all their content.
        Contrast with "strip tag but keep content" — for dangerous elements,
        the content itself is the attack payload, so we drop both.

        Example:
            <script>alert(1)</script>  →  (empty string, nothing left)
            <p>Safe text</p>           →  <p>Safe text</p>

    drop_attributes:
        Attribute names (lowercase) stripped from every element they appear on.
        Note: ALL on* attributes (onclick, onload, etc.) are ALWAYS stripped
        regardless of this list. This list is for named attributes like
        srcdoc and formaction.

    allowed_url_schemes:
        Allowlist of URL schemes for href and src attributes. URLs with schemes
        not in this list are replaced with "".
        None = allow any scheme (dangerous for untrusted content).

    drop_comments:
        Whether to strip HTML comments <!-- … -->.
        Default: True.
        Rationale: Comments can carry IE conditional markup with scripts,
        and <!-- --> syntax can be used to hide payloads.

    sanitize_style_attributes:
        Whether to strip style attributes containing expression() or url()
        with non-http/https arguments.
        Default: True.
        Rationale: CSS expression() is an IE-specific JavaScript execution
        mechanism. url() can point to javascript: URIs.
    """

    drop_elements: tuple[str, ...] = DEFAULT_DROP_ELEMENTS

    drop_attributes: tuple[str, ...] = DEFAULT_DROP_ATTRIBUTES

    allowed_url_schemes: tuple[str, ...] | None = ("http", "https", "mailto", "ftp")

    drop_comments: bool = True

    sanitize_style_attributes: bool = True


# ─── Named Presets ─────────────────────────────────────────────────────────────

HTML_STRICT: HtmlSanitizationPolicy = HtmlSanitizationPolicy(
    # HTML_STRICT — for untrusted HTML from external sources.
    #
    # Use this for user-submitted HTML, CMS content that may contain injections,
    # HTML pasted from rich-text editors, or any HTML from a source you do not
    # fully trust.
    #
    # This drops all dangerous elements (script, style, iframe, form, etc.),
    # strips all event handler attributes, restricts URL schemes, and strips
    # HTML comments.
    drop_elements=DEFAULT_DROP_ELEMENTS,
    drop_attributes=(),  # on* handled by default logic; no extra named attrs here
    allowed_url_schemes=("http", "https", "mailto"),
    drop_comments=True,
    sanitize_style_attributes=True,
)

HTML_RELAXED: HtmlSanitizationPolicy = HtmlSanitizationPolicy(
    # HTML_RELAXED — for authenticated users / internal tools.
    #
    # Allows style elements (needed for rich documents), but still drops
    # the most dangerous elements. Comments are preserved (useful for HTML
    # fragments with IE conditional comments in internal tools).
    drop_elements=("script", "iframe", "object", "embed", "applet"),
    drop_attributes=(),
    allowed_url_schemes=("http", "https", "mailto", "ftp"),
    drop_comments=False,
    sanitize_style_attributes=True,
)

HTML_PASSTHROUGH: HtmlSanitizationPolicy = HtmlSanitizationPolicy(
    # HTML_PASSTHROUGH — no sanitization.
    #
    # Use only for fully trusted HTML (e.g. from your own renderer, in a
    # controlled context where XSS is not a concern).
    # This is the identity transformation — output equals input.
    drop_elements=(),
    drop_attributes=(),
    allowed_url_schemes=None,
    drop_comments=False,
    sanitize_style_attributes=False,
)
