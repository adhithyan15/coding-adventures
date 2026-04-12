# frozen_string_literal: true

# ================================================================
# Brainfuck Parser -- Parses Brainfuck Source into ASTs from Ruby
# ================================================================
#
# This module completes the grammar-driven pipeline for Brainfuck.
# Where the Lexer converts raw source characters into a flat token
# stream, the Parser converts that token stream into a tree that
# captures the nested loop structure.
#
# The full pipeline:
#
#   1. source (String)
#      → Lexer.tokenize → [Token, Token, ...]
#      → Parser.parse   → ASTNode (tree)
#
# Brainfuck's grammar has just four rules:
#
#   program     = { instruction } ;
#   instruction = loop | command ;
#   loop        = LOOP_START { instruction } LOOP_END ;
#   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
#
# This small grammar is all that's needed to describe the full
# structural syntax of Brainfuck. The grammar-driven parser engine
# handles the recursive descent automatically from the grammar file.
#
# Why a parser at all? The Brainfuck VM already works without one —
# it compiles source directly to bytecode in the Translator. The
# parser's value is pedagogical and compositional: it produces a
# proper AST that other tools (optimizers, visualizers, transpilers)
# can consume without rewriting a custom recursive descent parser.
#
# Usage:
#   ast = CodingAdventures::Brainfuck::Parser.parse("++[>+<-]")
#   # => ASTNode(rule_name: "program", children: [...]
# ================================================================

require_relative "lexer"

module CodingAdventures
  module Brainfuck
    module Parser
      # Path to the grammar file, computed relative to this file.
      #
      # Same directory navigation as the lexer: 6 levels up from
      # lib/coding_adventures/brainfuck/ reaches code/, then into
      # grammars/.
      GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
      BF_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "brainfuck.grammar")

      # Parse a Brainfuck source string into an Abstract Syntax Tree.
      #
      # This is the primary entry point. The pipeline is:
      #   1. Tokenize the source with Brainfuck::Lexer (loads brainfuck.tokens)
      #   2. Read brainfuck.grammar and build a ParserGrammar
      #   3. Feed tokens and grammar into GrammarDrivenParser
      #   4. Return the root ASTNode
      #
      # The root node has rule_name "program" and zero or more children
      # of rule_name "instruction". Each instruction is either a "loop"
      # (with nested instructions) or a "command" (a leaf token).
      #
      # If the source contains unmatched brackets, the grammar-driven
      # parser will raise an error because the loop rule requires a
      # matching LOOP_END for every LOOP_START.
      #
      # @param source [String] Brainfuck source code
      # @return [CodingAdventures::Parser::ASTNode] the root AST node
      # @raise [RuntimeError] if parsing fails (e.g. unmatched brackets)
      def self.parse(source)
        # Step 1: Tokenize the source.
        # Comments are already stripped by the lexer's skip: mechanism.
        # The resulting array contains only command tokens and EOF.
        tokens = CodingAdventures::Brainfuck::Lexer.tokenize(source)

        # Step 2: Load and parse the Brainfuck grammar.
        # The EBNF grammar file describes program, instruction, loop, and
        # command rules. The GrammarTools library converts this text into
        # a ParserGrammar struct holding the compiled rule graph.
        grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
          File.read(BF_GRAMMAR_PATH, encoding: "UTF-8")
        )

        # Step 3: Run the grammar-driven parser.
        # The parser uses recursive descent with the grammar's rule graph.
        # It starts with the first rule (program) and recursively matches
        # instructions. Loops trigger recursion — the parser re-enters
        # the instruction rule for each token inside the brackets.
        parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
        parser.parse
      end

      # Return a GrammarDrivenParser instance ready to parse Brainfuck source.
      #
      # Use this when you need direct access to the parser object. Most
      # callers should use parse/1 instead.
      #
      # @param source [String] Brainfuck source code
      # @return [CodingAdventures::Parser::GrammarDrivenParser] a ready parser
      def self.create_parser(source)
        tokens = CodingAdventures::Brainfuck::Lexer.tokenize(source)
        grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
          File.read(BF_GRAMMAR_PATH, encoding: "UTF-8")
        )
        CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      end
    end
  end
end
