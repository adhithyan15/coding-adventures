-- document_ast_sanitizer / policy.lua
-- =====================================
--
-- Defines the SanitizationPolicy table and three named presets:
-- STRICT, RELAXED, and PASSTHROUGH.
--
-- A policy is a plain Lua table (analogous to a TypeScript interface value).
-- Because Lua has no type system, we document the expected keys and values
-- here. Callers can create custom policies by copying a preset and overriding
-- individual fields using Lua table literal syntax:
--
--   local my_policy = {
--     allowRawBlockFormats  = "drop-all",
--     allowedUrlSchemes     = { "https" },
--     minHeadingLevel       = 2,
--     -- all other fields from STRICT …
--   }
--
-- === Policy Field Reference ===
--
-- allowRawBlockFormats   "drop-all" | "passthrough" | { string... }
--   Controls which raw_block nodes are kept.
--   "drop-all"    — every raw_block is dropped regardless of format.
--   "passthrough" — every raw_block is kept regardless of format.
--   table         — allowlist: keep only blocks whose format is in the table.
--
-- allowRawInlineFormats  same semantics, for raw_inline nodes.
--
-- allowedUrlSchemes      { string... } | false
--   Allowlist of URL schemes for link, image, and autolink nodes.
--   false         — any scheme is allowed (passthrough).
--   table         — only listed schemes (lowercased) are permitted.
--                   Relative URLs (no colon before first / or ?) always pass.
--
-- dropLinks              boolean  — true: promote link children to parent.
-- dropImages             boolean  — true: drop image nodes entirely.
-- transformImageToText   boolean  — true: replace image with alt TextNode.
-- maxHeadingLevel        1-6 | "drop"
-- minHeadingLevel        1-6
-- dropBlockquotes        boolean
-- dropCodeBlocks         boolean
-- transformCodeSpanToText boolean
--
-- @module coding_adventures.document_ast_sanitizer.policy

local M = {}

-- ─── Named Presets ────────────────────────────────────────────────────────────

--- STRICT — for user-generated content (comments, forum posts, chat messages).
--
-- Drops all raw HTML/format passthrough. Allows only http, https, mailto URLs.
-- Images are converted to alt text. Links are kept but URL-sanitized.
-- Headings are clamped to h2–h6 (h1 is reserved for page title).
M.STRICT = {
  allowRawBlockFormats    = "drop-all",
  allowRawInlineFormats   = "drop-all",
  allowedUrlSchemes       = { "http", "https", "mailto" },
  dropImages              = false,
  transformImageToText    = true,
  minHeadingLevel         = 2,
  maxHeadingLevel         = 6,
  dropLinks               = false,
  dropBlockquotes         = false,
  dropCodeBlocks          = false,
  transformCodeSpanToText = false,
}

--- RELAXED — for semi-trusted content (authenticated users, internal wikis).
--
-- Allows HTML raw blocks (but not other formats). Allows http, https, mailto,
-- ftp. Images pass through unchanged. Headings unrestricted.
M.RELAXED = {
  allowRawBlockFormats    = { "html" },
  allowRawInlineFormats   = { "html" },
  allowedUrlSchemes       = { "http", "https", "mailto", "ftp" },
  dropImages              = false,
  transformImageToText    = false,
  minHeadingLevel         = 1,
  maxHeadingLevel         = 6,
  dropLinks               = false,
  dropBlockquotes         = false,
  dropCodeBlocks          = false,
  transformCodeSpanToText = false,
}

--- PASSTHROUGH — for fully trusted content (documentation, static sites).
--
-- No sanitization. Everything passes through unchanged.
-- Equivalent to not calling sanitize() at all.
M.PASSTHROUGH = {
  allowRawBlockFormats    = "passthrough",
  allowRawInlineFormats   = "passthrough",
  allowedUrlSchemes       = false,   -- false = allow any scheme
  dropImages              = false,
  transformImageToText    = false,
  minHeadingLevel         = 1,
  maxHeadingLevel         = 6,
  dropLinks               = false,
  dropBlockquotes         = false,
  dropCodeBlocks          = false,
  transformCodeSpanToText = false,
}

--- Merge a partial policy table on top of PASSTHROUGH defaults.
--
-- This lets callers specify only the fields they care about:
--
--   local p = M.with_defaults({ dropLinks = true, minHeadingLevel = 2 })
--
-- @param overrides  table — partial policy fields
-- @return table     complete policy table
function M.with_defaults(overrides)
  local result = {}
  for k, v in pairs(M.PASSTHROUGH) do
    result[k] = v
  end
  if overrides then
    for k, v in pairs(overrides) do
      result[k] = v
    end
  end
  return result
end

return M
