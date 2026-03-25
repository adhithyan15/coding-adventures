"""Document AST Sanitizer — policy-driven tree transformation.

This package provides a sanitize() function that walks a Document AST and
applies a caller-defined SanitizationPolicy to decide what to keep, drop,
or transform.

Pipeline position:

    parse(markdown)          → DocumentNode  (TE01 — CommonMark Parser)
           ↓
    sanitize(doc, STRICT)   → DocumentNode  (TE02 — this package)
           ↓
    to_html(doc)             → str           (TE00 — document-ast-to-html)

Usage:

    from coding_adventures_document_ast_sanitizer import sanitize, STRICT, RELAXED, PASSTHROUGH

    # Sanitize user-generated content (forum post, comment, chat message)
    safe = sanitize(parse(user_markdown), STRICT)
    html = to_html(safe)

    # Sanitize authenticated-user content (internal wiki, documentation)
    safe = sanitize(parse(editor_markdown), RELAXED)

    # Custom policy — RELAXED base, reserve h1 for page title
    from coding_adventures_document_ast_sanitizer import SanitizationPolicy
    my_policy = SanitizationPolicy(**{**RELAXED.__dict__, "min_heading_level": 2})
    safe = sanitize(parse(markdown), my_policy)

    # Fully trusted content — no sanitization (same as not calling sanitize)
    doc = sanitize(parse(trusted_markdown), PASSTHROUGH)

Spec: TE02 — Document Sanitization
"""

from coding_adventures_document_ast_sanitizer.policy import (
    PASSTHROUGH,
    RELAXED,
    STRICT,
    SanitizationPolicy,
)
from coding_adventures_document_ast_sanitizer.sanitizer import sanitize
from coding_adventures_document_ast_sanitizer.url_utils import (
    extract_scheme,
    is_scheme_allowed,
    strip_control_chars,
)

__all__ = [
    # Main API
    "sanitize",
    # Policy types and presets
    "SanitizationPolicy",
    "STRICT",
    "RELAXED",
    "PASSTHROUGH",
    # URL utilities (exported for testing and downstream use)
    "strip_control_chars",
    "extract_scheme",
    "is_scheme_allowed",
]
