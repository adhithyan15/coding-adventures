# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_mosaic_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_directed_graph"
require "coding_adventures_parser"
require "coding_adventures_state_machine"

require_relative "coding_adventures/mosaic_analyzer/version"

module CodingAdventures
  # Validates the Mosaic AST and produces a typed MosaicIR
  module MosaicAnalyzer
  end
end
