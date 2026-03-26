"""Document HTML Sanitizer — regex-based HTML string sanitizer.

This package sanitizes an HTML string by removing dangerous elements and
attributes. It has NO dependency on the Document AST package — it is a pure
string-in, string-out transformation.

Pipeline position:

    parse(markdown)          → DocumentNode  (TE01 — CommonMark Parser)
           ↓
    sanitize(doc, STRICT)   → DocumentNode  (TE02 — document-ast-sanitizer)
           ↓
    to_html(doc)             → str           (TE00 — document-ast-to-html)
           ↓
    sanitize_html(html, .)  → str           (TE02 — this package, optional 2nd stage)

Or standalone for HTML from external sources:

    sanitize_html(cms_api_response, HTML_STRICT)  → safe str

Usage:

    from coding_adventures_document_html_sanitizer import (
        sanitize_html,
        HTML_STRICT,
        HTML_RELAXED,
        HTML_PASSTHROUGH,
    )

    # Sanitize untrusted HTML from a CMS or external API
    safe = sanitize_html(raw_html, HTML_STRICT)

    # Sanitize with a custom policy
    from coding_adventures_document_html_sanitizer import HtmlSanitizationPolicy
    my_policy = HtmlSanitizationPolicy(
        drop_elements=HTML_STRICT.drop_elements,
        allowed_url_schemes=("http", "https"),
        drop_comments=True,
        sanitize_style_attributes=True,
    )
    safe = sanitize_html(raw_html, my_policy)

Spec: TE02 — Document Sanitization
"""

from coding_adventures_document_html_sanitizer.html_sanitizer import sanitize_html
from coding_adventures_document_html_sanitizer.policy import (
    DEFAULT_DROP_ATTRIBUTES,
    DEFAULT_DROP_ELEMENTS,
    HTML_PASSTHROUGH,
    HTML_RELAXED,
    HTML_STRICT,
    HtmlSanitizationPolicy,
)
from coding_adventures_document_html_sanitizer.url_utils import (
    is_url_allowed,
    strip_control_chars,
)

__all__ = [
    # Main API
    "sanitize_html",
    # Policy types and presets
    "HtmlSanitizationPolicy",
    "HTML_STRICT",
    "HTML_RELAXED",
    "HTML_PASSTHROUGH",
    # Default constants
    "DEFAULT_DROP_ELEMENTS",
    "DEFAULT_DROP_ATTRIBUTES",
    # URL utilities
    "is_url_allowed",
    "strip_control_chars",
]
