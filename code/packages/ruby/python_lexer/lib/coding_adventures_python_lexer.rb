# frozen_string_literal: true

# ================================================================
# coding_adventures_python_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_python_lexer"
#
# Ruby loads this file, which in turn loads the version and the
# tokenizer module. The tokenizer is where the real work happens.
# ================================================================

require_relative "coding_adventures/python_lexer/version"
require_relative "coding_adventures/python_lexer/tokenizer"

module CodingAdventures
  module PythonLexer
  end
end
