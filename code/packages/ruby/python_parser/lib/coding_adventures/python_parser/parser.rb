# frozen_string_literal: true

# ================================================================
# Python Parser -- Parses Python Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the python_lexer pattern: instead of writing
# a Python-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it versioned Python grammars.
#
# The pipeline is:
#
#   1. Read python{version}.tokens -> GrammarLexer -> tokens
#   2. Read python{version}.grammar -> GrammarDrivenParser -> AST
#
# The same two grammar files that describe Python's syntax are all
# that's needed to go from raw source code to an Abstract Syntax Tree.
# No Python-specific code is involved in the parsing itself -- just
# data files and generic engines.
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::PythonParser.parse("x = 1 + 2")
#   ast = CodingAdventures::PythonParser.parse('print "hello"', version: "2.7")
#   # => ASTNode(...)
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_python_lexer"

module CodingAdventures
  module PythonParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    DEFAULT_VERSION = CodingAdventures::PythonLexer::DEFAULT_VERSION
    SUPPORTED_VERSIONS = CodingAdventures::PythonLexer::SUPPORTED_VERSIONS
    COMPILED_GRAMMAR_DIR = __dir__

    def self.grammar_path(version)
      normalized_version = normalize_version(version)
      File.join(GRAMMAR_DIR, "python", "python#{normalized_version}.grammar")
    end

    def self.compiled_grammar_path(version)
      normalized_version = normalize_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{normalized_version.tr(".", "_")}.rb")
    end

    def self.parser_grammar(version)
      normalized_version = normalize_version(version)

      unless SUPPORTED_VERSIONS.include?(normalized_version)
        raise ArgumentError,
          "Unsupported Python version: #{version.inspect}. " \
          "Supported versions: #{SUPPORTED_VERSIONS.join(", ")}"
      end

      path = grammar_path(normalized_version)
      raise ArgumentError, "Missing Python grammar file: #{path}" unless File.exist?(path)

      CodingAdventures::GrammarTools.load_parser_grammar(compiled_grammar_path(normalized_version))
    end

    # Parse a string of Python source code into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Resolves the requested Python version (defaults to 3.12)
    # 2. Tokenizes with the matching versioned Python lexer grammar
    # 3. Loads the matching compiled parser grammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # @param source [String] Python source code to parse
    # @param version [String, nil] Python version or nil for DEFAULT_VERSION
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source, version: DEFAULT_VERSION)
      normalized_version = normalize_version(version)
      tokens = CodingAdventures::PythonLexer.tokenize(source, version: normalized_version)

      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar(normalized_version))
      parser.parse
    end

    def self.normalize_version(version)
      return DEFAULT_VERSION if version.nil? || version.empty?

      version
    end
  end
end
