# frozen_string_literal: true

# HTML Sanitization Policy
#
# Controls what the regex-based HTML sanitizer keeps, strips, or rewrites.
# Like the AST sanitizer policy, this is a plain value object (Data.define)
# so it can be derived with `with(field: new_value)`.
#
# == DOM adapter
#
# Ruby's stdlib has no DOM parser, so the domAdapter field in the TypeScript
# spec is omitted here. The sanitizer always uses regex-based string
# operations, matching the spec's "no DOM dependency" default mode.

module CodingAdventures
  module DocumentHtmlSanitizer
    # HtmlSanitizationPolicy — the tuning knobs for the HTML string sanitizer.
    #
    # All fields correspond directly to the TypeScript spec fields.
    HtmlSanitizationPolicy = Data.define(
      :drop_elements,
      :drop_attributes,
      :allowed_url_schemes,
      :drop_comments,
      :sanitize_style_attributes
    )

    # ─── Named Presets ─────────────────────────────────────────────────────────

    # HTML_STRICT — for untrusted HTML from external sources.
    #
    # Drops all active elements (script, iframe, form, meta, etc.).
    # Strips all event handler attributes (on*), srcdoc, and formaction.
    # Allows only http/https/mailto URLs in href/src.
    # Strips HTML comments (can hide injected payloads in older parsers).
    # Strips CSS expression() and url() with non-safe arguments.
    HTML_STRICT = HtmlSanitizationPolicy.new(
      drop_elements: %w[
        script style iframe object embed applet
        form input button select textarea
        noscript meta link base
      ],
      drop_attributes: [], # on* stripped by built-in logic
      allowed_url_schemes: %w[http https mailto],
      drop_comments: true,
      sanitize_style_attributes: true
    ).freeze

    # HTML_RELAXED — for semi-trusted HTML (authenticated users, internal tools).
    #
    # Drops obviously dangerous elements (script, iframe, etc.) but allows
    # style, form, and meta (useful in internal content).
    # Allows ftp:// in addition to http/https/mailto.
    # Comments are preserved (inline documentation, code review tools).
    HTML_RELAXED = HtmlSanitizationPolicy.new(
      drop_elements: %w[script iframe object embed applet],
      drop_attributes: [],
      allowed_url_schemes: %w[http https mailto ftp],
      drop_comments: false,
      sanitize_style_attributes: true
    ).freeze

    # HTML_PASSTHROUGH — no sanitization.
    #
    # Every element, attribute, and comment passes through unchanged.
    # Use only with fully trusted content (documentation, static sites).
    HTML_PASSTHROUGH = HtmlSanitizationPolicy.new(
      drop_elements: [],
      drop_attributes: [],
      allowed_url_schemes: nil,
      drop_comments: false,
      sanitize_style_attributes: false
    ).freeze
  end
end
