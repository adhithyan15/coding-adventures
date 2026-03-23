# frozen_string_literal: true

# ================================================================
# VHDL Parser -- Parses VHDL Source Code into ASTs from Ruby
# ================================================================
#
# This module follows the same pattern as the Verilog parser: instead
# of writing a VHDL-specific parser from scratch, we reuse the
# general-purpose GrammarDrivenParser engine from the coding_adventures_parser
# gem, feeding it the VHDL grammar from vhdl.grammar.
#
# VHDL is a Hardware Description Language (HDL) that takes a fundamentally
# different approach from Verilog. Where Verilog is implicit and concise
# (like C), VHDL is explicit and verbose (like Ada). The key differences
# that affect parsing:
#
#   1. CASE INSENSITIVE: "ENTITY", "Entity", and "entity" are identical.
#      The VHDL lexer normalizes everything to lowercase before we see it.
#
#   2. SEPARATE INTERFACE AND IMPLEMENTATION: An entity declares the
#      interface (ports), and a separate architecture provides the
#      implementation. In Verilog, both are in a single module.
#
#   3. STRONG TYPING: Every signal must have an explicit type declaration.
#      There are no implicit wire types.
#
# The grammar structure mirrors VHDL's two-part design:
#
#   design_file
#     └── design_unit
#           ├── entity_declaration      (the interface — ports and generics)
#           │     ├── port_clause
#           │     └── generic_clause
#           └── architecture_body       (the implementation — signals and logic)
#                 ├── signal_declaration
#                 ├── signal_assignment_concurrent
#                 ├── process_statement
#                 │     ├── if_statement
#                 │     ├── case_statement
#                 │     └── signal_assignment_seq
#                 └── component_instantiation
#
# The pipeline is:
#
#   1. Read vhdl.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#      (with case normalization to lowercase)
#   2. Read vhdl.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Usage:
#   ast = CodingAdventures::VhdlParser.parse("entity e is end entity e;")
#   # => ASTNode(rule_name: "design_file", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_vhdl_lexer"

module CodingAdventures
  module VhdlParser
    # Path to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/vhdl_parser/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VHDL_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "vhdl.grammar")

    # Parse a string of VHDL source code into a generic AST.
    #
    # The parser first tokenizes the source using the VHDL lexer (which
    # handles case normalization — VHDL is case-insensitive), then loads
    # the VHDL grammar, and finally feeds both into the grammar-driven
    # parser to produce an AST.
    #
    # Unlike the Verilog parser, there is no `preprocess:` option because
    # VHDL has no preprocessor. All constructs (generics, generate
    # statements, configurations) are part of the language proper.
    #
    # @param source [String] VHDL source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the VHDL lexer.
      # The lexer reads vhdl.tokens and produces a stream of Token objects.
      # Crucially, it normalizes all identifiers and keywords to lowercase,
      # implementing VHDL's case insensitivity. So "ENTITY", "Entity", and
      # "entity" all produce the same KEYWORD token with value "entity".
      tokens = CodingAdventures::VhdlLexer.tokenize(source)

      # Step 2: Load and parse the VHDL grammar.
      # The grammar file defines rules like:
      #   design_file = { design_unit } ;
      #   entity_declaration = "entity" NAME "is" [...] ;
      # These rules tell the parser how to build the AST from the token stream.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(VHDL_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      # The parser walks the token stream, matching tokens against grammar rules,
      # and builds a tree of ASTNode objects. Each ASTNode has a rule_name
      # (e.g., "entity_declaration") and children (sub-nodes and tokens).
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
