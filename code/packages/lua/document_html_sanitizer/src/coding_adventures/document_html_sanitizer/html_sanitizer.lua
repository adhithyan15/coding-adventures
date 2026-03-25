-- document_html_sanitizer / html_sanitizer.lua
-- ==============================================
--
-- Pattern-based HTML string sanitizer. sanitize_html(html, policy) takes an
-- opaque HTML string and returns a new string with dangerous content removed.
--
-- === Why pattern-based and not DOM-based? ===
--
-- The spec (TE02 §Decision 5) mandates a pattern-based approach for
-- portability: Go, Python, Rust, Elixir, Lua, and edge JS runtimes have no
-- shared DOM API. Pattern matching gives us the same logic everywhere.
--
-- === Lua pattern limitations vs full regex ===
--
-- Lua patterns are NOT full regular expressions. Key differences:
--   * No alternation with |    — must use multiple gsub passes
--   * No non-greedy %*?        — string.find is used for greedy-safe bounds
--   * No case-insensitive flag — use string.lower() before matching
--   * Character classes: %a, %d, %w, %s, %p, %u, %l — NOT \d, \w, etc.
--   * Anchors: ^ and $ — same meaning as regex
--   * Magic chars: ( ) . % + - * ? [ ] ^ $
--
-- For case-insensitive element matching we lower-case the entire input or
-- use per-call lower() comparisons.
--
-- === Algorithm overview ===
--
-- The sanitizer performs these passes in order:
--
--   1. Drop comments        — strip <!-- … -->
--   2. Drop elements        — strip <tagname …>…</tagname> (content included)
--   3. Sanitize attributes  — per open-tag pass:
--        a. Strip on* event handler attributes
--        b. Strip explicit drop_attributes entries
--        c. Sanitize href/src URL attributes
--        d. Strip dangerous style attributes (expression(), url(non-http))
--   4. Return cleaned string
--
-- === Security note on element dropping ===
--
-- Element dropping (step 2) removes the open tag, ALL inner content, AND
-- the close tag. We do NOT just strip the tags and keep the text content
-- inside a <script> element. The content of <script>alert(1)</script> would
-- still be rendered as text by some browsers if we kept it.
--
-- The implementation uses a multi-pass gsub approach rather than a full
-- parser. This is intentionally simple and portable. Deeply nested or
-- malformed HTML may not be fully sanitized — for high-security use cases,
-- prefer the AST sanitizer pipeline (stage 1) which operates on structured
-- data.
--
-- @module coding_adventures.document_html_sanitizer.html_sanitizer

local url_utils = require("coding_adventures.document_html_sanitizer.url_utils")

local M = {}

-- ─── Step 1: Comment removal ──────────────────────────────────────────────────

