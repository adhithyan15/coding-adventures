# frozen_string_literal: true

# ================================================================
# coding_adventures_vhdl_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_vhdl_lexer"
#
# Ruby loads this file, which in turn loads the version constant
# and the tokenizer. Dependencies are loaded in order: version
# first (no deps), then tokenizer (which depends on grammar_tools
# and lexer gems).
#
# Unlike the Verilog lexer, there is no preprocessor module.
# VHDL has no preprocessor -- all constructs are part of the
# language proper (generics, generate statements, configurations).
# ================================================================

require_relative "coding_adventures/vhdl_lexer/version"
require_relative "coding_adventures/vhdl_lexer/tokenizer"

module CodingAdventures
  module VhdlLexer
  end
end
