# frozen_string_literal: true

require_relative "test_helper"

class NibParserTest < Minitest::Test
  def test_parses_simple_program
    ast = CodingAdventures::NibParser.parse_nib("fn main() { return 0; }")
    assert_equal "program", ast.rule_name
    refute_empty ast.children
  end

  def test_parses_loop_construct
    ast = CodingAdventures::NibParser.parse_nib("fn main() { for i: u4 in 0..4 { return i; } }")
    assert_equal "program", ast.rule_name
  end
end
