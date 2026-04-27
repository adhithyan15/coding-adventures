# frozen_string_literal: true

# ================================================================
# coding_adventures_csharp_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_csharp_lexer"
#
# Ruby loads this file, which in turn loads the version and the
# tokenizer module. The tokenizer is where the real work happens.
# ================================================================

require_relative "coding_adventures/csharp_lexer/version"
require_relative "coding_adventures/csharp_lexer/tokenizer"

module CodingAdventures
  module CSharpLexer
  end
end
