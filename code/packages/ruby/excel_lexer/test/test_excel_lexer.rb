# frozen_string_literal: true

require_relative "test_helper"

class TestExcelLexer < Minitest::Test
  def tokenize(source)
    CodingAdventures::ExcelLexer.tokenize(source)
  end

  def test_function_name_reclassification
    tokens = tokenize("=SUM(A1)")
    assert_equal "FUNCTION_NAME", tokens[1].type_name
    assert_equal "SUM", tokens[1].value
  end

  def test_table_name_reclassification
    tokens = tokenize("DeptSales[Sales Amount]")
    assert_equal "TABLE_NAME", tokens[0].type_name
    assert_equal "DeptSales", tokens[0].value
  end

  def test_factory_exists
    lexer = CodingAdventures::ExcelLexer.create_excel_lexer("A1")
    assert_respond_to lexer, :tokenize
  end
end
