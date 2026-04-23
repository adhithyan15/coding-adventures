# frozen_string_literal: true

require_relative "test_helper"

class NibLexerTest < Minitest::Test
  def tokens(source)
    CodingAdventures::NibLexer.tokenize_nib(source)
  end

  def test_tokenizes_function_declaration
    assert_equal(
      %w[KEYWORD NAME LPAREN RPAREN LBRACE KEYWORD INT_LIT SEMICOLON RBRACE EOF],
      tokens("fn main() { return 0; }").map(&:type_name)
    )
  end

  def test_prefers_multicharacter_operators
    assert_equal(
      ["1", "+%", "2", "+?", "3", ""],
      tokens("1 +% 2 +? 3").map(&:value)
    )
  end
end
