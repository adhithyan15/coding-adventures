-- AsciiDoc Pipeline
-- =================
--
-- A thin convenience wrapper that combines the AsciiDoc parser and the
-- Document AST → HTML renderer into a single pipeline.
--
-- === Why a separate package? ===
--
-- The Document AST ecosystem follows an N × M architecture:
--
--   N front-ends (Markdown, AsciiDoc, GFM, …)
--       ↓  each produces a Document AST
--   M back-ends (HTML, plain text, PDF, …)
--       ↑  each consumes a Document AST
--
-- The parser (`coding_adventures.asciidoc_parser`) and renderer
-- (`coding_adventures.document_ast_to_html`) are kept separate so that
-- users who only need one half of the pipeline do not have to pull in
-- the other.  This package bundles both for the common case.
--
-- === Quick Start ===
--
--   local asciidoc = require("coding_adventures.asciidoc")
--
--   -- AsciiDoc → HTML (one step)
--   local html = asciidoc.to_html("= Hello\n\nWorld\n")
--   -- → "<h1>Hello</h1>\n<p>World</p>\n"
--
--   -- Two-step: parse then render
--   local doc  = asciidoc.parse("= Hello\n")
--   local html = asciidoc.to_html_from_ast(doc)
--
-- @module coding_adventures.asciidoc

local asciidoc_parser = require("coding_adventures.asciidoc_parser")
local renderer        = require("coding_adventures.document_ast_to_html")

local M = {}

M.VERSION = "0.1.0"

--- Parse an AsciiDoc string into a Document AST node.
--
-- This re-exports `coding_adventures.asciidoc_parser.parse`.
-- Use it when you need to inspect or transform the AST before rendering.
--
-- @param text  string — AsciiDoc source
-- @return table       — document node `{ type="document", children={…} }`
M.parse = asciidoc_parser.parse

--- Render a Document AST to an HTML string.
--
-- This re-exports `coding_adventures.document_ast_to_html.to_html`.
--
-- @param document  table — document node produced by `parse`
-- @return string         — HTML string
M.to_html_from_ast = renderer.to_html

--- Parse AsciiDoc and render it to HTML in one step.
--
-- This is the most convenient entry point for the common use case.
--
-- @param text  string — AsciiDoc source
-- @return string      — HTML output
function M.to_html(text)
  local doc = asciidoc_parser.parse(text)
  return renderer.to_html(doc)
end

--- Alias for `to_html`.  Provided for consistency with the commonmark package.
M.render = M.to_html

return M
