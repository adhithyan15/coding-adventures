-- document_ast_sanitizer / sanitizer.lua
-- ========================================
--
-- Core tree-transformation logic. sanitize(doc, policy) performs a single
-- recursive descent of the Document AST and returns a freshly constructed
-- tree with all policy violations removed or neutralised.
--
-- === Design: pure and immutable ===
--
-- Every node in the output is a brand-new Lua table — we never modify the
-- input nodes in place. Callers can safely pass the same document through
-- multiple sanitizers with different policies:
--
--   local strict_doc  = sanitize(doc, STRICT)
--   local relaxed_doc = sanitize(doc, RELAXED)
--   -- doc is unchanged; strict_doc and relaxed_doc are independent copies
--
-- === Design: complete dispatch ===
--
-- Every node type in the Document AST is handled explicitly by an if/elseif
-- chain. There is no catch-all "keep unknown nodes" path. If new node types
-- are added to the AST, this file must be updated to handle them.
--
-- === Design: empty-child pruning ===
--
-- After recursing into a container node's children, if the resulting
-- children list is empty, the container node is dropped from the output
-- (preventing empty <p></p> tags). The sole exception is DocumentNode —
-- an empty document is valid and is never dropped.
--
-- === Truth table for each node type ===
--
-- Node type          Condition                            Action
-- ──────────────────────────────────────────────────────────────────────────
-- document           always                               recurse into children
-- heading            maxHeadingLevel == "drop"            drop node
-- heading            level < minHeadingLevel              clamp level up
-- heading            level > maxHeadingLevel              clamp level down
-- heading            otherwise                            recurse into children
-- paragraph          always                               recurse into children
-- code_block         dropCodeBlocks == true               drop node
-- code_block         otherwise                            keep as-is (leaf)
-- blockquote         dropBlockquotes == true              drop node
-- blockquote         otherwise                            recurse into children
-- list               always                               recurse into children
-- list_item          always                               recurse into children
-- thematic_break     always                               keep as-is (leaf)
-- raw_block          allowRawBlockFormats="drop-all"      drop node
-- raw_block          allowRawBlockFormats="passthrough"   keep as-is
-- raw_block          allowRawBlockFormats=[…]             keep if in list, else drop
--
-- text               always                               keep as-is
-- emphasis           always                               recurse into children
-- strong             always                               recurse into children
-- code_span          transformCodeSpanToText == true      convert to text node
-- code_span          otherwise                            keep as-is
-- link               dropLinks == true                    promote children
-- link               URL scheme not allowed               keep, set destination=""
-- link               otherwise                            sanitize URL, recurse
-- image              dropImages == true                   drop node
-- image              transformImageToText == true         TextNode { value=alt }
-- image              URL scheme not allowed               keep, set destination=""
-- image              otherwise                            sanitize URL, keep as-is
-- autolink           URL scheme not allowed               drop node
-- autolink           otherwise                            sanitize URL, keep as-is
-- raw_inline         allowRawInlineFormats="drop-all"     drop node
-- raw_inline         allowRawInlineFormats="passthrough"  keep as-is
-- raw_inline         allowRawInlineFormats=[…]            keep if in list, else drop
-- hard_break         always                               keep as-is
-- soft_break         always                               keep as-is
--
-- @module coding_adventures.document_ast_sanitizer.sanitizer

local url_utils = require("coding_adventures.document_ast_sanitizer.url_utils")

local M = {}

-- ─── Helpers ──────────────────────────────────────────────────────────────────

--- Check whether a raw_block or raw_inline format passes the format policy.
--
-- @param node_format  string         — format tag from the node (e.g. "html")
-- @param policy_field string | table — "drop-all", "passthrough", or allowlist
-- @return boolean — true if this node should be kept
local function format_allowed(node_format, policy_field)
  if policy_field == "drop-all" then
    return false
  end
  if policy_field == "passthrough" then
    return true
  end
  -- allowlist table: check membership
  for _, allowed in ipairs(policy_field) do
    if node_format == allowed then
      return true
    end
  end
  return false
end

-- Forward declarations for mutual recursion between block and inline handlers.
local sanitize_blocks
local sanitize_inlines
local sanitize_block
local sanitize_inline

-- ─── Inline Node Handlers ────────────────────────────────────────────────────

