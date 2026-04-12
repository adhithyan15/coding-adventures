# frozen_string_literal: true

# ================================================================
# C# Parser -- Parses C# Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the csharp_lexer pattern: instead of writing
# a C#-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the C# grammar from csharp/csharp<version>.grammar.
#
# The pipeline is:
#
#   1. Read csharp/csharp<version>.tokens
#        -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read csharp/csharp<version>.grammar
#        -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# This two-step pipeline is exactly the same as the Java parser
# pipeline, just pointing at the C# grammar files instead.
#
# Why two grammar files?
# ----------------------
# The .tokens file describes what the *lexer* should recognize:
#   - Regular expressions matching individual tokens (keywords,
#     operators, literals, identifiers, whitespace)
#   - Token type names like KEYWORD, NAME, NUMBER, STRING
#
# The .grammar file describes what the *parser* should accept:
#   - Context-free grammar rules (BNF or EBNF style)
#   - Productions like: statement -> var_declaration | assignment | ...
#
# Keeping them separate allows the same lexer grammar to feed
# multiple different parser grammars (e.g., a C# expression evaluator
# vs. a full C# compiler front-end).
#
# Version-aware usage
# -------------------
# Both the lexer and parser steps can target a specific C# version:
#
#   # Default (uses csharp/csharp12.0.grammar + csharp/csharp12.0.tokens)
#   ast = CodingAdventures::CSharpParser.parse("class Foo { }")
#
#   # C# 8.0 (uses csharp/csharp8.0.grammar + csharp/csharp8.0.tokens)
#   ast = CodingAdventures::CSharpParser.parse("int x = 1;", version: "8.0")
#
# Valid versions:
#   "1.0", "2.0", "3.0", "4.0", "5.0", "6.0", "7.0",
#   "8.0", "9.0", "10.0", "11.0", "12.0"
#
# Pass nil (or omit) for the default grammar (C# 12.0).
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_csharp_lexer"

module CodingAdventures
  module CSharpParser
    # Paths to the grammar files, computed relative to this file.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/csharp_parser/  <- __dir__
    #   lib/coding_adventures/                <- ..
    #   lib/                                  <- ../..
    #   <gem root>/                           <- ../../..
    #   ruby/                                 <- ../../../..
    #   packages/                             <- ../../../../..
    #   code/                                 <- ../../../../../..
    #   grammars/                             <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)

    # The default C# version used when no version is specified.
    DEFAULT_VERSION = "12.0"

    # All valid C# grammar versions supported by the versioned grammar
    # files in code/grammars/csharp/.
    VALID_VERSIONS = %w[
      1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0
    ].freeze

    # Resolve the path to the .grammar file for a given C# version.
    #
    # When `version` is nil or empty, the default version (C# 12.0) is used.
    # When a valid version string is given, the versioned file is returned.
    # An unknown version raises ArgumentError immediately.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .grammar file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_grammar_path(version)
      effective_version = if version.nil? || version.empty?
        DEFAULT_VERSION
      elsif VALID_VERSIONS.include?(version)
        version
      else
        raise ArgumentError,
          "Unknown C# version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
      File.join(GRAMMAR_DIR, "csharp", "csharp#{effective_version}.grammar")
    end

    # Parse a string of C# source code into a generic AST.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # C# grammar.  The same version is forwarded to the C# lexer so that
    # both the token grammar and the parser grammar match.
    # When omitted (or nil), the default grammars are used (C# 12.0).
    #
    # @param source [String] C# source code to parse
    # @param version [String, nil] C# version tag (e.g. "8.0", "12.0") or nil
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.parse(source, version: nil)
      # Step 1: Tokenize using the C# lexer (version-aware)
      tokens = CodingAdventures::CSharpLexer.tokenize(source, version: version)

      # Step 2: Load and parse the C# grammar for this version
      grammar_path = resolve_grammar_path(version)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(grammar_path, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end

    # Alias: parse_csharp delegates to parse for API parity with the
    # public function name described in the package specification.
    #
    # @param source [String] C# source code to parse
    # @param version [String, nil] C# version tag or nil
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse_csharp(source, version: nil)
      parse(source, version: version)
    end

    # Create a parser context for C# source code.
    #
    # Unlike `parse`, which eagerly produces the full AST,
    # `create_csharp_parser` returns a hash describing the configured
    # parser state.  This is useful when building pipelines or deferred
    # parsing workflows where you want to defer actual parsing.
    #
    # @param source [String] C# source code to parse
    # @param version [String, nil] C# version tag (e.g. "8.0", "12.0") or nil
    # @return [Hash] a map with :source, :version, and :language keys
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.create_csharp_parser(source, version: nil)
      # Validate the version eagerly so callers get immediate feedback
      resolve_grammar_path(version)
      { source: source, version: version, language: :csharp }
    end
  end
end
