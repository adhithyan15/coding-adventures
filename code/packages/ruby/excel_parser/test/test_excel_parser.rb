# frozen_string_literal: true

require_relative "test_helper"

class TestExcelParser < Minitest::Test
  def parse(source)
    CodingAdventures::ExcelParser.parse(source)
  end

  def test_parse_function_formula
    assert_equal "formula", parse("=SUM(A1:B2)").rule_name
  end

  def test_parse_column_range
    assert_equal "formula", parse("A:C").rule_name
  end

  def test_parse_row_range
    assert_equal "formula", parse("1:3").rule_name
  end

  def test_factory_exists
    parser = CodingAdventures::ExcelParser.create_excel_parser("A1")
    assert_equal "formula", parser.parse.rule_name
  end
end
