# frozen_string_literal: true

# Sanitization Policy — the caller-controlled knob board
#
# A SanitizationPolicy is a plain Struct (immutable-by-convention) that
# controls what the AST sanitizer keeps, transforms, or drops. Every field
# has a safe default so callers can start from PASSTHROUGH and override only
# what they need:
#
#   custom = CodingAdventures::DocumentAstSanitizer::RELAXED.with(
#     min_heading_level: 2
#   )
#
# == Design: data object, not method chain
#
# Policies are plain data. This makes them:
#   - Composable: derive a new policy by overriding specific fields
#   - Serialisable: no procs or closures in the policy object
#   - Portable: mirrors the TypeScript/Python/Elixir implementations

module CodingAdventures
  module DocumentAstSanitizer
    # SanitizationPolicy captures every tuning knob for the AST sanitizer.
    #
    # Implemented as Ruby 3.2+ Data.define so that instances are frozen value
    # objects with structural equality AND support `with(field: new_value)`
    # for deriving customised policies.
    #
    #   policy = STRICT.with(drop_code_blocks: true)
    #
    # Field summary (see individual field comments for details):
    #
    #   allow_raw_block_formats    — which RawBlockNode formats survive
    #   allow_raw_inline_formats   — which RawInlineNode formats survive
    #   allowed_url_schemes        — URL scheme allowlist (nil = any)
    #   drop_links                 — promote link children, remove wrapper
    #   drop_images                — remove ImageNode entirely
    #   transform_image_to_text    — replace ImageNode with alt TextNode
    #   max_heading_level          — clamp or drop deep headings
    #   min_heading_level          — clamp shallow headings upward
    #   drop_blockquotes           — remove BlockquoteNode
    #   drop_code_blocks           — remove CodeBlockNode
    #   transform_code_span_to_text — replace CodeSpanNode with TextNode
    SanitizationPolicy = Data.define(
      :allow_raw_block_formats,
      :allow_raw_inline_formats,
      :allowed_url_schemes,
      :drop_links,
      :drop_images,
      :transform_image_to_text,
      :max_heading_level,
      :min_heading_level,
      :drop_blockquotes,
      :drop_code_blocks,
      :transform_code_span_to_text
    )

    # ─── Named Presets ─────────────────────────────────────────────────────────
    #
    # Three battle-tested starting points. Use them as-is, or derive a custom
    # policy via Struct#with:
    #
    #   STRICT.with(drop_code_blocks: true)

    # STRICT — for user-generated content (comments, forum posts, chat).
    #
    # Security philosophy:
    #   - Raw HTML is the top attack surface — drop all of it.
    #   - Only http/https/mailto URLs — no javascript:, data:, blob:.
    #   - Images are converted to alt text (prevents tracking pixels and
    #     content-injection via SVG data URIs).
    #   - h1 is reserved for the page title — clamp user headings to h2+.
    STRICT = SanitizationPolicy.new(
      allow_raw_block_formats: "drop-all",
      allow_raw_inline_formats: "drop-all",
      allowed_url_schemes: %w[http https mailto],
      drop_links: false,
      drop_images: false,
      transform_image_to_text: true,
      max_heading_level: 6,
      min_heading_level: 2,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    ).freeze

    # RELAXED — for semi-trusted content (authenticated users, internal wikis).
    #
    # Allows HTML raw blocks (useful for embedding widgets in internal docs).
    # Allows ftp:// in addition to http/https/mailto.
    # Images and headings unrestricted.
    RELAXED = SanitizationPolicy.new(
      allow_raw_block_formats: ["html"],
      allow_raw_inline_formats: ["html"],
      allowed_url_schemes: %w[http https mailto ftp],
      drop_links: false,
      drop_images: false,
      transform_image_to_text: false,
      max_heading_level: 6,
      min_heading_level: 1,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    ).freeze

    # PASSTHROUGH — for fully trusted content (documentation, static sites).
    #
    # No sanitization. Everything passes through unchanged.
    # Equivalent to not calling sanitize() at all.
    PASSTHROUGH = SanitizationPolicy.new(
      allow_raw_block_formats: "passthrough",
      allow_raw_inline_formats: "passthrough",
      allowed_url_schemes: nil,
      drop_links: false,
      drop_images: false,
      transform_image_to_text: false,
      max_heading_level: 6,
      min_heading_level: 1,
      drop_blockquotes: false,
      drop_code_blocks: false,
      transform_code_span_to_text: false
    ).freeze
  end
end
