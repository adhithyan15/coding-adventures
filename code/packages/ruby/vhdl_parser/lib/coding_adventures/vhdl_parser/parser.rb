# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_vhdl_lexer"

module CodingAdventures
  module VhdlParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    COMPILED_GRAMMAR_DIR = __dir__
    DEFAULT_VERSION = CodingAdventures::VhdlLexer::DEFAULT_VERSION
    SUPPORTED_VERSIONS = CodingAdventures::VhdlLexer::SUPPORTED_VERSIONS

    VHDL_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "vhdl", "vhdl2008.grammar")

    def self.resolve_version(version = nil)
      CodingAdventures::VhdlLexer.resolve_version(version)
    end

    def self.resolve_grammar_path(version = nil)
      File.join(GRAMMAR_DIR, "vhdl", "vhdl#{resolve_version(version)}.grammar")
    end

    def self.resolve_compiled_grammar_path(version = nil)
      resolved = resolve_version(version)
      File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{resolved}.rb")
    end

    def self.parser_grammar(version = nil)
      CodingAdventures::GrammarTools.load_parser_grammar(resolve_compiled_grammar_path(version))
    end

    def self.parse(source, version: nil)
      tokens = CodingAdventures::VhdlLexer.tokenize(source, version: version)
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(
        tokens,
        parser_grammar(version)
      )
      parser.parse
    end
  end
end
