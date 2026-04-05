# frozen_string_literal: true

# ================================================================
# coding_adventures_mosaic_analyzer -- Top-Level Require File
# ================================================================
#
# Usage:
#   require "coding_adventures_mosaic_analyzer"
#   ir = CodingAdventures::MosaicAnalyzer.analyze(source)
# ================================================================

# IMPORTANT: Require dependencies FIRST.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_mosaic_parser"

require_relative "coding_adventures/mosaic_analyzer/version"
require_relative "coding_adventures/mosaic_analyzer/ir"
require_relative "coding_adventures/mosaic_analyzer/analyzer"

module CodingAdventures
  # Validates the Mosaic AST and produces a typed MosaicIR
  module MosaicAnalyzer
  end
end
