# frozen_string_literal: true

# ================================================================
# CSS Parser — Parses CSS Source Code into an AST from Ruby
# ================================================================
#
# The CSS parser follows the same two-phase grammar-driven pipeline
# as every other parser in this repo:
#
#   Phase 1: Tokenize with css.tokens → token stream
#   Phase 2: Parse with css.grammar → AST
#
# CSS grammar is more complex than Lisp's but simpler than Starlark's.
# The key structures in CSS:
#
#   stylesheet = { rule | at-rule | comment } ;
#   rule       = selector-list { declaration-list } ;
#   at-rule    = AT_KEYWORD ... ;
#   selector   = simple-selector { combinator simple-selector } ;
#   declaration = property COLON value SEMICOLON ;
#
# The grammar-driven parser handles operator precedence and rule
# structure automatically — we just supply the grammar file.
#
# Usage:
#   ast = CodingAdventures::CssParser.parse("h1 { color: red; }")
#   ast.rule_name # => "stylesheet"
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_css_lexer"

module CodingAdventures
  module CssParser
    GRAMMAR_DIR      = File.expand_path("../../../../../../grammars", __dir__)
    CSS_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "css", "css.grammar")
    COMPILED_GRAMMAR_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.parser_grammar
      @parser_grammar ||= CodingAdventures::GrammarTools.load_parser_grammar(COMPILED_GRAMMAR_PATH)
    end

    # Parse CSS source code into a generic AST.
    #
    # @param source [String] CSS source code
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      tokens = CodingAdventures::CssLexer.tokenize(source)
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar)
      parser.parse
    end
  end
end