--- Remove all HTML comments from the string.
--
-- Matches <!-- … --> where … can span any characters including newlines.
-- We use a Lua pattern with the non-greedy-like trick of matching up to
-- the first occurrence of -->.
--
-- Note: Lua patterns don't have non-greedy `*?`. We use a repeated
-- approach: find the comment start, then find the comment end, and
-- cut out the substring. This handles multiple comments correctly.
--
-- @param html  string — HTML input
-- @return string — HTML with comments removed
local function remove_comments(html)
  -- Find <!-- and --> pairs iteratively to avoid greedy over-matching.
  local result = {}
  local pos = 1
  while true do
    local s = html:find("<!%-%-", pos)
    if not s then
      result[#result + 1] = html:sub(pos)
      break
    end
    -- Keep content before the comment
    result[#result + 1] = html:sub(pos, s - 1)
    -- Find the end of the comment
    local e = html:find("%-%->", s + 4)
    if not e then
      -- Unclosed comment — drop everything from here to end
      break
    end
    -- Skip past the closing -->
    pos = e + 3
  end
  return table.concat(result)
end

-- ─── Step 2: Element dropping ─────────────────────────────────────────────────

--- Build a set (table with boolean values) from a list for O(1) lookups.
local function make_set(list)
  local set = {}
  for _, v in ipairs(list) do
    set[v:lower()] = true
  end
  return set
end

--- Remove all occurrences of a specific element (open tag + content + close tag).
--
-- Handles three cases:
--   1. <tagname …>…</tagname>  — block element with a close tag
--   2. <tagname …/>            — self-closing element
--   3. <tagname …>             — open tag with no close tag (drop only the tag)
--
-- Case-insensitive: we lower-case the HTML for matching but keep the
-- iteration position in terms of the original string.
--
-- @param html     string — HTML input
-- @param tagname  string — lowercase tag name to drop
-- @return string — HTML with all instances of that element removed
local function drop_element(html, tagname)
  -- We process the string in a loop, finding each open tag, then looking
  -- for the matching close tag.
  local result = {}
  local pos    = 1
  local lower  = html:lower()

  -- The open-tag pattern: <tagname followed by space, /, or >
  -- We use find() with the lowercased version, then apply positions to the original.
  local open_pat  = "<" .. tagname .. "([%s/>])"
  local close_pat = "</" .. tagname .. "%s*>"

  while true do
    local s, e, _ = lower:find(open_pat, pos)
    if not s then
      result[#result + 1] = html:sub(pos)
      break
    end

    -- Keep everything before the open tag
    result[#result + 1] = html:sub(pos, s - 1)

    -- Find the end of the open tag (the closing >)
    local open_end = lower:find(">", e)
    if not open_end then
      -- Malformed tag, no >, skip to end
      break
    end

    -- Check if it's self-closing (<tagname … />)
    local self_closing = html:sub(open_end - 1, open_end - 1) == "/"

    if self_closing then
      -- Self-closing: just skip the tag, no content to drop
      pos = open_end + 1
    else
      -- Look for the matching close tag </tagname>
      local cs = lower:find(close_pat, open_end + 1)
      if cs then
        -- Drop everything from open tag start to end of close tag
        local ce = lower:find(">", cs)
        pos = ce + 1
      else
        -- No close tag: just skip the open tag
        pos = open_end + 1
      end
    end
  end

  return table.concat(result)
end

--- Drop all elements listed in the policy's drop_elements list.
--
-- @param html    string — HTML input
-- @param policy  table — HtmlSanitizationPolicy
-- @return string — HTML with listed elements removed
local function drop_elements(html, policy)
  local elements = policy.drop_elements or {}
  for _, tagname in ipairs(elements) do
    html = drop_element(html, tagname:lower())
  end
  return html
end

-- ─── Step 3: Attribute sanitization ──────────────────────────────────────────
--
-- Rather than trying to parse the full HTML tag grammar, we use a targeted
-- approach: for each open tag found, we apply attribute transformations to
-- the tag's attribute string only.
--
-- This approach is not perfectly robust against all malformed HTML but
-- handles the common cases defined in the spec's test categories.

--- Remove a specific attribute (by name) from a tag's attribute string.
--
-- The attribute pattern covers:
--   name="value"
--   name='value'
--   name=value
--   name          (boolean attribute, no value)
--
-- All values are matched case-insensitively by lowercasing the attribute
-- string before matching, then applying the position to the original.
--
-- @param attrs    string — the attribute portion of a tag (everything between
--                          the tag name and the closing >)
-- @param attr_name string — lowercase attribute name to remove
-- @return string — attrs with all occurrences of that attribute removed
local function remove_attr(attrs, attr_name)
  -- Pattern to match:  attr_name = "value"  or  attr_name = 'value'
  --                    attr_name = value     or  attr_name (boolean)
  -- We use multiple passes for robustness.

  -- Remove: attr_name="value" or attr_name='value'
  -- %s* before and around = to handle whitespace
  attrs = attrs:gsub("[%s]*" .. attr_name .. "[%s]*=[%s]*\"[^\"]*\"", "")
  attrs = attrs:gsub("[%s]*" .. attr_name .. "[%s]*=[%s]*'[^']*'", "")
  -- Remove: attr_name=value (unquoted)
  attrs = attrs:gsub("[%s]*" .. attr_name .. "[%s]*=[%s]*[^%s>]*", "")
  -- Remove: attr_name (boolean, no value)
  attrs = attrs:gsub("[%s]+" .. attr_name .. "([%s/>])", function(suffix)
    return suffix
  end)
  -- Also handle attr at start of attrs string
  attrs = attrs:gsub("^" .. attr_name .. "([%s/>])", function(suffix)
    return suffix
  end)
  return attrs
end

--- Remove all event handler attributes (on*) from a tag attribute string.
--
-- Event handlers start with "on" followed by one or more word characters:
-- onclick, onload, onerror, onmouseover, etc.
--
-- We do this by scanning for " on" patterns and removing the full attr.
--
-- @param attrs  string — attribute portion of a tag
-- @return string — attrs with all on* attributes removed
local function remove_event_handlers(attrs)
  -- Match: (space)on<word>=... with quoted value
  attrs = attrs:gsub("[%s]+on%a+[%s]*=[%s]*\"[^\"]*\"", "")
  attrs = attrs:gsub("[%s]+on%a+[%s]*=[%s]*'[^']*'", "")
  -- Unquoted value
  attrs = attrs:gsub("[%s]+on%a+[%s]*=[%s]*[^%s>]*", "")
  -- Boolean form (e.g. onload without value — rare but possible)
  attrs = attrs:gsub("[%s]+on%a+([%s>])", function(suffix) return suffix end)
  -- Handle on* at the very start of attrs (tag opens with event handler)
  attrs = attrs:gsub("^on%a+[%s]*=[%s]*\"[^\"]*\"", "")
  attrs = attrs:gsub("^on%a+[%s]*=[%s]*'[^']*'", "")
  attrs = attrs:gsub("^on%a+[%s]*=[%s]*[^%s>]*", "")
  return attrs
end

--- Sanitize the value of href or src attributes.
--
-- If the scheme is not allowed, the attribute value is replaced with "".
--
-- @param attrs           string        — attribute portion of a tag
-- @param allowed_schemes table | false — scheme allowlist
-- @return string — attrs with href/src values sanitized
local function sanitize_url_attrs(attrs, allowed_schemes)
  -- Process href and src attributes.
  for _, attr_name in ipairs({ "href", "src" }) do
    -- Quoted double: attr="value"
    attrs = attrs:gsub(
      "(" .. attr_name .. "%s*=%s*\")([^\"]*)(\")",
      function(prefix, url_val, suffix)
        if not url_utils.is_scheme_allowed(url_val, allowed_schemes) then
          return prefix .. "" .. suffix
        end
        return prefix .. url_val .. suffix
      end
    )
    -- Quoted single: attr='value'
    attrs = attrs:gsub(
      "(" .. attr_name .. "%s*=%s*')([^']*)(')",
      function(prefix, url_val, suffix)
        if not url_utils.is_scheme_allowed(url_val, allowed_schemes) then
          return prefix .. "" .. suffix
        end
        return prefix .. url_val .. suffix
      end
    )
  end
  return attrs
end

--- Sanitize or strip a style attribute if it contains dangerous CSS.
--
-- The spec says: strip the ENTIRE style attribute if it contains:
--   expression(      — IE CSS expression() — executes JavaScript
--   url(             with a non-http/https argument
--
-- We check by lowercasing the style value and looking for these patterns.
--
-- @param attrs  string — attribute portion of a tag
-- @return string — attrs with dangerous style attributes removed
local function sanitize_style(attrs)
  -- Match style="value" and check if the value is dangerous.
  -- We need to find the style attribute value, check it, and maybe remove it.

  -- Double-quoted style
  attrs = attrs:gsub(
    "(style%s*=%s*\")([^\"]*)\"",
    function(prefix, style_val)
      local lower_val = style_val:lower()
      -- Dangerous patterns: expression( or url( not followed by http/https
      if lower_val:find("expression%(") then
        return ""   -- remove the entire style attribute
      end
      -- Check for url( with dangerous content
      if lower_val:find("url%(") then
        -- Find each url(...) call and check if it's http/https
        local safe = true
        for inner in lower_val:gmatch("url%(([^%)]+)%)") do
          local trimmed = inner:match("^%s*(.-)%s*$")
          -- Remove surrounding quotes from the URL value
          trimmed = trimmed:match('^"(.-)"$') or trimmed:match("^'(.-)'$") or trimmed
          if not url_utils.is_scheme_allowed(trimmed, { "http", "https" }) then
            safe = false
            break
          end
        end
        if not safe then
          return ""   -- strip the entire style attribute
        end
      end
      return prefix .. style_val .. "\""
    end
  )

  -- Single-quoted style
  attrs = attrs:gsub(
    "(style%s*=%s*')([^']*)'",
    function(prefix, style_val)
      local lower_val = style_val:lower()
      if lower_val:find("expression%(") then
        return ""
      end
      if lower_val:find("url%(") then
        local safe = true
        for inner in lower_val:gmatch("url%(([^%)]+)%)") do
          local trimmed = inner:match("^%s*(.-)%s*$")
          trimmed = trimmed:match('^"(.-)"$') or trimmed:match("^'(.-)'$") or trimmed
          if not url_utils.is_scheme_allowed(trimmed, { "http", "https" }) then
            safe = false
            break
          end
        end
        if not safe then return "" end
      end
      return prefix .. style_val .. "'"
    end
  )

  return attrs
end

--- Apply all attribute sanitization rules to a single open tag.
--
-- @param tag     string — the full open tag, e.g. <a href="..." onclick="...">
-- @param policy  table  — HtmlSanitizationPolicy
-- @return string — sanitized open tag
local function sanitize_open_tag(tag, policy)
  -- Extract the tag name and attribute string.
  -- Pattern: < tagname ( attrs ) >
  -- The tag name is one or more word chars; attrs are everything else.
  local tagname, attrs = tag:match("^<(%w+)(.*)")
  if not tagname then
    return tag   -- not a well-formed open tag, pass through unchanged
  end

  -- Lowercase the attrs for pattern matching, but we need to operate on
  -- the original for output. We do the removals directly on `attrs` since
  -- our patterns match the lowercase forms anyway.
  local lower_attrs = attrs:lower()
  -- We'll operate on attrs directly, applying case-insensitive removals
  -- by using lower() comparisons where needed.

  -- ── (a) Strip on* event handler attributes ─────────────────────────
  -- We always strip on* when any attribute policy is in effect.
  local drop_attrs = policy.drop_attributes
  if drop_attrs then
    attrs = remove_event_handlers(attrs)
    lower_attrs = attrs:lower()

    -- ── (b) Strip explicit drop_attributes entries ──────────────────────
    for _, attr_name in ipairs(drop_attrs) do
      attrs = remove_attr(lower_attrs, attr_name:lower())
      lower_attrs = attrs:lower()
    end

    -- Always strip srcdoc and formaction (spec requirement)
    attrs = remove_attr(attrs:lower(), "srcdoc")
    attrs = remove_attr(attrs:lower(), "formaction")
    lower_attrs = attrs:lower()
  end

  -- ── (c) Sanitize href and src URL values ────────────────────────────
  local allowed_schemes = policy.allowed_url_schemes
  if allowed_schemes ~= false then
    attrs = sanitize_url_attrs(lower_attrs, allowed_schemes)
    lower_attrs = attrs:lower()
  end

  -- ── (d) Sanitize style attributes ───────────────────────────────────
  if policy.sanitize_style_attributes then
    attrs = sanitize_style(lower_attrs)
  end

  return "<" .. tagname .. attrs
end

--- Walk through the HTML string and apply attribute sanitization to each open tag.
--
-- We iterate over every < … > region that looks like an open tag (not a
-- close tag, not a comment) and call sanitize_open_tag on it.
--
-- @param html    string — HTML input (after element dropping)
-- @param policy  table  — HtmlSanitizationPolicy
-- @return string — HTML with all open tags sanitized
local function sanitize_attributes(html, policy)
  -- Find each < … > that is an open tag (starts with <letter)
  -- We use string.gsub with a function to process each tag.
  return html:gsub(
    "<(%w[^>]-)>",     -- match <tagname...attrs...>
    function(inner)
      local rebuilt = sanitize_open_tag("<" .. inner, policy)
      -- Ensure the result ends with >
      if rebuilt:sub(-1) ~= ">" then
        rebuilt = rebuilt .. ">"
      end
      return rebuilt
    end
  )
end

-- ─── Public Entry Point ───────────────────────────────────────────────────────

--- Sanitize an HTML string by stripping dangerous elements and attributes.
--
-- Performs a multi-pass string transformation:
--   1. Strip HTML comments (if policy.drop_comments)
--   2. Drop dangerous elements + their content (script, iframe, etc.)
--   3. Strip on* attributes, srcdoc, formaction
--   4. Sanitize href/src URL values
--   5. Strip dangerous style attributes
--
-- This is a best-effort string-based sanitizer. It handles the common XSS
-- vectors defined in TE02 §Testing Strategy. For adversarial inputs with
-- deliberately malformed HTML, the AST sanitizer pipeline (document_ast_sanitizer)
-- is more reliable because it operates on structured data.
--
-- @param html    string — HTML input (may be nil, returns "")
-- @param policy  table  — HtmlSanitizationPolicy table
-- @return string — sanitized HTML string
function M.sanitize_html(html, policy)
  if not html or html == "" then return "" end
  policy = policy or {}

  -- Step 1: Remove HTML comments
  if policy.drop_comments then
    html = remove_comments(html)
  end

  -- Step 2: Drop dangerous elements (open + content + close)
  html = drop_elements(html, policy)

  -- Step 3–5: Sanitize attributes in all remaining open tags
  html = sanitize_attributes(html, policy)

  return html
end

return M
