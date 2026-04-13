# frozen_string_literal: true

# ================================================================
# Python Parser -- Parses Python Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the python_lexer pattern: instead of writing
# a Python-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the Python grammar from python.grammar.
#
# The pipeline is:
#
#   1. Read python.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read python.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# The same two grammar files that describe Python's syntax are all
# that's needed to go from raw source code to an Abstract Syntax Tree.
# No Python-specific code is involved in the parsing itself -- just
# data files and generic engines.
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::PythonParser.parse("x = 1 + 2")
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_python_lexer"

module CodingAdventures
  module PythonParser
    # Paths to the grammar files, computed relative to this file.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    PYTHON_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "python.grammar")
    COMPILED_GRAMMAR_PATH = File.expand_path("_grammar.rb", __dir__)

    def self.parser_grammar
      @parser_grammar ||= CodingAdventures::GrammarTools.load_parser_grammar(COMPILED_GRAMMAR_PATH)
    end

    # Parse a string of Python source code into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using PythonLexer (which loads python.tokens)
    # 2. Reads the python.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # @param source [String] Python source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the Python lexer
      tokens = CodingAdventures::PythonLexer.tokenize(source, version: nil)

      # Step 2: Parse tokens using the compiled grammar.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar)
      parser.parse
    end
  end
end
