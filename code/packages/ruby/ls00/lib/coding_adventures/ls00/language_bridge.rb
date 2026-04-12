# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00 — LanguageBridge (Duck-Type Documentation)
# ================================================================
#
# # Design Philosophy: Duck Typing
#
# Ruby uses duck typing instead of Go's interface mechanism. Rather than
# declaring explicit interfaces, we document the expected methods and use
# +respond_to?+ at runtime for capability detection.
#
# A bridge object must implement two required methods:
#
#   bridge.tokenize(source)  -> [Token, ...]
#   bridge.parse(source)     -> [ast, [Diagnostic, ...]]
#
# All other features are optional. The server checks at runtime:
#
#   if bridge.respond_to?(:hover)
#     result = bridge.hover(ast, pos)
#   end
#
# This matches the LSP spec's philosophy: capabilities are advertised,
# not assumed. An editor won't even try to ask for hover if the server
# didn't advertise it.
#
# # Required Methods
#
#   tokenize(source)
#     Lexes the source string and returns an array of Token structs.
#     Used for semantic highlighting.
#
#   parse(source)
#     Parses the source string and returns [ast, diagnostics].
#     ast: any object representing the parsed syntax tree.
#     diagnostics: array of Diagnostic structs (errors, warnings, hints).
#     Even on parse error, return a partial AST when possible.
#
# # Optional Methods (checked via respond_to?)
#
#   hover(ast, pos)
#     Returns a HoverResult or nil.
#
#   definition(ast, pos, uri)
#     Returns a Location or nil.
#
#   references(ast, pos, uri, include_declaration)
#     Returns an array of Location.
#
#   completion(ast, pos)
#     Returns an array of CompletionItem.
#
#   rename(ast, pos, new_name)
#     Returns a WorkspaceEdit or nil.
#
#   semantic_tokens(source, tokens)
#     Returns an array of SemanticToken.
#
#   document_symbols(ast)
#     Returns an array of DocumentSymbol.
#
#   folding_ranges(ast)
#     Returns an array of FoldingRange.
#
#   signature_help(ast, pos)
#     Returns a SignatureHelpResult or nil.
#
#   format(source)
#     Returns an array of TextEdit.
#
# ================================================================

module CodingAdventures
  module Ls00
    # This module exists purely as documentation. Ruby's duck typing means
    # there is no interface to enforce. The server uses respond_to? checks
    # at runtime to detect which optional methods the bridge supports.
    #
    # See the comment block above for the full contract.
    module LanguageBridge
    end
  end
end
