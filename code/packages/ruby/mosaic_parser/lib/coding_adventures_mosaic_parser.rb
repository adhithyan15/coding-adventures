# frozen_string_literal: true

# ================================================================
# coding_adventures_mosaic_parser -- Top-Level Require File
# ================================================================
#
# Entry point for the gem. Loads version and parser modules.
#
# Usage:
#   require "coding_adventures_mosaic_parser"
#   ast = CodingAdventures::MosaicParser.parse(source)
# ================================================================

# IMPORTANT: Require dependencies FIRST, before own modules.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"

require_relative "coding_adventures/mosaic_parser/version"
require_relative "coding_adventures/mosaic_parser/parser"

module CodingAdventures
  # Parses Mosaic source text into an ASTNode tree
  module MosaicParser
  end
end
