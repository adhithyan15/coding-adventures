# frozen_string_literal: true

# ==========================================================================
# Grammar Tools -- Reading .tokens and .grammar Files
# ==========================================================================
#
# This gem is the Ruby port of the Python grammar-tools package. It reads
# two kinds of declarative grammar files:
#
#   .tokens files -- describe the *lexical* grammar (what tokens look like)
#   .grammar files -- describe the *syntactic* grammar (how tokens combine)
#
# Together they form a complete, machine-readable specification of a
# programming language's surface syntax. The cross-validator checks that
# the two files agree with each other.
#
# Why two files? The same reason compilers have separate lexer and parser
# phases: separation of concerns. The .tokens file says "these are the
# words," and the .grammar file says "these are the sentences."
# ==========================================================================

require_relative "coding_adventures/grammar_tools/version"
require_relative "coding_adventures/grammar_tools/token_grammar"
require_relative "coding_adventures/grammar_tools/parser_grammar"
require_relative "coding_adventures/grammar_tools/cross_validator"
require_relative "coding_adventures/grammar_tools/compiler"
require_relative "coding_adventures/grammar_tools/compiled_loader"

module CodingAdventures
  module GrammarTools
  end
end
