# frozen_string_literal: true

# ================================================================
# Ruby Parser — Parses Ruby Source Code into an AST from Ruby
# ================================================================
#
# Another moment of meta-circularity: Ruby code parsing Ruby code.
# The grammar-driven approach means we don't write a hand-crafted
# Ruby parser — we just supply the ruby.grammar file and let the
# generic GrammarDrivenParser handle the rest.
#
# Ruby's grammar covers the core language:
#
#   - Method definitions: def name(args) body end
#   - Assignments: x = expr
#   - Method calls: obj.method(args)
#   - Arithmetic with full precedence hierarchy
#   - Control flow: if/elsif/else/end, while/end, etc.
#   - Literals: integers, floats, strings, symbols, arrays, hashes
#
# Usage:
#   ast = CodingAdventures::RubyParser.parse("def greet(name)\n  puts name\nend")
#   ast.rule_name # => "program"
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_ruby_lexer"

module CodingAdventures
  module RubyParser
    GRAMMAR_DIR        = File.expand_path("../../../../../../grammars", __dir__)
    RUBY_GRAMMAR_PATH  = File.join(GRAMMAR_DIR, "ruby.grammar")
    COMPILED_GRAMMAR_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.parser_grammar
      @parser_grammar ||= CodingAdventures::GrammarTools.load_parser_grammar(COMPILED_GRAMMAR_PATH)
    end

    # Parse Ruby source code into a generic AST.
    #
    # @param source [String] Ruby source code
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      tokens = CodingAdventures::RubyLexer.tokenize(source)
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar)
      parser.parse
    end
  end
end
