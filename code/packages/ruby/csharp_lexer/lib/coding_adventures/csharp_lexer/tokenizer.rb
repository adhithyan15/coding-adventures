# frozen_string_literal: true

# ================================================================
# C# Lexer -- Tokenizes C# Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a C#-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the C#
# token definitions from csharp/csharp<version>.tokens.
#
# C# has features that distinguish it from Java and JavaScript:
# - Strong static typing with value and reference semantics
# - `class`, `struct`, `interface`, `enum`, `delegate` for OOP
# - Access modifiers: `public`, `private`, `protected`, `internal`
# - `using`, `namespace` for the module system (vs Java's `import`/`package`)
# - `var` for local type inference (C# 3.0+)
# - `async`/`await` for asynchronous programming (C# 5.0+)
# - LINQ query keywords: `from`, `select`, `where`, `orderby` (C# 3.0+)
# - `nullable` reference types with `?` suffix (C# 8.0+)
# - Records and init-only properties (C# 9.0+)
# - C#-specific operators: `??` (null-coalescing), `?.` (null-conditional),
#   `=>` (lambda / expression body), `::` (namespace alias qualifier)
#
# All of these are handled by the grammar file -- no new code needed.
#
# Version-aware usage
# -------------------
# The lexer supports an optional `version:` keyword argument to select
# a specific C# grammar file from the versioned grammar directory.
#
#   # Default (uses csharp/csharp12.0.tokens)
#   tokens = CodingAdventures::CSharpLexer.tokenize("class Foo { }")
#
#   # C# 8.0 (uses csharp/csharp8.0.tokens)
#   tokens = CodingAdventures::CSharpLexer.tokenize("int x = 1;", version: "8.0")
#
#   # C# 1.0 (uses csharp/csharp1.0.tokens)
#   tokens = CodingAdventures::CSharpLexer.tokenize(source, version: "1.0")
#
# Valid versions:
#   "1.0", "2.0", "3.0", "4.0", "5.0", "6.0", "7.0",
#   "8.0", "9.0", "10.0", "11.0", "12.0"
#
# Pass nil (or omit) for the default grammar (C# 12.0).
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module CSharpLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/csharp_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/csharp_lexer/  <- __dir__
    #   lib/coding_adventures/               <- ..
    #   lib/                                 <- ../..
    #   <gem root>/                          <- ../../..
    #   ruby/                                <- ../../../..
    #   packages/                            <- ../../../../..
    #   code/                                <- ../../../../../..
    #   grammars/                            <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)

    # The default C# version used when no version is specified.
    # C# 12.0 (released with .NET 8, November 2023) is the most recent
    # stable version and therefore the best default.
    DEFAULT_VERSION = "12.0"

    # All valid C# grammar versions supported by the versioned grammar
    # files in code/grammars/csharp/.
    #
    # C# versioning uses dot-separated numbers (unlike Java which uses
    # bare integers for post-1.x releases).  Every version here
    # corresponds to a pair of files:
    #   code/grammars/csharp/csharp<version>.tokens
    #   code/grammars/csharp/csharp<version>.grammar
    VALID_VERSIONS = %w[
      1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0
    ].freeze

    # Resolve the path to the .tokens file for a given C# version.
    #
    # When `version` is nil or empty, the default version (C# 12.0) is used.
    # When a valid version string is given (e.g. "8.0"), the file
    # code/grammars/csharp/csharp<version>.tokens is returned.
    # An unknown version raises ArgumentError immediately so callers get
    # a clear error message rather than a cryptic file-not-found.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .tokens file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_tokens_path(version)
      effective_version = if version.nil? || version.empty?
        DEFAULT_VERSION
      elsif VALID_VERSIONS.include?(version)
        version
      else
        raise ArgumentError,
          "Unknown C# version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
      File.join(GRAMMAR_DIR, "csharp", "csharp#{effective_version}.tokens")
    end

    # Tokenize a string of C# source code into an array of Token objects.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # C# grammar file.  When omitted (or nil), the default C# 12.0
    # grammar is used.
    #
    # @param source [String] C# source code to tokenize
    # @param version [String, nil] C# version tag (e.g. "8.0", "12.0") or nil
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.tokenize(source, version: nil)
      tokens_path = resolve_tokens_path(version)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(tokens_path, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end

    # Alias: tokenize_csharp delegates to tokenize for API parity with the
    # public function name described in the package specification.
    #
    # @param source [String] C# source code to tokenize
    # @param version [String, nil] C# version tag or nil
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize_csharp(source, version: nil)
      tokenize(source, version: version)
    end

    # Create a lexer context for C# source code.
    #
    # Unlike `tokenize`, which eagerly produces the full token list,
    # `create_csharp_lexer` returns a hash describing the configured lexer
    # state.  This is useful when building pipelines or streaming tokenizers
    # where you want to defer actual tokenization.
    #
    # @param source [String] C# source code to tokenize
    # @param version [String, nil] C# version tag (e.g. "8.0", "12.0") or nil
    # @return [Hash] a map with :source, :version, and :language keys
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.create_csharp_lexer(source, version: nil)
      # Validate the version eagerly so callers get immediate feedback
      resolve_tokens_path(version)
      { source: source, version: version, language: :csharp }
    end
  end
end
