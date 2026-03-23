# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

require_relative "coding_adventures/lattice_lexer/version"
require_relative "coding_adventures/lattice_lexer/tokenizer"

module CodingAdventures
  # Tokenizer for the Lattice CSS superset language.
  #
  # Lattice extends CSS with variables, mixins, control flow, functions,
  # and modules. This module exposes the tokenizer via two methods:
  #
  #   CodingAdventures::LatticeLexer.tokenize(source) -> Array<Token>
  #   CodingAdventures::LatticeLexer.create_lexer(source) -> GrammarLexer
  module LatticeLexer
  end
end
