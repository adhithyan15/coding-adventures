# frozen_string_literal: true

# ================================================================
# coding_adventures_haskell_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_haskell_lexer"
#
# Ruby loads this file, which in turn loads the version and the
# lexer module. The lexer is where the real work happens.
# ================================================================

require_relative "coding_adventures/haskell_lexer/version"
require_relative "coding_adventures/haskell_lexer/lexer"

module CodingAdventures
  module HaskellLexer
  end
end

