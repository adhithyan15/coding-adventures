# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_verilog_lexer"

module CodingAdventures
  module VerilogParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__
    DEFAULT_VERSION = CodingAdventures::VerilogLexer::DEFAULT_VERSION
    SUPPORTED_VERSIONS = CodingAdventures::VerilogLexer::SUPPORTED_VERSIONS

    VERILOG_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "verilog", "verilog2005.grammar")

    def self.resolve_version(version = nil)
      CodingAdventures::VerilogLexer.resolve_version(version)
    end

    def self.resolve_grammar_path(version = nil)
      File.join(GRAMMAR_DIR, "verilog", "verilog#{resolve_version(version)}.grammar")
    end

    def self.resolve_compiled_grammar_path(version = nil)
      resolved = resolve_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{resolved}.rb")
    end

    def self.parser_grammar(version = nil)
      CodingAdventures::GrammarTools.load_parser_grammar(resolve_compiled_grammar_path(version))
    end

    def self.parse(source, preprocess: false, version: nil)
      tokens = CodingAdventures::VerilogLexer.tokenize(
        source,
        preprocess: preprocess,
        version: version
      )
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(
        tokens,
        parser_grammar(version)
      )
      parser.parse
    end
  end
end
