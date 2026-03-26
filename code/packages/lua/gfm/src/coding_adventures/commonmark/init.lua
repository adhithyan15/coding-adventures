-- GFM Pipeline
-- ====================
--
-- A thin convenience wrapper that combines the GFM parser and the
-- Document AST → HTML renderer into a single pipeline function.
--
-- === Why a separate package? ===
--
-- The Document AST ecosystem is designed so that parsers and renderers are
-- decoupled via the IR (Intermediate Representation). You can:
--
--   - Use only the parser (coding_adventures.commonmark_parser) to get an AST
--     and do custom processing
--   - Use only the renderer (coding_adventures.document_ast_to_html) to render
--     an AST produced by a different front-end
--   - Use this package for the common case: Markdown → HTML
--
-- This mirrors the N front-ends × M back-ends architecture:
--
--   Markdown ─── commonmark_parser ──► DocumentNode ──► document_ast_to_html ──► HTML
--
-- === Quick Start ===
--
--   local commonmark = require("coding_adventures.commonmark")
--
--   -- Simple Markdown → HTML
--   local html = commonmark.render("# Hello\n\nWorld\n")
--   -- → "<h1>Hello</h1>\n<p>World</p>\n"
--
--   -- Two-step: parse then render
--   local doc = commonmark.parse("# Hello\n")
--   local html = commonmark.to_html(doc)
--
--   -- Access the AST directly
--   local doc = commonmark.parse("# Hello\n")
--   print(doc.type)              -- "document"
--   print(doc.children[1].type)  -- "heading"
--   print(doc.children[1].level) -- 1
--
-- @module coding_adventures.commonmark

local parser = require("coding_adventures.commonmark_parser")
local renderer = require("coding_adventures.document_ast_to_html")

local M = {}

--- Parse a GitHub Flavored Markdown string into a DocumentNode AST.
--
-- This is a direct re-export of coding_adventures.commonmark_parser.parse.
-- Use it when you need to inspect or transform the AST before rendering.
--
-- @param markdown  string — the Markdown source string
-- @param options   table  — optional parse options (reserved)
-- @return table           — the root document node
--
-- @example
--   local doc = commonmark.parse("## Heading\n\n- item 1\n- item 2\n")
--   doc.children[1].type   -- "heading"
--   doc.children[2].type   -- "list"
M.parse = parser.parse

--- Render a Document AST to an HTML string.
--
-- This is a direct re-export of coding_adventures.document_ast_to_html.to_html.
-- Use it when you have an AST from any front-end and want HTML output.
--
-- @param document  table — the root document node
-- @param options   table — render options (optional)
--   options.sanitize  boolean — when true, strip all raw HTML
-- @return string — HTML string
--
-- @example
--   local doc = commonmark.parse("Hello *world*\n")
--   local html = commonmark.to_html(doc)
--   -- → "<p>Hello <em>world</em></p>\n"
M.to_html = renderer.to_html

--- Parse Markdown and render it to HTML in one step.
--
-- Equivalent to calling `to_html(parse(markdown, parse_options), render_options)`.
-- This is the most convenient entry point for the common use case.
--
-- ⚠️  Security notice: Pass `{ sanitize=true }` as the second argument when
-- rendering untrusted Markdown (user-supplied content). By default, raw HTML
-- blocks in the Markdown are passed through verbatim — a potential XSS vector.
--
-- @param markdown       string — the Markdown source string
-- @param render_options table  — render options (optional)
--   render_options.sanitize  boolean — strip raw HTML when true
-- @return string — HTML output
--
-- @example
--   -- Trusted Markdown (documentation, static sites):
--   local html = commonmark.render("# Hello\n\nWorld\n")
--   -- → "<h1>Hello</h1>\n<p>World</p>\n"
--
--   -- Untrusted Markdown (user content, forums, etc.):
--   local html = commonmark.render(user_input, { sanitize=true })
function M.render(markdown, render_options)
  local doc = parser.parse(markdown)
  return renderer.to_html(doc, render_options)
end

M.VERSION = "0.1.0"

return M
