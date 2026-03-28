# frozen_string_literal: true

# ================================================================
# Lisp Parser — Parses Lisp Source Code into an AST from Ruby
# ================================================================
#
# Lisp has the simplest grammar of any programming language:
#
#   program   = { sexpr } ;
#   sexpr     = atom | list | quoted ;
#   atom      = NUMBER | SYMBOL | STRING ;
#   list      = LPAREN list_body RPAREN ;
#   list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
#   quoted    = QUOTE sexpr ;
#
# That's 6 rules. The beauty of Lisp is that code and data share
# the same structure — everything is a list. The parser doesn't
# need to distinguish (define x 1) from (+ 1 2) from (lambda (n) n).
# They're all just "list" nodes. The compiler assigns meaning.
#
# Parsing (define x 42) produces:
#   program → sexpr → list → list_body → [sexpr(define), sexpr(x), sexpr(42)]
#
# Usage:
#   ast = CodingAdventures::LispParser.parse("(define x 42)")
#   ast.rule_name # => "program"
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_lisp_lexer"

module CodingAdventures
  module LispParser
    GRAMMAR_DIR       = File.expand_path("../../../../../../grammars", __dir__)
    LISP_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "lisp.grammar")

    # Parse Lisp source code into a generic AST.
    #
    # @param source [String] Lisp source code
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      tokens = CodingAdventures::LispLexer.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(LISP_GRAMMAR_PATH, encoding: "UTF-8")
      )
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end
