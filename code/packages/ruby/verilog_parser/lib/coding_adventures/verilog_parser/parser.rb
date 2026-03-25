# frozen_string_literal: true

# ================================================================
# Verilog Parser -- Parses Verilog HDL Source Code into ASTs from Ruby
# ================================================================
#
# This module follows the same pattern as the JavaScript parser: instead
# of writing a Verilog-specific parser from scratch, we reuse the
# general-purpose GrammarDrivenParser engine from the coding_adventures_parser
# gem, feeding it the Verilog grammar from verilog.grammar.
#
# Verilog is a Hardware Description Language (HDL). Unlike software
# languages that describe sequential computations, Verilog describes
# physical circuits — modules, wires, registers, and gates that all
# operate simultaneously. The grammar captures this structure:
#
#   source_text
#     └── description
#           └── module_declaration
#                 ├── port_list (inputs/outputs)
#                 ├── continuous_assign (combinational logic)
#                 ├── always_construct (sequential/combinational behavior)
#                 └── module_instantiation (hierarchy)
#
# The pipeline is:
#
#   1. Read verilog.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read verilog.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::VerilogParser.parse("module top; endmodule")
#   # => ASTNode(rule_name: "source_text", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_verilog_lexer"

module CodingAdventures
  module VerilogParser
    # Path to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/verilog_parser/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VERILOG_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "verilog.grammar")

    # Parse a string of Verilog HDL source code into a generic AST.
    #
    # The parser first tokenizes the source using the Verilog lexer, then
    # loads the Verilog grammar, and finally feeds both into the grammar-
    # driven parser to produce an AST.
    #
    # @param source [String] Verilog source code to parse
    # @param preprocess [Boolean] whether to run the Verilog preprocessor
    #   first (resolves `define, `ifdef, etc.). Default: false.
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source, preprocess: false)
      # Step 1: Tokenize using the Verilog lexer.
      # The lexer reads verilog.tokens and produces a stream of Token objects
      # (keywords like "module", operators like "&", identifiers, numbers, etc.).
      tokens = CodingAdventures::VerilogLexer.tokenize(source, preprocess: preprocess)

      # Step 2: Load and parse the Verilog grammar.
      # The grammar file defines rules like:
      #   source_text = { description } ;
      #   module_declaration = "module" NAME [...] "endmodule" ;
      # These rules tell the parser how to build the AST from the token stream.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(VERILOG_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      # The parser walks the token stream, matching tokens against grammar rules,
      # and builds a tree of ASTNode objects. Each ASTNode has a rule_name
      # (e.g., "module_declaration") and children (sub-nodes and tokens).
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
