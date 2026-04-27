# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module VhdlLexer
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__
    DEFAULT_VERSION = "2008"
    SUPPORTED_VERSIONS = %w[1987 1993 2002 2008 2019].freeze

    VHDL_TOKENS_PATH = File.join(GRAMMAR_DIR, "vhdl", "vhdl2008.tokens")

    def self.resolve_version(version = nil)
      resolved = version.nil? || version.empty? ? DEFAULT_VERSION : version
      return resolved if SUPPORTED_VERSIONS.include?(resolved)

      raise ArgumentError,
        "Unknown VHDL version #{resolved.inspect}. " \
        "Valid versions: #{SUPPORTED_VERSIONS.sort.join(", ")}"
    end

    def self.resolve_tokens_path(version = nil)
      File.join(GRAMMAR_DIR, "vhdl", "vhdl#{resolve_version(version)}.tokens")
    end

    def self.resolve_compiled_tokens_path(version = nil)
      resolved = resolve_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{resolved}.rb")
    end

    def self.token_grammar(version = nil)
      CodingAdventures::GrammarTools.load_token_grammar(resolve_compiled_tokens_path(version))
    end

    def self.tokenize(source, version: nil)
      grammar = token_grammar(version)
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      raw_tokens = lexer.tokenize
      keyword_set = {}
      grammar.keywords.each { |keyword| keyword_set[keyword] = true }

      raw_tokens.map do |token|
        if token.type == Lexer::TokenType::NAME || token.type == Lexer::TokenType::KEYWORD
          downcased = token.value.downcase
          new_type = keyword_set[downcased] ? Lexer::TokenType::KEYWORD : Lexer::TokenType::NAME
          Lexer::Token.new(
            type: new_type,
            value: downcased,
            line: token.line,
            column: token.column
          )
        else
          token
        end
      end
    end
  end
end
