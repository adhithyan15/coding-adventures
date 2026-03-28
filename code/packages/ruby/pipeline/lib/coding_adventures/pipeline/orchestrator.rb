# frozen_string_literal: true

require "coding_adventures_lexer"
require "coding_adventures_parser"

# ================================================================
# Pipeline Orchestrator — Wiring the Computing Stack
# ================================================================
#
# Chapter 1: What Is a Pipeline?
# ================================
# Imagine a factory assembly line. Raw steel enters at one end and a
# finished car rolls out the other. Between those two points, a dozen
# stations each do one specific job: stamping, welding, painting.
# No single station builds the whole car — each one transforms its
# input and passes the result downstream.
#
# A **compiler pipeline** works the same way. Raw source code enters at
# one end, and structured data (an AST, variables, output) comes out the
# other. Our pipeline has two stations:
#
#     Source code  →  Lexer  →  Parser  →  AST
#
# 1. **Lexer** (tokenizer): Reads raw characters and groups them into
#    meaningful tokens — identifiers, numbers, operators, keywords.
#    Input:  `"x = 1 + 2"`
#    Output: `[NAME("x"), EQUALS, NUMBER(1), PLUS, NUMBER(2)]`
#
# 2. **Parser**: Takes the flat token stream and builds a tree structure
#    (the Abstract Syntax Tree) that encodes precedence and grouping.
#    Input:  token list
#    Output: `Assignment(Name("x"), BinaryOp(1, "+", 2))`
#
# Chapter 2: Stage Captures
# ==========================
# The pipeline doesn't just *run* code — it **records** what happened
# at every stage. This is critical for visualizers and debuggers:
#
# - `LexerStage` captures the full token list and a count.
# - `ParserStage` captures the AST and the number of tree nodes.
# - `PipelineResult` bundles both stages together with the source.
#
# Chapter 3: Extension Points
# ============================
# The `run` method accepts optional `lexer_keywords` so callers can
# configure the lexer for different languages. The pipeline itself is
# stateless — call `run` as many times as you like.
#
# Example:
#
#   result = CodingAdventures::Pipeline::Orchestrator.new.run("x = 1 + 2")
#   result.lexer_stage.token_count    # => 7
#   result.parser_stage.ast           # => Program(...)
#   result.source                     # => "x = 1 + 2"

module CodingAdventures
  module Pipeline
    # Snapshot from the lexer stage.
    #
    # @!attribute tokens [Array<CodingAdventures::Lexer::Token>]
    #   Every token produced by the lexer, including the final EOF.
    # @!attribute token_count [Integer]
    #   Quick summary: how many tokens were produced.
    # @!attribute source [String]
    #   The original source code that was tokenized.
    LexerStage = Struct.new(:tokens, :token_count, :source, keyword_init: true)

    # Snapshot from the parser stage.
    #
    # @!attribute ast [Object]
    #   The root AST node (a Program or similar) produced by the parser.
    # @!attribute node_count [Integer]
    #   Approximate number of AST nodes (for quick summaries).
    ParserStage = Struct.new(:ast, :node_count, keyword_init: true)

    # The complete result of running the pipeline.
    #
    # @!attribute source [String] original source code
    # @!attribute lexer_stage [LexerStage] lexer output
    # @!attribute parser_stage [ParserStage] parser output
    PipelineResult = Struct.new(:source, :lexer_stage, :parser_stage, keyword_init: true)

    # Orchestrator chains the lexer and parser into a single `run` call.
    class Orchestrator
      # Run the full pipeline on +source+ and return a PipelineResult.
      #
      # @param source [String] source code to process
      # @param lexer_keywords [Array<String>] keywords to recognize (default: common set)
      # @return [PipelineResult]
      def run(source, lexer_keywords: default_keywords)
        # ── Stage 1: Lex ────────────────────────────────────────────
        # Tokenize the source into a flat list of tokens.
        lexer  = CodingAdventures::Lexer::Tokenizer.new(source, keywords: lexer_keywords)
        tokens = lexer.tokenize

        lexer_stage = LexerStage.new(
          tokens: tokens,
          token_count: tokens.size,
          source: source
        )

        # ── Stage 2: Parse ──────────────────────────────────────────
        # Build an AST from the token list.
        parser = CodingAdventures::Parser::RecursiveDescentParser.new(tokens)
        ast    = parser.parse

        parser_stage = ParserStage.new(
          ast: ast,
          node_count: count_nodes(ast)
        )

        PipelineResult.new(
          source: source,
          lexer_stage: lexer_stage,
          parser_stage: parser_stage
        )
      end

      private

      # Default keyword list used by the Starlark-flavoured lexer.
      # These are the words that get emitted as KEYWORD tokens rather
      # than NAME tokens.
      def default_keywords
        %w[if else elif while for def return True False None and or not in]
      end

      # Recursively count the number of nodes in an AST.
      # Our AST nodes are built with Data.define, so they respond to #members.
      # Array fields (like Program#statements) are iterated to count children.
      #
      # @param node [Object] any AST node
      # @return [Integer] total node count
      def count_nodes(node)
        return 0 if node.nil?
        return node.sum { |el| count_nodes(el) } if node.is_a?(Array)
        return 1 unless node.respond_to?(:members)

        1 + node.members.sum { |m| count_nodes(node.send(m)) }
      end
    end
  end
end
