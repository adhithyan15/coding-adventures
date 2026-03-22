# frozen_string_literal: true

# ================================================================
# TOML Parser -- Parses TOML Text into ASTs from Ruby
# ================================================================
#
# This module is a thin wrapper around the grammar-driven
# GrammarDrivenParser engine, feeding it the TOML grammar from
# toml.grammar.
#
# The pipeline is:
#
#   1. Read toml.tokens -> TokenGrammar -> GrammarLexer -> tokens
#   2. Read toml.grammar -> ParserGrammar -> GrammarDrivenParser -> AST
#
# TOML's grammar has 11 rules -- more than JSON (4) but fewer than
# CSS (36). The key rules are:
#
#   document           = { NEWLINE | expression } ;
#   expression         = array_table_header | table_header | keyval ;
#   keyval             = key EQUALS value ;
#   key                = simple_key { DOT simple_key } ;
#   simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING
#                      | TRUE | FALSE | INTEGER | FLOAT
#                      | OFFSET_DATETIME | LOCAL_DATETIME
#                      | LOCAL_DATE | LOCAL_TIME ;
#   table_header       = LBRACKET key RBRACKET ;
#   array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
#   value              = ... (12 alternatives) ;
#   array              = LBRACKET array_values RBRACKET ;
#   array_values       = { NEWLINE } [ value ... ] ;
#   inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
#
# Two-Phase Design
# ~~~~~~~~~~~~~~~~
#
# This module only handles the first phase (syntax). The AST it
# produces captures structure but does NOT enforce semantic rules
# like key uniqueness, table path consistency, or inline table
# immutability. A converter/semantic layer (like the Python
# implementation) would be needed for full TOML compliance.
#
# Usage:
#   ast = CodingAdventures::TomlParser.parse('name = "TOML"')
#   # => ASTNode(rule_name: "document", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_toml_lexer"

module CodingAdventures
  module TomlParser
    # Path to the grammar files, computed relative to this file.
    # Same 6-level navigation as the lexer.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    TOML_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "toml.grammar")

    # Parse a string of TOML text into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using TomlLexer (which loads toml.tokens)
    # 2. Reads the toml.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # The root node of the AST will have rule_name "document".
    # Children are NEWLINE tokens and expression nodes. Each expression
    # wraps a keyval, table_header, or array_table_header.
    #
    # @param source [String] TOML text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the TOML lexer.
      # This produces tokens including NEWLINEs (TOML is newline-sensitive).
      tokens = CodingAdventures::TomlLexer.tokenize(source)

      # Step 2: Load and parse the TOML grammar.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(TOML_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
