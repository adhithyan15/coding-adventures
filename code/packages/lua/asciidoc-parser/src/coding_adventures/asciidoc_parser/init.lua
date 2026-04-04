-- AsciiDoc Parser — Public API
-- ============================
--
-- This module is the single entry point for the asciidoc_parser package.
-- It wires together the block parser and the document node constructor.
--
-- === Pipeline ===
--
--   AsciiDoc text
--       ↓
--   block_parser.parse_blocks(text)   → list of block nodes
--       ↓
--   { type = "document", children = blocks }
--
-- The returned document node is a plain Lua table that follows the
-- Document AST schema used throughout the coding-adventures ecosystem.
-- It can be passed directly to `coding_adventures.document_ast_to_html`
-- to produce HTML output.
--
-- === Usage ===
--
--   local parser = require("coding_adventures.asciidoc_parser")
--
--   local doc = parser.parse("= Hello\n\nWorld\n")
--   print(doc.type)              -- "document"
--   print(doc.children[1].type)  -- "heading"
--   print(doc.children[1].level) -- 1
--
-- @module coding_adventures.asciidoc_parser

local block_parser = require("coding_adventures.asciidoc_parser.block_parser")

local M = {}

M.VERSION = "0.1.0"

--- Parse an AsciiDoc string into a Document AST node.
--
-- The returned table has `type = "document"` and a `children` array of
-- block AST nodes (headings, paragraphs, code_blocks, lists, etc.).
--
-- @param text  string — AsciiDoc source text
-- @return table       — document node `{ type="document", children={…} }`
function M.parse(text)
  text = text or ""
  local blocks = block_parser.parse_blocks(text)
  return { type = "document", children = blocks }
end

return M
