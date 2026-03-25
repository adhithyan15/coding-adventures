-- document_html_sanitizer / url_utils.lua
-- =========================================
--
-- URL scheme extraction and scheme-allowlist checking for the HTML
-- sanitizer. This is an independent copy — it shares no module-level
-- dependency with document_ast_sanitizer.
--
-- The logic is identical to the AST sanitizer's url_utils.lua so that
-- both packages enforce exactly the same URL scheme policy. Having separate
-- copies avoids creating a shared "url_utils" micro-package, which would
-- introduce a dependency graph complication for a tiny file.
--
-- === Bypass vectors we defend against ===
--
-- Browsers silently ignore invisible code points when parsing URLs. An
-- attacker can smuggle a dangerous scheme past naive pattern-matching:
--
--   java\x00script:alert(1)     — null byte stripped → javascript:
--   java\rscript:alert(1)        — CR stripped → javascript:
--   \u200bjavascript:alert(1)    — zero-width space stripped → javascript:
--
-- We strip all such characters before extracting the scheme, so we always
-- compare against the same string the browser will see.
--
-- @module coding_adventures.document_html_sanitizer.url_utils

local M = {}

--- Strip control characters and invisible Unicode code points from a URL.
--
-- @param url  string — raw URL value extracted from an HTML attribute
-- @return string — URL with dangerous invisible chars removed
function M.strip_control_chars(url)
  return url
    :gsub("[\000-\031\127]", "")   -- ASCII C0 controls (0x00-0x1F) + DEL
    :gsub("\226\128\139", "")      -- U+200B ZERO WIDTH SPACE      (E2 80 8B)
    :gsub("\226\128\140", "")      -- U+200C ZERO WIDTH NON-JOINER (E2 80 8C)
    :gsub("\226\128\141", "")      -- U+200D ZERO WIDTH JOINER     (E2 80 8D)
    :gsub("\226\129\160", "")      -- U+2060 WORD JOINER           (E2 81 A0)
    :gsub("\239\187\191", "")      -- U+FEFF BOM                   (EF BB BF)
end

--- Extract the URL scheme from a stripped URL string.
--
-- Returns nil for relative URLs (no scheme component found).
--
-- Rules:
--   * Scheme is everything before the first colon.
--   * If a slash or question mark appears before the first colon, the
--     colon is part of the path/query — URL is relative.
--   * Scheme is lowercased before return.
--
-- @param url  string — URL with control chars already stripped
-- @return string|nil — lowercase scheme (e.g. "https"), or nil if relative
function M.extract_scheme(url)
  local colon_pos = url:find(":")
  if not colon_pos then return nil end
  if colon_pos == 1 then return nil end   -- leading colon → relative

  local slash_pos    = url:find("/")
  local question_pos = url:find("?")

  if slash_pos and slash_pos < colon_pos then return nil end
  if question_pos and question_pos < colon_pos then return nil end

  return url:sub(1, colon_pos - 1):lower()
end

--- Return true if the URL is safe according to the given scheme allowlist.
--
-- @param url              string        — attribute value (href, src, etc.)
-- @param allowed_schemes  table | false
--   false → any scheme is allowed (passthrough policy)
--   table → allowlist of lowercase scheme strings
-- @return boolean — true if safe to pass through
function M.is_scheme_allowed(url, allowed_schemes)
  if allowed_schemes == false then return true end

  local stripped = M.strip_control_chars(url)
  local scheme   = M.extract_scheme(stripped)

  -- Relative URLs always pass
  if scheme == nil then return true end

  for _, allowed in ipairs(allowed_schemes) do
    if scheme == allowed:lower() then return true end
  end
  return false
end

return M
