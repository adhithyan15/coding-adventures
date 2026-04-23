# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_haskell_lexer"

module CodingAdventures
  module HaskellParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__

    DEFAULT_VERSION = "2010"
    VALID_VERSIONS = %w[
      1.0 1.1 1.2 1.3 1.4 98 2010
    ].freeze

    def self.resolve_grammar_path(version)
      effective_version = if version.nil? || version.empty?
        DEFAULT_VERSION
      elsif VALID_VERSIONS.include?(version)
        version
      else
        raise ArgumentError,
          "Unknown Haskell version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end

      File.join(GRAMMAR_DIR, "haskell", "haskell#{effective_version}.grammar")
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

    def self.parse(source, version: nil)
      tokens = CodingAdventures::HaskellLexer.tokenize(source, version: version)
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar(version))
      parser.parse
    end

    def self.create_parser(source, version: nil)
      resolve_grammar_path(version)
      { source: source, version: version, language: :haskell }
    end
  end
end
