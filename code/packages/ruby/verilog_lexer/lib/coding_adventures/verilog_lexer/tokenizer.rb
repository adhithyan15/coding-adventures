# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module VerilogLexer
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__
    DEFAULT_VERSION = "2005"
    SUPPORTED_VERSIONS = %w[1995 2001 2005].freeze

    VERILOG_TOKENS_PATH = File.join(GRAMMAR_DIR, "verilog", "verilog2005.tokens")

    def self.resolve_version(version = nil)
      resolved = version.nil? || version.empty? ? DEFAULT_VERSION : version
      return resolved if SUPPORTED_VERSIONS.include?(resolved)

      raise ArgumentError,
        "Unknown Verilog version #{resolved.inspect}. " \
        "Valid versions: #{SUPPORTED_VERSIONS.sort.join(", ")}"
    end

    def self.resolve_tokens_path(version = nil)
      File.join(GRAMMAR_DIR, "verilog", "verilog#{resolve_version(version)}.tokens")
    end

    def self.resolve_compiled_tokens_path(version = nil)
      resolved = resolve_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{resolved}.rb")
    end

    def self.token_grammar(version = nil)
      CodingAdventures::GrammarTools.load_token_grammar(resolve_compiled_tokens_path(version))
    end

    def self.tokenize(source, preprocess: false, version: nil)
      processed_source = preprocess ? Preprocessor.process(source) : source
      lexer = CodingAdventures::Lexer::GrammarLexer.new(
        processed_source,
        token_grammar(version)
      )
      lexer.tokenize
    end
  end
end
