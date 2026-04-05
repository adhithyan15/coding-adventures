# frozen_string_literal: true

# ================================================================
# coding_adventures_mosaic_lexer -- Top-Level Require File
# ================================================================
#
# Entry point for the gem. Loads the version and tokenizer modules.
#
# Usage:
#   require "coding_adventures_mosaic_lexer"
#   tokens = CodingAdventures::MosaicLexer.tokenize(source)
# ================================================================

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

require_relative "coding_adventures/mosaic_lexer/version"
require_relative "coding_adventures/mosaic_lexer/tokenizer"

module CodingAdventures
  # Tokenizes .mosaic source using the grammar-driven lexer
  module MosaicLexer
  end
end
