-- document_html_sanitizer / policy.lua
-- ======================================
--
-- Defines the HtmlSanitizationPolicy table and three named presets:
-- HTML_STRICT, HTML_RELAXED, and HTML_PASSTHROUGH.
--
-- This module has NO dependency on document_ast — it is a standalone
-- string-processing package. A policy controls how sanitize_html() strips
-- dangerous elements, attributes, URLs, and CSS from an HTML string.
--
-- === Policy Field Reference ===
--
-- drop_elements        { string... }
--   Lowercase HTML tag names whose entire content (open tag + content +
--   close tag) is removed from the output. This is a content-dropping
--   removal, not just a tag strip. Example: "script" removes
--   <script>…</script> entirely.
--
-- drop_attributes      { string... }
--   Lowercase attribute names stripped from every element. This list is
--   ADDITIVE to the built-in "on*" handler stripping, which is ALWAYS
--   applied when drop_attributes is present (even as an empty table).
--
-- allowed_url_schemes  { string... } | false
--   Allowlist of URL schemes for href and src attributes.
--   false → allow any scheme (passthrough).
--   table → only listed schemes pass; others are replaced with "".
--
-- drop_comments        boolean
--   When true, HTML comments (<!-- … -->) are removed.
--
-- sanitize_style_attributes  boolean
--   When true, any style attribute containing expression( or url( with a
--   non-http/https argument is stripped entirely.
--
-- @module coding_adventures.document_html_sanitizer.policy

local M = {}

--- HTML_STRICT — for untrusted HTML from external sources.
--
-- Drops all scripting elements, frames, form controls, and metadata.
-- Strips all event handlers (on*), srcdoc, and formaction attributes.
-- Allows only http, https, mailto URLs.
-- Strips HTML comments.
-- Strips CSS expressions.
M.HTML_STRICT = {
  drop_elements = {
    "script", "style", "iframe", "object", "embed", "applet",
    "form", "input", "button", "select", "textarea",
    "noscript", "meta", "link", "base",
  },
  drop_attributes = {},   -- on* attributes stripped by default sanitize logic
  allowed_url_schemes = { "http", "https", "mailto" },
  drop_comments = true,
  sanitize_style_attributes = true,
}

--- HTML_RELAXED — for authenticated users / internal tools.
--
-- Drops only scripting and frame elements. Allows comments.
-- Still strips CSS expressions.
M.HTML_RELAXED = {
  drop_elements = {
    "script", "iframe", "object", "embed", "applet",
  },
  drop_attributes = {},
  allowed_url_schemes = { "http", "https", "mailto", "ftp" },
  drop_comments = false,
  sanitize_style_attributes = true,
}

--- HTML_PASSTHROUGH — no sanitization.
--
-- Nothing is stripped. Useful for trusted content or testing.
-- HTML_PASSTHROUGH uses false for drop_attributes and allowed_url_schemes
-- to signal "skip all attribute sanitization". An empty table {} would still
-- trigger the on* stripping logic (empty table is truthy in Lua).
M.HTML_PASSTHROUGH = {
  drop_elements = {},
  drop_attributes = false,      -- false = skip all attribute sanitization
  allowed_url_schemes = false,  -- false = allow any scheme
  drop_comments = false,
  sanitize_style_attributes = false,
}

return M