--- Sanitize a single inline node.
--
-- Returns a table or nil. nil means "drop this node entirely."
-- Returns a table (possibly a list when link children are promoted) of
-- zero or more nodes to splice into the parent.
--
-- Actually, to keep the API simple, the return convention is:
--   nil           — drop this node
--   single table  — a single replacement node
--   multiple      — handled by sanitize_inlines returning a flat list
--
-- For link child promotion (dropLinks=true), we return the children list
-- directly from sanitize_inlines and the caller splices them in.
-- We use a sentinel: returning the special marker table { __promoted = children }
-- so the caller knows to splice rather than wrap.

sanitize_inline = function(node, policy)
  local t = node.type

  -- ─── text — keep as-is ─────────────────────────────────────────────────
  if t == "text" then
    return { type = "text", value = node.value }

  -- ─── emphasis — recurse ────────────────────────────────────────────────
  elseif t == "emphasis" then
    local children = sanitize_inlines(node.children, policy)
    if #children == 0 then return nil end
    return { type = "emphasis", children = children }

  -- ─── strong — recurse ──────────────────────────────────────────────────
  elseif t == "strong" then
    local children = sanitize_inlines(node.children, policy)
    if #children == 0 then return nil end
    return { type = "strong", children = children }

  -- ─── code_span ─────────────────────────────────────────────────────────
  elseif t == "code_span" then
    if policy.transformCodeSpanToText then
      return { type = "text", value = node.value }
    end
    return { type = "code_span", value = node.value }

  -- ─── link ──────────────────────────────────────────────────────────────
  elseif t == "link" then
    if policy.dropLinks then
      -- Promote children to parent. Return a sentinel table.
      local children = sanitize_inlines(node.children, policy)
      return { __promoted = children }
    end

    local dest = node.destination
    if not url_utils.is_scheme_allowed(dest, policy.allowedUrlSchemes) then
      dest = ""
    end

    local children = sanitize_inlines(node.children, policy)
    -- An empty-children link is still kept (the link may have no text)
    return { type = "link", destination = dest, title = node.title, children = children }

  -- ─── image ─────────────────────────────────────────────────────────────
  elseif t == "image" then
    -- dropImages takes precedence
    if policy.dropImages then
      return nil
    end
    if policy.transformImageToText then
      return { type = "text", value = node.alt or "" }
    end

    local dest = node.destination
    if not url_utils.is_scheme_allowed(dest, policy.allowedUrlSchemes) then
      dest = ""
    end
    return { type = "image", destination = dest, title = node.title, alt = node.alt or "" }

  -- ─── autolink ──────────────────────────────────────────────────────────
  elseif t == "autolink" then
    local dest = node.destination
    if not url_utils.is_scheme_allowed(dest, policy.allowedUrlSchemes) then
      -- Autolinks with a disallowed scheme are dropped entirely (no text to promote).
      return nil
    end
    return { type = "autolink", destination = dest, is_email = node.is_email }

  -- ─── raw_inline ────────────────────────────────────────────────────────
  elseif t == "raw_inline" then
    local field = policy.allowRawInlineFormats
    if field == nil then field = "passthrough" end
    if not format_allowed(node.format, field) then
      return nil
    end
    return { type = "raw_inline", format = node.format, value = node.value }

  -- ─── hard_break ────────────────────────────────────────────────────────
  elseif t == "hard_break" then
    return { type = "hard_break" }

  -- ─── soft_break ────────────────────────────────────────────────────────
  elseif t == "soft_break" then
    return { type = "soft_break" }

  -- ─── unknown node type — never silently pass through ───────────────────
  else
    -- Spec: unknown node types must not pass through.
    return nil
  end
end

