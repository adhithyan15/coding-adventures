# frozen_string_literal: true

# ================================================================
# TypeScript Parser -- Parses TypeScript Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the typescript_lexer pattern: instead of writing
# a TypeScript-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the TypeScript grammar from typescript.grammar.
#
# The pipeline is:
#
#   1. Read typescript.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read typescript.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Version-aware usage
# -------------------
# Both the lexer and parser steps can target a specific TypeScript version:
#
#   # Generic (uses typescript.grammar + typescript.tokens)
#   ast = CodingAdventures::TypescriptParser.parse("let x = 1 + 2;")
#
#   # TypeScript 5.0 (uses typescript/ts5.0.grammar + typescript/ts5.0.tokens)
#   ast = CodingAdventures::TypescriptParser.parse("let x = 1;", version: "ts5.0")
#
# Valid versions: "ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"
# Pass nil (or omit) for the generic grammar.
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_typescript_lexer"

module CodingAdventures
  module TypescriptParser
    # Paths to the grammar files, computed relative to this file.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/typescript_parser/  <- __dir__
    #   lib/coding_adventures/                    <- ..
    #   lib/                                      <- ../..
    #   <gem root>/                               <- ../../..
    #   ruby/                                     <- ../../../..
    #   packages/                                 <- ../../../../..
    #   code/                                     <- ../../../../../..
    #   grammars/                                 <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    TS_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "typescript.grammar")
    COMPILED_GRAMMAR_DIR = __dir__

    # All valid TypeScript grammar versions supported by the versioned grammar
    # files in code/grammars/typescript/.
    VALID_VERSIONS = %w[ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8].freeze

    # Resolve the path to the .grammar file for a given version.
    #
    # When `version` is nil or empty, the generic typescript.grammar is used.
    # When a valid version string is given, the versioned file is returned.
    # An unknown version raises ArgumentError immediately.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .grammar file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_grammar_path(version)
      if version.nil? || version.empty?
        File.join(GRAMMAR_DIR, "typescript.grammar")
      elsif VALID_VERSIONS.include?(version)
        File.join(GRAMMAR_DIR, "typescript", "#{version}.grammar")
      else
        raise ArgumentError,
          "Unknown TypeScript version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
    end

    def self.resolve_compiled_grammar_path(version)
      resolve_grammar_path(version)

      if version.nil? || version.empty?
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        suffix = version.tr(".", "_")
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{suffix}.rb")
      end
    end

    def self.parser_grammar(version)
      CodingAdventures::GrammarTools.load_parser_grammar(resolve_compiled_grammar_path(version))
    end

    # Parse a string of TypeScript source code into a generic AST.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # grammar.  The same version is forwarded to the TypeScript lexer so that
    # both the token grammar and the parser grammar match.  When omitted (or
    # nil), the generic grammars are used — the same behaviour as version 0.1.0.
    #
    # @param source [String] TypeScript source code to parse
    # @param version [String, nil] TypeScript version tag (e.g. "ts5.0") or nil
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.parse(source, version: nil)
      # Step 1: Tokenize using the TypeScript lexer (version-aware)
      tokens = CodingAdventures::TypescriptLexer.tokenize(source, version: version)

      # Step 2: Parse tokens using the compiled grammar for this version.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar(version))
      parser.parse
    end
  end
end
