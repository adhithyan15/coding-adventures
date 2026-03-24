# frozen_string_literal: true

# ================================================================
# coding_adventures_vhdl_parser -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_vhdl_parser"
#
# Ruby loads this file, which requires:
#   1. The version constant (no dependencies)
#   2. The parser module (depends on grammar_tools, lexer, parser,
#      and vhdl_lexer gems — all loaded via require statements
#      in parser.rb)
#
# The public API is a single method:
#   CodingAdventures::VhdlParser.parse(source)
# ================================================================

require_relative "coding_adventures/vhdl_parser/version"
require_relative "coding_adventures/vhdl_parser/parser"

module CodingAdventures
  module VhdlParser
  end
end
