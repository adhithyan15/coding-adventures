# frozen_string_literal: true

# ================================================================
# TypeScript Lexer -- Tokenizes TypeScript Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a TypeScript-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the TypeScript
# token definitions from typescript.tokens.
#
# TypeScript extends JavaScript with additional features:
# - `interface`, `type`, `enum`, `namespace`, `declare` keywords
# - Type annotation keywords: `number`, `string`, `boolean`, etc.
# - `readonly`, `abstract`, `implements` keywords
# - All JavaScript features carry over
#
# All of these are handled by the grammar file — no new code needed.
#
# Version-aware usage
# -------------------
# The lexer supports an optional `version:` keyword argument to select
# a specific TypeScript grammar file from the versioned grammar directory.
#
#   # Generic (uses typescript.tokens)
#   tokens = CodingAdventures::TypescriptLexer.tokenize("let x: number = 1 + 2;")
#
#   # TypeScript 5.0 (uses typescript/ts5.0.tokens)
#   tokens = CodingAdventures::TypescriptLexer.tokenize("let x: number = 1;", version: "ts5.0")
#
#   # TypeScript 5.8 (uses typescript/ts5.8.tokens)
#   tokens = CodingAdventures::TypescriptLexer.tokenize(source, version: "ts5.8")
#
# Valid versions: "ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"
# Pass nil (or omit) for the generic grammar.
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module TypescriptLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/typescript_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/typescript_lexer/  <- __dir__
    #   lib/coding_adventures/                   <- ..
    #   lib/                                     <- ../..
    #   <gem root>/                              <- ../../..
    #   ruby/                                    <- ../../../..
    #   packages/                                <- ../../../../..
    #   code/                                    <- ../../../../../..
    #   grammars/                                <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    # The generic typescript.tokens file (no version qualifier).
    TS_TOKENS_PATH = File.join(GRAMMAR_DIR, "typescript.tokens")

    # All valid TypeScript grammar versions supported by the versioned grammar
    # files in code/grammars/typescript/.  Versions correspond to official
    # TypeScript releases; "ts5.8" is the latest at time of writing.
    VALID_VERSIONS = %w[ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8].freeze

    # Resolve the path to the .tokens file for a given version.
    #
    # When `version` is nil or empty, the generic grammar is used.
    # When a valid version string is given (e.g. "ts5.0"), the file
    # code/grammars/typescript/<version>.tokens is returned.
    # An unknown version raises ArgumentError immediately so callers get
    # a clear error message rather than a cryptic file-not-found.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .tokens file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_tokens_path(version)
      if version.nil? || version.empty?
        File.join(GRAMMAR_DIR, "typescript.tokens")
      elsif VALID_VERSIONS.include?(version)
        File.join(GRAMMAR_DIR, "typescript", "#{version}.tokens")
      else
        raise ArgumentError,
          "Unknown TypeScript version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
    end

    def self.resolve_compiled_tokens_path(version)
      resolve_tokens_path(version)

      if version.nil? || version.empty?
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        suffix = version.tr(".", "_")
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{suffix}.rb")
      end
    end

    def self.token_grammar(version)
      CodingAdventures::GrammarTools.load_token_grammar(resolve_compiled_tokens_path(version))
    end

    # Tokenize a string of TypeScript source code into an array of Token objects.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # grammar file.  When omitted (or nil), the generic `typescript.tokens`
    # grammar is used — the same behaviour as version 0.1.0.
    #
    # @param source [String] TypeScript source code to tokenize
    # @param version [String, nil] TypeScript version tag (e.g. "ts5.0") or nil
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.tokenize(source, version: nil)
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, token_grammar(version))
      lexer.tokenize
    end
  end
end
