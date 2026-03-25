"""Sanitization policy types and named presets.

A SanitizationPolicy is a plain data object — a frozen dataclass — that
controls which nodes in a Document AST pass through unchanged, which are
transformed, and which are dropped entirely.

Think of a policy as a set of knobs on a content-filtering pipeline:

    parse(markdown)          → raw DocumentNode (may contain javascript: links,
                               raw HTML blocks, h1 headings, etc.)
           ↓
    sanitize(doc, STRICT)   → safe DocumentNode suitable for public rendering
           ↓
    toHtml(doc)              → safe HTML string

Policies are composable via dict-merge/spread:

    my_policy = SanitizationPolicy(**{**RELAXED.__dict__, "min_heading_level": 2})

This makes it trivial to build custom policies on top of the named presets
without duplicating all the defaults.

Spec: TE02 — Document Sanitization, section "Stage 1 — SanitizationPolicy"
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class SanitizationPolicy:
    """Policy that controls what the AST sanitizer keeps, transforms, or drops.

    All fields have PASSTHROUGH defaults — omitting a field (or constructing
    a default instance) applies no restrictions. Use STRICT, RELAXED, or
    PASSTHROUGH as named starting points.

    Design note: frozen=True makes policies immutable and safely shareable.
    Sharing a policy object across multiple sanitize() calls is safe — no
    state is mutated during sanitization.

    Field summary (see individual docstrings for full semantics):
      allow_raw_block_formats   — which RawBlockNode formats survive
      allow_raw_inline_formats  — which RawInlineNode formats survive
      allowed_url_schemes       — URL schemes permitted in links/images/autolinks
      drop_links                — drop LinkNode, promote children to parent
      drop_images               — drop ImageNode entirely
      transform_image_to_text   — replace ImageNode with alt TextNode
      max_heading_level         — cap heading depth (or drop all headings)
      min_heading_level         — promote shallow headings upward
      drop_blockquotes          — drop BlockquoteNode (children NOT promoted)
      drop_code_blocks          — drop CodeBlockNode
      transform_code_span_to_text — replace CodeSpanNode with TextNode
    """

    # ─── Raw node handling ──────────────────────────────────────────────────

    allow_raw_block_formats: str | tuple[str, ...] = "passthrough"
    """Controls which RawBlockNode formats are allowed through.

    Truth table:
      "drop-all"    → every RawBlockNode dropped regardless of format
      "passthrough" → every RawBlockNode kept regardless of format
      ("html",...)  → kept only if node.format is in this tuple; others dropped

    Default: "passthrough" (no restriction)

    Recommendation for user-generated content: "drop-all"
    """

    allow_raw_inline_formats: str | tuple[str, ...] = "passthrough"
    """Controls which RawInlineNode formats are allowed through.

    Same semantics as allow_raw_block_formats, but for inline-level raw nodes.

    Default: "passthrough" (no restriction)
    """

    # ─── URL scheme policy ──────────────────────────────────────────────────

    allowed_url_schemes: tuple[str, ...] | None = ("http", "https", "mailto", "ftp")
    """Allowlist of URL schemes permitted in link/image/autolink destinations.

    The sanitizer strips C0 control characters and zero-width characters from
    the URL before extracting the scheme, defeating bypass attempts like
    `java\x00script:alert(1)`.

    Rules:
      None            → any scheme passes (including javascript:, data:, etc.)
      ("http",...)    → only listed schemes pass; others get destination=""
      Relative URLs (no scheme) always pass through.

    Default: ("http", "https", "mailto", "ftp") — safe for most use cases.
    """

    # ─── Node type policy ───────────────────────────────────────────────────

    drop_links: bool = False
    """If True, LinkNode instances are dropped but their children are promoted.

    Promotion means: a paragraph [TextNode("visit "), LinkNode("here")] becomes
    [TextNode("visit "), TextNode("here")] — the link wrapper disappears but
    the text is preserved.

    This is better than silently dropping link text, which would produce
    confusing output like "visit  for more information."
    """

    drop_images: bool = False
    """If True, ImageNode instances are dropped entirely (no alt text fallback).

    Note: drop_images takes precedence over transform_image_to_text.
    If both are True, the image is dropped (not converted to text).
    """

    transform_image_to_text: bool = False
    """If True, ImageNode instances become TextNode { value: node.alt }.

    This is the recommended setting for UGC pipelines: image alt text
    (which authors control) is preserved as readable plain text, but
    image loading (which can be used for tracking) is disabled.

    Precedence: only applies if drop_images is False.
    """

    max_heading_level: int | str = 6
    """Maximum heading depth allowed.

    Values:
      "drop" → all HeadingNode instances are dropped
      1–6    → headings with level > max_heading_level are clamped down

    Example: max_heading_level=3 converts h4/h5/h6 → h3.
    Default: 6 (no clamping)
    """

    min_heading_level: int = 1
    """Minimum heading depth allowed.

    Headings with level < min_heading_level are promoted (level raised) to
    min_heading_level.

    Example: min_heading_level=2 converts h1 → h2. This is the standard
    setting for user content where h1 is reserved for the page title.
    Default: 1 (no promotion)
    """

    drop_blockquotes: bool = False
    """If True, BlockquoteNode instances are dropped entirely.

    Unlike drop_links, children are NOT promoted — the entire blockquote
    subtree disappears.
    """

    drop_code_blocks: bool = False
    """If True, CodeBlockNode instances are dropped.

    Useful for pipelines where code blocks from untrusted authors should
    not appear in the rendered output.
    """

    transform_code_span_to_text: bool = False
    """If True, CodeSpanNode { value } → TextNode { value }.

    This removes the <code> wrapper from inline code, making the text
    indistinguishable from surrounding prose. Rarely useful in practice,
    but included for completeness.
    """


# ─── Named Presets ─────────────────────────────────────────────────────────────
#
# These three presets cover the most common use cases. They are frozen
# dataclass instances — safe to share as module-level constants.

STRICT: SanitizationPolicy = SanitizationPolicy(
    # STRICT — for user-generated content (comments, forum posts, chat messages).
    #
    # Risk model: the author is untrusted and may try to inject JavaScript,
    # phish credentials, or embed tracking pixels. We:
    #   1. Drop ALL raw HTML/format pass-through (no javascript: injection via
    #      raw blocks)
    #   2. Allow only safe URL schemes (no javascript:, data:, vbscript:)
    #   3. Convert images to alt text (no tracking pixels, no image embedding)
    #   4. Keep links but URL-sanitize them (destination="" if scheme bad)
    #   5. Clamp headings to h2–h6 (h1 is the page title — user can't steal it)
    allow_raw_block_formats="drop-all",
    allow_raw_inline_formats="drop-all",
    allowed_url_schemes=("http", "https", "mailto"),
    drop_images=False,
    transform_image_to_text=True,
    min_heading_level=2,
    max_heading_level=6,
    drop_links=False,
    drop_blockquotes=False,
    drop_code_blocks=False,
    transform_code_span_to_text=False,
)

RELAXED: SanitizationPolicy = SanitizationPolicy(
    # RELAXED — for semi-trusted content (authenticated users, internal wikis).
    #
    # Risk model: the author is authenticated and somewhat trusted, but not
    # fully (they may paste content from external sources). We:
    #   1. Allow HTML raw blocks (useful for internal docs) but not other formats
    #   2. Allow http/https/mailto/ftp URL schemes
    #   3. Keep images and links unchanged
    #   4. No heading restrictions
    allow_raw_block_formats=("html",),
    allow_raw_inline_formats=("html",),
    allowed_url_schemes=("http", "https", "mailto", "ftp"),
    drop_images=False,
    transform_image_to_text=False,
    min_heading_level=1,
    max_heading_level=6,
    drop_links=False,
    drop_blockquotes=False,
    drop_code_blocks=False,
    transform_code_span_to_text=False,
)

PASSTHROUGH: SanitizationPolicy = SanitizationPolicy(
    # PASSTHROUGH — for fully trusted content (documentation, static sites).
    #
    # Risk model: the author is fully trusted (e.g. a developer writing docs
    # that are committed to the repository). Sanitization is a no-op. The
    # result is semantically identical to calling toHtml(doc) directly.
    allow_raw_block_formats="passthrough",
    allow_raw_inline_formats="passthrough",
    allowed_url_schemes=None,
    drop_images=False,
    transform_image_to_text=False,
    min_heading_level=1,
    max_heading_level=6,
    drop_links=False,
    drop_blockquotes=False,
    drop_code_blocks=False,
    transform_code_span_to_text=False,
)
