# frozen_string_literal: true

# ================================================================
# coding_adventures_verilog_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_verilog_lexer"
#
# Ruby loads this file, which in turn loads the version, the
# preprocessor module, and the tokenizer. Dependencies are loaded
# in order: version first (no deps), then preprocessor (standalone),
# then tokenizer (which depends on grammar_tools and lexer gems).
# ================================================================

require_relative "coding_adventures/verilog_lexer/version"
require_relative "coding_adventures/verilog_lexer/preprocessor"
require_relative "coding_adventures/verilog_lexer/tokenizer"

module CodingAdventures
  module VerilogLexer
  end
end
