# frozen_string_literal: true

# coding_adventures_document_ast — Format-Agnostic Document IR
#
# The Document AST is the "LLVM IR of documents" — a stable, typed,
# immutable tree that every front-end parser produces and every back-end
# renderer consumes. With a shared IR, N front-ends × M back-ends requires
# only N + M implementations instead of N × M.
#
#   Markdown ────────────────────────────────► HTML
#   reStructuredText ────► Document AST ────► PDF
#   HTML ────────────────────────────────────► Plain text
#   DOCX ────────────────────────────────────► DOCX
#
# This is a **types-only** package — there is no runtime logic and no
# parsing. Use it to annotate the AST values produced by a front-end
# or consumed by a back-end.
#
# === Quick Start ===
#
#   require "coding_adventures_document_ast"
#
#   include CodingAdventures::DocumentAst
#
#   doc = DocumentNode.new(children: [
#     HeadingNode.new(level: 1, children: [TextNode.new(value: "Hello")])
#   ])
#   doc.type         # => "document"
#   doc.children[0].type  # => "heading"
#
# === Key Design Decisions ===
#
# **No LinkDefinitionNode** — links in the IR are always fully resolved.
# Markdown's [text][label] reference syntax is resolved by the front-end;
# the IR only ever contains LinkNode { destination: "..." }.
#
# **RawBlockNode / RawInlineNode** instead of HtmlBlockNode / HtmlInlineNode.
# A `format` field ("html", "latex", ...) identifies the target back-end.
# Renderers skip nodes with an unknown `format`.
#
# Spec: TE00 — Document AST

require_relative "coding_adventures/document_ast/version"
require_relative "coding_adventures/document_ast/nodes"