--- Sanitize a list of inline nodes.
--
-- Handles the "promoted children" case from dropLinks by splicing the link's
-- children directly into the parent list.
--
-- @param nodes   table — list of inline nodes
-- @param policy  table — SanitizationPolicy
-- @return table  flat list of sanitized inline nodes
sanitize_inlines = function(nodes, policy)
  local result = {}
  for _, node in ipairs(nodes) do
    local sanitized = sanitize_inline(node, policy)
    if sanitized ~= nil then
      if sanitized.__promoted then
        -- Link children promotion: splice children into result
        for _, child in ipairs(sanitized.__promoted) do
          result[#result + 1] = child
        end
      else
        result[#result + 1] = sanitized
      end
    end
  end
  return result
end

-- ─── Block Node Handlers ─────────────────────────────────────────────────────

--- Sanitize a single block node.
--
-- Returns nil to drop the node, or a new node table to keep it.
--
-- @param node    table — a block-level Document AST node
-- @param policy  table — SanitizationPolicy
-- @return table|nil — sanitized node or nil if dropped
sanitize_block = function(node, policy)
  local t = node.type

  -- ─── heading ───────────────────────────────────────────────────────────
  if t == "heading" then
    local max = policy.maxHeadingLevel
    local min = policy.minHeadingLevel or 1

    -- "drop" variant: remove all heading nodes
    if max == "drop" then
      return nil
    end

    -- Clamp the heading level within [min, max].
    -- Default max is 6 when not specified.
    local effective_max = max or 6
    local level = node.level
    if level < min then
      level = min
    elseif level > effective_max then
      level = effective_max
    end

    local children = sanitize_inlines(node.children, policy)
    if #children == 0 then return nil end
    return { type = "heading", level = level, children = children }

  -- ─── paragraph ─────────────────────────────────────────────────────────
  elseif t == "paragraph" then
    local children = sanitize_inlines(node.children, policy)
    if #children == 0 then return nil end
    return { type = "paragraph", children = children }

  -- ─── code_block ────────────────────────────────────────────────────────
  elseif t == "code_block" then
    if policy.dropCodeBlocks then
      return nil
    end
    -- Leaf node: copy as-is
    return { type = "code_block", language = node.language, value = node.value }

  -- ─── blockquote ────────────────────────────────────────────────────────
  elseif t == "blockquote" then
    if policy.dropBlockquotes then
      return nil
    end
    local children = sanitize_blocks(node.children, policy)
    if #children == 0 then return nil end
    return { type = "blockquote", children = children }

  -- ─── list ──────────────────────────────────────────────────────────────
  elseif t == "list" then
    local children = sanitize_blocks(node.children, policy)
    -- An empty list is dropped (nothing left to show)
    if #children == 0 then return nil end
    return { type = "list", ordered = node.ordered, start = node.start,
             tight = node.tight, children = children }

  -- ─── list_item ─────────────────────────────────────────────────────────
  elseif t == "list_item" then
    local children = sanitize_blocks(node.children, policy)
    if #children == 0 then return nil end
    return { type = "list_item", children = children }

  -- ─── thematic_break ────────────────────────────────────────────────────
  elseif t == "thematic_break" then
    -- Leaf node: always keep
    return { type = "thematic_break" }

  -- ─── raw_block ─────────────────────────────────────────────────────────
  elseif t == "raw_block" then
    local field = policy.allowRawBlockFormats
    if field == nil then field = "passthrough" end
    if not format_allowed(node.format, field) then
      return nil
    end
    return { type = "raw_block", format = node.format, value = node.value }

  -- ─── unknown node type ─────────────────────────────────────────────────
  else
    return nil
  end
end

--- Sanitize a list of block nodes.
--
-- @param nodes   table — list of block-level nodes
-- @param policy  table — SanitizationPolicy
-- @return table  flat list of sanitized block nodes (never nil)
sanitize_blocks = function(nodes, policy)
  local result = {}
  for _, node in ipairs(nodes) do
    local sanitized = sanitize_block(node, policy)
    if sanitized ~= nil then
      result[#result + 1] = sanitized
    end
  end
  return result
end

-- ─── Public Entry Point ───────────────────────────────────────────────────────

--- Sanitize a DocumentNode by applying a SanitizationPolicy.
--
-- Returns a new DocumentNode with all policy violations removed or
-- neutralised. The input is never mutated.
--
-- The return value is always a DocumentNode — it is never nil, even if
-- all children are dropped. An empty document { type="document", children={} }
-- is a valid result.
--
-- @param document  table — root DocumentNode (must have type="document")
-- @param policy    table — SanitizationPolicy table (see policy.lua)
-- @return table    new DocumentNode
--
-- @example
--   local sanitizer = require("coding_adventures.document_ast_sanitizer")
--   local cm        = require("coding_adventures.commonmark_parser")
--
--   -- User-generated content — strict policy
--   local safe = sanitizer.sanitize(cm.parse(user_markdown), sanitizer.STRICT)
--
--   -- Documentation — pass through everything
--   local doc = sanitizer.sanitize(cm.parse(trusted_markdown), sanitizer.PASSTHROUGH)
function M.sanitize(document, policy)
  -- Normalise missing fields to their PASSTHROUGH defaults.
  -- This means callers can pass partial policy objects.
  policy = policy or {}

  local children = sanitize_blocks(document.children or {}, policy)
  return { type = "document", children = children }
end

return M
