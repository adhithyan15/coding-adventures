# frozen_string_literal: true

# ================================================================
# Java Parser -- Parses Java Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the java_lexer pattern: instead of writing
# a Java-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the Java grammar from java/java<version>.grammar.
#
# The pipeline is:
#
#   1. Read java/java<version>.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read java/java<version>.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Version-aware usage
# -------------------
# Both the lexer and parser steps can target a specific Java version:
#
#   # Default (uses java/java21.grammar + java/java21.tokens)
#   ast = CodingAdventures::JavaParser.parse("int x = 1 + 2;")
#
#   # Java 8 (uses java/java8.grammar + java/java8.tokens)
#   ast = CodingAdventures::JavaParser.parse("int x = 1;", version: "8")
#
# Valid versions: "1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21"
# Pass nil (or omit) for the default grammar (Java 21).
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_java_lexer"

module CodingAdventures
  module JavaParser
    # Paths to the grammar files, computed relative to this file.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/java_parser/  <- __dir__
    #   lib/coding_adventures/              <- ..
    #   lib/                                <- ../..
    #   <gem root>/                         <- ../../..
    #   ruby/                               <- ../../../..
    #   packages/                           <- ../../../../..
    #   code/                               <- ../../../../../..
    #   grammars/                           <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    # The default Java version used when no version is specified.
    DEFAULT_VERSION = "21"

    # All valid Java grammar versions supported by the versioned grammar
    # files in code/grammars/java/.
    VALID_VERSIONS = %w[
      1.0 1.1 1.4 5 7 8 10 14 17 21
    ].freeze

    # Resolve the path to the .grammar file for a given Java version.
    #
    # When `version` is nil or empty, the default version (Java 21) is used.
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
          "Unknown Java version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
      File.join(GRAMMAR_DIR, "java", "java#{effective_version}.grammar")
    end

    def self.resolve_compiled_grammar_path(version)
      effective_version = if version.nil? || version.empty?
        DEFAULT_VERSION
      else
        resolve_grammar_path(version)
        version
      end

      if effective_version == DEFAULT_VERSION && (version.nil? || version.empty?)
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        suffix = effective_version.tr(".", "_")
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{suffix}.rb")
      end
    end

    def self.parser_grammar(version)
      CodingAdventures::GrammarTools.load_parser_grammar(resolve_compiled_grammar_path(version))
    end

    # Parse a string of Java source code into a generic AST.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # Java grammar.  The same version is forwarded to the Java
    # lexer so that both the token grammar and the parser grammar match.
    # When omitted (or nil), the default grammars are used (Java 21).
    #
    # @param source [String] Java source code to parse
    # @param version [String, nil] Java version tag (e.g. "8", "17") or nil
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.parse(source, version: nil)
      # Step 1: Tokenize using the Java lexer (version-aware)
      tokens = CodingAdventures::JavaLexer.tokenize(source, version: version)

      # Step 2: Parse tokens using the compiled grammar for this version.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar(version))
      parser.parse
    end

    # Create a parser context for Java source code.
    #
    # Unlike `parse`, which eagerly produces the full AST,
    # `create_parser` returns a hash describing the configured parser state.
    # This is useful when building pipelines or deferred parsing workflows.
    #
    # @param source [String] Java source code to parse
    # @param version [String, nil] Java version tag (e.g. "8", "17") or nil
    # @return [Hash] a map with :source, :version, and :language keys
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.create_parser(source, version: nil)
      # Validate the version eagerly so callers get immediate feedback
      resolve_grammar_path(version)
      { source: source, version: version, language: :java }
    end
  end
end
