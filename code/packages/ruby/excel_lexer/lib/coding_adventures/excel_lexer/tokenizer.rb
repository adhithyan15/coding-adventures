# frozen_string_literal: true

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module ExcelLexer
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    EXCEL_TOKENS_PATH = File.join(GRAMMAR_DIR, "excel.tokens")

    def self.next_non_space_char(ctx)
      offset = 1
      loop do
        ch = ctx.peek(offset)
        return ch if ch.empty? || ch != " "
        offset += 1
      end
    end

    EXCEL_ON_TOKEN = proc do |token, ctx|
      next unless token.type_name == "NAME"

      next_char = next_non_space_char(ctx)
      if next_char == "("
        ctx.suppress
        ctx.emit(
          CodingAdventures::Lexer::Token.new(
            type: "FUNCTION_NAME",
            value: token.value,
            line: token.line,
            column: token.column
          )
        )
        next
      end

      next unless next_char == "["

      ctx.suppress
      ctx.emit(
        CodingAdventures::Lexer::Token.new(
          type: "TABLE_NAME",
          value: token.value,
          line: token.line,
          column: token.column
        )
      )
    end

    def self.create_excel_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(EXCEL_TOKENS_PATH, encoding: "UTF-8")
      )
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.set_on_token(EXCEL_ON_TOKEN)
      lexer
    end

    def self.tokenize(source)
      create_excel_lexer(source).tokenize
    end
  end
end
