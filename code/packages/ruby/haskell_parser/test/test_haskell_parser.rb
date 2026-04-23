# frozen_string_literal: true

require_relative "test_helper"

class TestHaskellParser < Minitest::Test
  def test_parser_uses_file_root
    ast = CodingAdventures::HaskellParser.parse("x")
    assert_equal "file", ast.rule_name
  end

  def test_versioned_parse_uses_selected_grammar
    ast = CodingAdventures::HaskellParser.parse("x", version: "98")
    assert_equal "file", ast.rule_name
  end
end
