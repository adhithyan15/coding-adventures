# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_directed_graph"
require "coding_adventures_state_machine"

require_relative "coding_adventures/css_lexer/version"
require_relative "coding_adventures/css_lexer/tokenizer"

module CodingAdventures
  # Tokenizes CSS source code using the grammar-driven lexer engine.
  #
  # See CodingAdventures::CssLexer::Tokenizer for the implementation.
  module CssLexer
  end
end
