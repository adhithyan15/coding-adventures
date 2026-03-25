# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_excel_lexer"

module CodingAdventures
  module ExcelParser
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    EXCEL_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "excel.grammar")

    def self.previous_significant_token(tokens, index)
      (index - 1).downto(0) do |i|
        return tokens[i] unless tokens[i].type_name == "SPACE"
      end
      nil
    end

    def self.next_significant_token(tokens, index)
      (index + 1).upto(tokens.length - 1) do |i|
        return tokens[i] unless tokens[i].type_name == "SPACE"
      end
      nil
    end

    def self.normalize_excel_reference_tokens(tokens)
      tokens.each_with_index.map do |token, index|
        next token unless %w[NAME NUMBER].include?(token.type_name)

        previous = previous_significant_token(tokens, index)
        following = next_significant_token(tokens, index)
        adjacent_to_colon =
          previous&.type_name == "COLON" || following&.type_name == "COLON"

        if token.type_name == "NAME" && adjacent_to_colon
          next CodingAdventures::Lexer::Token.new(
            type: "COLUMN_REF",
            value: token.value,
            line: token.line,
            column: token.column
          )
        end

        if token.type_name == "NUMBER" && adjacent_to_colon
          next CodingAdventures::Lexer::Token.new(
            type: "ROW_REF",
            value: token.value,
            line: token.line,
            column: token.column
          )
        end

        token
      end
    end

    def self.create_excel_parser(source)
      tokens = CodingAdventures::ExcelLexer.tokenize(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(EXCEL_GRAMMAR_PATH, encoding: "UTF-8")
      )
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.add_pre_parse(method(:normalize_excel_reference_tokens).to_proc)
      parser
    end

    def self.parse(source)
      create_excel_parser(source).parse
    end
  end
end
