# frozen_string_literal: true

# ================================================================
# Verilog Lexer -- Tokenizes Verilog HDL Source Code from Ruby
# ================================================================
#
# This module demonstrates the grammar-driven approach applied to a
# Hardware Description Language. Verilog is fundamentally different
# from software languages like JavaScript or Python:
#
# - It describes physical structures (gates, wires, flip-flops)
#   that exist simultaneously and operate in parallel.
# - Numbers carry bit-width information: 8'hFF is an 8-bit hex value.
# - Special identifiers: $system_tasks, `compiler_directives, \escaped_ids.
# - Operators like & and | serve double duty (bitwise AND/OR and
#   reduction operators).
#
# Despite these differences, the grammar-driven lexer handles them
# all through the token definitions in verilog.tokens. No Verilog-
# specific code is needed in the lexer engine itself.
#
# The optional preprocessor (see preprocessor.rb) resolves `define,
# `ifdef, `include, and `timescale directives before tokenization.
#
# Usage:
#   # Without preprocessing:
#   tokens = CodingAdventures::VerilogLexer.tokenize("wire [7:0] data;")
#
#   # With preprocessing (resolves `define, `ifdef, etc.):
#   source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
#   tokens = CodingAdventures::VerilogLexer.tokenize(source, preprocess: true)
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module VerilogLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/verilog_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VERILOG_TOKENS_PATH = File.join(GRAMMAR_DIR, "verilog.tokens")

    # Tokenize a string of Verilog HDL source code into an array of Token objects.
    #
    # @param source [String] Verilog source code to tokenize
    # @param preprocess [Boolean] whether to run the preprocessor first (default: false)
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source, preprocess: false)
      # If preprocessing is requested, resolve all compiler directives
      # (`define, `ifdef, `include, `timescale) before tokenizing.
      processed_source = if preprocess
        Preprocessor.process(source)
      else
        source
      end

      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(VERILOG_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(processed_source, grammar)
      lexer.tokenize
    end
  end
end
