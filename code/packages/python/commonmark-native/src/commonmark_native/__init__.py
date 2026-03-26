"""
commonmark_native -- Rust-backed CommonMark Markdown to HTML
=============================================================

A Python package backed by the Rust ``commonmark`` crate via our
zero-dependency python-bridge FFI. All Markdown parsing and HTML rendering
runs in Rust -- only the function call boundary crosses between Python and Rust.

Functions
---------

markdown_to_html(markdown: str) -> str
    Convert CommonMark Markdown to HTML. Raw HTML blocks are passed through
    unchanged. Use for trusted, author-controlled Markdown (docs, blog posts).

markdown_to_html_safe(markdown: str) -> str
    Convert CommonMark Markdown to HTML, stripping all raw HTML. Use for
    untrusted user-supplied Markdown (comments, forum posts, chat messages)
    to prevent XSS attacks.

Examples
--------

    >>> from commonmark_native import markdown_to_html, markdown_to_html_safe
    >>> markdown_to_html("# Hello\\n\\nWorld\\n")
    '<h1>Hello</h1>\\n<p>World</p>\\n'
    >>> markdown_to_html_safe("<script>alert(1)</script>\\n\\n**bold**\\n")
    '<p><strong>bold</strong></p>\\n'
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports markdown_to_html and
# markdown_to_html_safe via PyInit_commonmark_native.
from commonmark_native.commonmark_native import (  # type: ignore[import]
    markdown_to_html,
    markdown_to_html_safe,
)

__all__ = [
    "markdown_to_html",
    "markdown_to_html_safe",
]
