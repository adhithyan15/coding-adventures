# frozen_string_literal: true

# ================================================================
# Python Lexer -- Tokenizes Python Source Code from Ruby
# ================================================================
#
# This module demonstrates the power of the grammar-driven approach
# to language tooling. Instead of writing a Python-specific lexer
# from scratch, we reuse the general-purpose GrammarLexer engine
# from the coding_adventures_lexer gem, feeding it the Python token
# definitions from versioned .tokens files.
#
# The insight is simple but profound: the same lexer code that
# tokenizes one language can tokenize any language, as long as you
# provide the right grammar file. This is exactly how tools like
# Tree-sitter and TextMate grammars work -- the engine is fixed,
# and the grammar is the variable.
#
# Donald Knuth called this "separation of concerns" -- the lexer
# engine knows *how* to tokenize (the algorithm), while the .tokens
# file knows *what* to tokenize (the language-specific patterns).
#
# Versioned grammars
# ------------------
#
# Python's token set has evolved across versions. Python 2.7 has
# print as a keyword; Python 3.0 removed it. Python 3.6 added
# f-strings. Python 3.8 added the walrus operator (:=). Python
# 3.10 added soft keywords (match, case). Python 3.12 added the
# type soft keyword.
#
# Each version has its own grammar file:
#
#   code/grammars/python/python2.7.tokens
#   code/grammars/python/python3.0.tokens
#   code/grammars/python/python3.6.tokens
#   code/grammars/python/python3.8.tokens
#   code/grammars/python/python3.10.tokens
#   code/grammars/python/python3.12.tokens
#
# The lexer loads the grammar for the requested version and caches
# the parsed result so subsequent calls with the same version skip
# the file I/O and parsing overhead.
#
# Usage:
#   tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2")
#   tokens = CodingAdventures::PythonLexer.tokenize("x = 1 + 2", version: "3.8")
#   tokens = CodingAdventures::PythonLexer.tokenize("print 1", version: "2.7")
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module PythonLexer
    # Standalone callers default to the latest supported version.
    DEFAULT_VERSION = "3.12"

    # All Python versions with grammar files in the repository.
    SUPPORTED_VERSIONS = %w[2.7 3.0 3.6 3.8 3.10 3.12].freeze

    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/python_lexer/ to the
    # repository root's code/grammars/ directory.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    # Per-version grammar cache. Once a grammar is parsed it is stored
    # here and reused for all subsequent calls with that version. This
    # avoids re-reading and re-parsing the .tokens file on every call.
    #
    # Thread safety: Ruby's GIL makes Hash#[]= atomic for simple
    # assignments. Even without it, the worst case is a harmless
    # double-parse -- the grammar is immutable once constructed.
    @grammar_cache = {}

    # Build the path to the versioned .tokens file for a given version.
    def self.grammar_path(version)
      normalized_version = normalize_version(version)
      File.join(GRAMMAR_DIR, "python", "python#{normalized_version}.tokens")
    end

    def self.compiled_grammar_path(version)
      normalized_version = normalize_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{normalized_version.tr(".", "_")}.rb")
    end

    # Load and cache the parsed TokenGrammar for the given version.
    #
    # @param version [String, nil] Python version string (e.g. "3.12") or nil
    # @return [CodingAdventures::GrammarTools::TokenGrammar]
    # @raise [ArgumentError] if the version is not supported
    def self.load_grammar(version)
      normalized_version = normalize_version(version)
      return @grammar_cache[normalized_version] if @grammar_cache.key?(normalized_version)

      unless SUPPORTED_VERSIONS.include?(normalized_version)
        raise ArgumentError,
          "Unsupported Python version: #{version.inspect}. " \
          "Supported versions: #{SUPPORTED_VERSIONS.join(", ")}"
      end

      path = grammar_path(normalized_version)
      raise ArgumentError, "Missing Python grammar file: #{path}" unless File.exist?(path)

      grammar = CodingAdventures::GrammarTools.load_token_grammar(compiled_grammar_path(normalized_version))
      @grammar_cache[normalized_version] = grammar
      grammar
    end

    # Tokenize a string of Python source code into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Resolves the version (defaults to DEFAULT_VERSION)
    # 2. Loads the versioned grammar (cached after first load)
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # @param source [String] Python source code to tokenize
    # @param version [String, nil] Python version or nil for DEFAULT_VERSION
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source, version: DEFAULT_VERSION)
      grammar = load_grammar(version)
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end

    # Clear the grammar cache. Primarily useful for testing.
    def self.clear_cache!
      @grammar_cache.clear
      CodingAdventures::GrammarTools.clear_compiled_grammar_cache!
    end

    def self.normalize_version(version)
      return DEFAULT_VERSION if version.nil? || version.empty?

      version
    end
  end
end
