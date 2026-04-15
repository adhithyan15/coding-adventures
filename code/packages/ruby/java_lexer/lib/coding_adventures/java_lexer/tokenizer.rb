# frozen_string_literal: true

# ================================================================
# Java Lexer -- Tokenizes Java Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a Java-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the Java
# token definitions from java/java<version>.tokens.
#
# Java has features that distinguish it from JavaScript:
# - Strong static typing: `int`, `double`, `boolean`, `char`, etc.
# - Access modifiers: `public`, `private`, `protected`
# - `class`, `interface`, `extends`, `implements` for OOP
# - `package` and `import` for module system
# - `throws`, `try`, `catch`, `finally` for exception handling
# - Annotations with `@`
#
# All of these are handled by the grammar file -- no new code needed.
#
# Version-aware usage
# -------------------
# The lexer supports an optional `version:` keyword argument to select
# a specific Java grammar file from the versioned grammar directory.
#
#   # Default (uses java/java21.tokens)
#   tokens = CodingAdventures::JavaLexer.tokenize("int x = 1 + 2;")
#
#   # Java 8 (uses java/java8.tokens)
#   tokens = CodingAdventures::JavaLexer.tokenize("int x = 1;", version: "8")
#
#   # Java 1.0 (uses java/java1.0.tokens)
#   tokens = CodingAdventures::JavaLexer.tokenize(source, version: "1.0")
#
# Valid versions: "1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21"
# Pass nil (or omit) for the default grammar (Java 21).
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module JavaLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/java_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/java_lexer/  <- __dir__
    #   lib/coding_adventures/             <- ..
    #   lib/                               <- ../..
    #   <gem root>/                        <- ../../..
    #   ruby/                              <- ../../../..
    #   packages/                          <- ../../../../..
    #   code/                              <- ../../../../../..
    #   grammars/                          <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    # The default Java version used when no version is specified.
    DEFAULT_VERSION = "21"

    # All valid Java grammar versions supported by the versioned grammar
    # files in code/grammars/java/.
    VALID_VERSIONS = %w[
      1.0 1.1 1.4 5 7 8 10 14 17 21
    ].freeze

    # Resolve the path to the .tokens file for a given Java version.
    #
    # When `version` is nil or empty, the default version (Java 21) is used.
    # When a valid version string is given (e.g. "8"), the file
    # code/grammars/java/java<version>.tokens is returned.
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
          "Unknown Java version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
      File.join(GRAMMAR_DIR, "java", "java#{effective_version}.tokens")
    end

    def self.resolve_compiled_tokens_path(version)
      effective_version = if version.nil? || version.empty?
        DEFAULT_VERSION
      else
        resolve_tokens_path(version)
        version
      end

      if effective_version == DEFAULT_VERSION && (version.nil? || version.empty?)
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        suffix = effective_version.tr(".", "_")
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{suffix}.rb")
      end
    end

    def self.token_grammar(version)
      CodingAdventures::GrammarTools.load_token_grammar(resolve_compiled_tokens_path(version))
    end

    # Tokenize a string of Java source code into an array of Token objects.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # Java grammar file.  When omitted (or nil), the default Java 21
    # grammar is used.
    #
    # @param source [String] Java source code to tokenize
    # @param version [String, nil] Java version tag (e.g. "8", "17") or nil
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.tokenize(source, version: nil)
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, token_grammar(version))
      lexer.tokenize
    end

    # Create a lexer context for Java source code.
    #
    # Unlike `tokenize`, which eagerly produces the full token list,
    # `create_lexer` returns a hash describing the configured lexer state.
    # This is useful when building pipelines or streaming tokenizers.
    #
    # @param source [String] Java source code to tokenize
    # @param version [String, nil] Java version tag (e.g. "8", "17") or nil
    # @return [Hash] a map with :source, :version, and :language keys
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.create_lexer(source, version: nil)
      # Validate the version eagerly so callers get immediate feedback
      resolve_tokens_path(version)
      { source: source, version: version, language: :java }
    end
  end
end
