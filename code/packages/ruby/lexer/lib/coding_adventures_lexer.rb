# frozen_string_literal: true

# ==========================================================================
# Lexer -- Breaking Source Code into Tokens
# ==========================================================================
#
# This gem is the Ruby port of the Python lexer package. It provides two
# complementary approaches to tokenization:
#
# 1. A hand-written Tokenizer that reads source code character by character
#    and produces tokens using hardcoded rules. This is the reference
#    implementation -- clear, well-documented, easy to debug.
#
# 2. A grammar-driven GrammarLexer that reads token definitions from a
#    .tokens file (via the grammar_tools gem) and uses regex matching to
#    tokenize. This is the flexible, data-driven approach.
#
# Both produce identical Token objects so downstream consumers (the parser)
# don't care which one generated the tokens.
# ==========================================================================

require_relative "coding_adventures/lexer/version"
require_relative "coding_adventures/lexer/token_type"
require_relative "coding_adventures/lexer/token"
require_relative "coding_adventures/lexer/tokenizer"
require_relative "coding_adventures/lexer/grammar_lexer"

module CodingAdventures
  module Lexer
  end
end
