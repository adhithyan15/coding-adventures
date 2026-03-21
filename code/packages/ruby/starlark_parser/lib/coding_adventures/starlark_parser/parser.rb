# frozen_string_literal: true

# ================================================================
# Starlark Parser -- Parses Starlark Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the starlark_lexer pattern: instead of writing
# a Starlark-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the Starlark grammar from starlark.grammar.
#
# The pipeline is:
#
#   1. Read starlark.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read starlark.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# The same two grammar files that describe Starlark's syntax are all
# that's needed to go from raw source code to an Abstract Syntax Tree.
# No Starlark-specific code is involved in the parsing itself -- just
# data files and generic engines.
#
# Starlark's grammar is richer than Python's subset grammar because
# Starlark has a complete formal specification. It includes:
#
#   - Compound statements: if/elif/else, for, def (but NOT while or class)
#   - Simple statements: assignments, return, break, continue, pass, load
#   - Full expression hierarchy with 15 precedence levels
#   - List/dict literals and comprehensions
#   - Function calls with positional, keyword, *args, **kwargs
#   - Lambda expressions
#   - Tuple unpacking
#   - Augmented assignments (+=, -=, *=, etc.)
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::StarlarkParser.parse("x = 1 + 2")
#   # => ASTNode(rule_name: "file", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_starlark_lexer"

module CodingAdventures
  module StarlarkParser
    # Paths to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/starlark_parser/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       starlark.grammar   <-- we need this file
    #     packages/
    #       ruby/
    #         starlark_parser/
    #           lib/
    #             coding_adventures/
    #               starlark_parser/
    #                 parser.rb  <-- we are here (__dir__)
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    STARLARK_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "starlark.grammar")

    # Parse a string of Starlark source code into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using StarlarkLexer (which loads starlark.tokens)
    # 2. Reads the starlark.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # The root node of the AST will have rule_name "file" (as defined
    # in starlark.grammar's top-level rule). Each child node corresponds
    # to a grammar rule match, and leaf nodes are tokens.
    #
    # @param source [String] Starlark source code to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source)
      # Step 1: Tokenize using the Starlark lexer.
      # This loads starlark.tokens and produces a token stream that
      # includes INDENT/DEDENT/NEWLINE tokens for block structure.
      tokens = CodingAdventures::StarlarkLexer.tokenize(source)

      # Step 2: Load and parse the Starlark grammar.
      # The grammar file uses EBNF notation to describe Starlark's
      # syntax rules, from top-level file structure down to atoms.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(STARLARK_GRAMMAR_PATH, encoding: "UTF-8")
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
