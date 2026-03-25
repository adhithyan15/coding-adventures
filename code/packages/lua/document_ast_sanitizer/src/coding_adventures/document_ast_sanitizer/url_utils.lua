-- document_ast_sanitizer / url_utils.lua
-- ========================================
--
-- URL scheme extraction and control-character sanitization for the AST
-- sanitizer. These functions implement the URL scheme policy defined in
-- the TE02 spec.
--
-- === Why control-char stripping matters ===
--
-- Browsers silently ignore C0 control characters (bytes 0x00–0x1F), DEL
-- (0x7F), and certain Unicode "invisible" code points when parsing URL
-- schemes. This creates bypass vectors:
--
--   java\x00script:alert(1)   — browser ignores \x00, sees "javascript:"
--   \u200bjavascript:alert(1) — browser ignores zero-width space
--
-- We strip ALL of these before extracting the scheme, so the comparison
-- is made against the clean string the browser will actually see.
--
-- === Relative URL detection ===
--
-- A URL is relative if it has no scheme component. The scheme is the part
-- before the first colon. But a colon after a slash or question mark is
-- NOT a scheme separator — it's a port number or path component:
--
--   "/path:here"       — relative (colon after slash)
--   "?q=foo:bar"       — relative (colon after question mark)
--   "mailto:foo@bar"   — absolute, scheme = "mailto"
--   "https://example"  — absolute, scheme = "https"
--   "JAVASCRIPT:alert" — absolute, scheme = "javascript" (lowercased)
--
-- @module coding_adventures.document_ast_sanitizer.url_utils

local M = {}

--- Strip control characters and invisible Unicode from a URL string.
--
-- Removes:
--   C0 controls     U+0000–U+001F  (bytes 0x00–0x1F)
--   DEL             U+007F         (byte 0x7F)
--   U+200B  ZERO WIDTH SPACE        (UTF-8: E2 80 8B)
--   U+200C  ZERO WIDTH NON-JOINER   (UTF-8: E2 80 8C)
--   U+200D  ZERO WIDTH JOINER       (UTF-8: E2 80 8D)
--   U+2060  WORD JOINER             (UTF-8: E2 81 A0)
--   U+FEFF  BOM / ZERO WIDTH NBSP   (UTF-8: EF BB BF)
--
-- @param url  string — raw URL as stored in the AST node
-- @return string — URL with dangerous invisible characters removed
function M.strip_control_chars(url)
  return url
    :gsub("[\000-\031\127]", "")   -- ASCII C0 controls + DEL
    :gsub("\226\128\139", "")      -- U+200B ZERO WIDTH SPACE
    :gsub("\226\128\140", "")      -- U+200C ZERO WIDTH NON-JOINER
    :gsub("\226\128\141", "")      -- U+200D ZERO WIDTH JOINER
    :gsub("\226\129\160", "")      -- U+2060 WORD JOINER
    :gsub("\239\187\191", "")      -- U+FEFF BOM
end

--- Extract the scheme from a (pre-stripped) URL string.
--
-- Returns nil if the URL is relative (no colon found, or colon appears
-- only after a slash or question mark).
--
-- Examples:
--   "https://example.com"  → "https"
--   "JAVASCRIPT:alert(1)"  → "javascript"  (lowercased)
--   "mailto:foo@bar"       → "mailto"
--   "/relative/path"       → nil
--   "?query=value"         → nil
--   "/path:with:colons"    → nil  (colon after slash)
--
-- @param url  string — URL with control characters already stripped
-- @return string|nil — lowercase scheme, or nil if URL is relative
function M.extract_scheme(url)
  -- Find the position of the first colon.
  local colon_pos = url:find(":")
  if not colon_pos then
    return nil   -- no colon at all → relative URL
  end

  -- A colon appearing after a slash or question mark is not a scheme
  -- separator. Check if any slash or question mark appears before the colon.
  local slash_pos    = url:find("/")
  local question_pos = url:find("?")

  if slash_pos and slash_pos < colon_pos then
    return nil   -- colon is in the path component → relative
  end
  if question_pos and question_pos < colon_pos then
    return nil   -- colon is in the query component → relative
  end

  -- Extract everything before the colon and lowercase it.
  -- An empty scheme (":" at position 1) is treated as relative.
  if colon_pos == 1 then
    return nil
  end

  return url:sub(1, colon_pos - 1):lower()
end

--- Check whether a URL's scheme is allowed by the given scheme list.
--
-- @param url              string        — the destination URL from the AST node
-- @param allowed_schemes  table | false
--   false  — all schemes are allowed (passthrough policy)
--   table  — allowlist of lowercase scheme strings (e.g. {"http","https"})
-- @return boolean — true if the URL may pass through, false if it must be blocked
function M.is_scheme_allowed(url, allowed_schemes)
  -- PASSTHROUGH policy: allow everything
  if allowed_schemes == false then
    return true
  end

  local stripped = M.strip_control_chars(url)
  local scheme   = M.extract_scheme(stripped)

  -- Relative URLs always pass through regardless of scheme policy.
  if scheme == nil then
    return true
  end

  -- Linear scan over the allowlist.
  -- In practice, the list has at most 4–5 entries, so this is fast.
  for _, allowed in ipairs(allowed_schemes) do
    if scheme == allowed:lower() then
      return true
    end
  end

  return false
end

return M
