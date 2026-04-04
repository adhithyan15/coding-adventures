# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_directed_graph"

require_relative "coding_adventures/mosaic_lexer/version"

module CodingAdventures
  # Tokenizes .mosaic source using the grammar-driven lexer
  module MosaicLexer
  end
end
