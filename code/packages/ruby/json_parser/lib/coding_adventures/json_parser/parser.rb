# frozen_string_literal: true

# ================================================================
# JSON Parser -- Parses JSON Text into ASTs from Ruby
# ================================================================
#
# This module mirrors the json_lexer pattern: instead of writing
# a JSON-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the JSON grammar from json.grammar.
#
# The pipeline is:
#
#   1. Read json.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read json.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# The same two grammar files that describe JSON's syntax are all
# that's needed to go from raw JSON text to an Abstract Syntax Tree.
# No JSON-specific code is involved in the parsing itself -- just
# data files and generic engines.
#
# JSON's grammar is remarkably small -- just four rules:
#
#   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
#   object = LBRACE [ pair { COMMA pair } ] RBRACE
#   pair   = STRING COLON value
#   array  = LBRACKET [ value { COMMA value } ] RBRACKET
#
# The grammar is recursive: value references object and array, which
# reference value again. This mutual recursion is what allows JSON
# to represent arbitrarily deep nested structures.
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::JsonParser.parse('{"key": 42}')
#   # => ASTNode(rule_name: "value", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_json_lexer"

module CodingAdventures
  module JsonParser
    # Paths to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/json_parser/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       json.grammar   <-- we need this file
    #     packages/
    #       ruby/
    #         json_parser/
    #           lib/
    #             coding_adventures/
    #               json_parser/
    #                 parser.rb  <-- we are here (__dir__)
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    JSON_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "json.grammar")

    # Parse a string of JSON text into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using JsonLexer (which loads json.tokens)
    # 2. Reads the json.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # The root node of the AST will have rule_name "value" (as defined
    # in json.grammar's first rule). Each child node corresponds to a
    # grammar rule match, and leaf nodes are tokens.
    #
    # @param source [String] JSON text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the JSON lexer.
      # This loads json.tokens and produces a flat token stream with
      # no INDENT/DEDENT/NEWLINE tokens (JSON ignores whitespace).
      tokens = CodingAdventures::JsonLexer.tokenize(source)

      # Step 2: Load and parse the JSON grammar.
      # The grammar file uses EBNF notation to describe JSON's four
      # syntax rules: value, object, pair, and array.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(JSON_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      # The parser uses recursive descent with backtracking to match
      # the token stream against the grammar rules, producing an AST
      # where each node records which rule produced it.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
