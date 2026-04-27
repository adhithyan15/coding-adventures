# frozen_string_literal: true

# ================================================================
# JavaScript Lexer -- Tokenizes JavaScript Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a JavaScript-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the JavaScript
# token definitions from javascript.tokens.
#
# JavaScript has features that Python and Ruby do not:
# - `let`, `const`, `var` for variable declarations
# - `===` and `!==` for strict equality
# - Semicolons terminate statements
# - Curly braces `{}` for blocks
# - `$` is valid in identifiers
# - `=>` for arrow functions
#
# All of these are handled by the grammar file — no new code needed.
#
# Version-aware usage
# -------------------
# The lexer supports an optional `version:` keyword argument to select
# a specific ECMAScript grammar file from the versioned grammar directory.
#
#   # Generic (uses javascript.tokens)
#   tokens = CodingAdventures::JavascriptLexer.tokenize("let x = 1 + 2;")
#
#   # ES2020 (uses ecmascript/es2020.tokens)
#   tokens = CodingAdventures::JavascriptLexer.tokenize("let x = 1;", version: "es2020")
#
#   # ES5 (uses ecmascript/es5.tokens)
#   tokens = CodingAdventures::JavascriptLexer.tokenize(source, version: "es5")
#
# Valid versions: "es1", "es3", "es5", "es2015", "es2016", "es2017",
#   "es2018", "es2019", "es2020", "es2021", "es2022", "es2023", "es2024", "es2025"
# Pass nil (or omit) for the generic grammar.
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module JavascriptLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/javascript_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/javascript_lexer/  <- __dir__
    #   lib/coding_adventures/                   <- ..
    #   lib/                                     <- ../..
    #   <gem root>/                              <- ../../..
    #   ruby/                                    <- ../../../..
    #   packages/                                <- ../../../../..
    #   code/                                    <- ../../../../../..
    #   grammars/                                <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    # The generic javascript.tokens file (no version qualifier).
    JS_TOKENS_PATH = File.join(GRAMMAR_DIR, "javascript.tokens")

    # All valid ECMAScript grammar versions supported by the versioned grammar
    # files in code/grammars/ecmascript/.  Covers ES1 through ES2025.
    VALID_VERSIONS = %w[
      es1 es3 es5
      es2015 es2016 es2017 es2018 es2019 es2020
      es2021 es2022 es2023 es2024 es2025
    ].freeze

    # Resolve the path to the .tokens file for a given ECMAScript version.
    #
    # When `version` is nil or empty, the generic grammar is used.
    # When a valid version string is given (e.g. "es2020"), the file
    # code/grammars/ecmascript/<version>.tokens is returned.
    # An unknown version raises ArgumentError immediately so callers get
    # a clear error message rather than a cryptic file-not-found.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .tokens file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_tokens_path(version)
      if version.nil? || version.empty?
        File.join(GRAMMAR_DIR, "javascript.tokens")
      elsif VALID_VERSIONS.include?(version)
        File.join(GRAMMAR_DIR, "ecmascript", "#{version}.tokens")
      else
        raise ArgumentError,
          "Unknown JavaScript/ECMAScript version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
    end

    def self.resolve_compiled_tokens_path(version)
      resolve_tokens_path(version)

      if version.nil? || version.empty?
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{version}.rb")
      end
    end

    def self.token_grammar(version)
      CodingAdventures::GrammarTools.load_token_grammar(resolve_compiled_tokens_path(version))
    end

    # Tokenize a string of JavaScript source code into an array of Token objects.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # ECMAScript grammar file.  When omitted (or nil), the generic
    # `javascript.tokens` grammar is used — the same behaviour as version 0.1.0.
    #
    # @param source [String] JavaScript source code to tokenize
    # @param version [String, nil] ECMAScript version tag (e.g. "es2020") or nil
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.tokenize(source, version: nil)
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, token_grammar(version))
      lexer.tokenize
    end
  end
end
