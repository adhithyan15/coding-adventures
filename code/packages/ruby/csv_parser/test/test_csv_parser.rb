# frozen_string_literal: true

require "test_helper"

# CsvParserTest — comprehensive minitest suite for CodingAdventures::CsvParser.
#
# Test organisation:
#   1. Basic / happy-path tests  — simple tables, typical usage
#   2. Quoted field tests        — commas, newlines, escaped quotes inside quotes
#   3. Edge case tests           — empty file, header-only, empty fields, trailing newlines
#   4. Ragged row tests          — short rows (padded), long rows (truncated)
#   5. Custom delimiter tests    — TSV, semicolons, pipes
#   6. Error tests               — UnclosedQuoteError
#   7. Whitespace handling       — spaces are significant, not trimmed
#   8. No stdlib usage check     — verify we don't require Ruby's CSV class
#   9. Spec example tests        — verbatim examples from csv-parser.md
class CsvParserTest < Minitest::Test
  # Convenience alias
  CsvParser = CodingAdventures::CsvParser

  # ============================================================================
  # 1. BASIC / HAPPY-PATH TESTS
  # ============================================================================

  def test_simple_two_column_table
    source = "name,age\nAlice,30\nBob,25"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice", "age" => "30"},
      {"name" => "Bob", "age" => "25"}
    ], result
  end

  def test_three_column_table
    # Spec Example 1
    source = "name,age,city\nAlice,30,New York\nBob,25,London"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice", "age" => "30", "city" => "New York"},
      {"name" => "Bob", "age" => "25", "city" => "London"}
    ], result
  end

  def test_single_data_row
    source = "col1,col2\nval1,val2"
    result = CsvParser.parse_csv(source)
    assert_equal [{"col1" => "val1", "col2" => "val2"}], result
  end

  def test_many_rows
    lines = ["a,b"] + (0...100).map { |i| "#{i},#{i * 2}" }
    source = lines.join("\n")
    result = CsvParser.parse_csv(source)
    assert_equal 100, result.length
    assert_equal({"a" => "0", "b" => "0"}, result[0])
    assert_equal({"a" => "99", "b" => "198"}, result[99])
  end

  def test_single_column
    source = "name\nAlice\nBob\nCharlie"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice"},
      {"name" => "Bob"},
      {"name" => "Charlie"}
    ], result
  end

  def test_all_values_are_strings
    # Even numeric-looking values must be returned as strings.
    source = "id,score,flag\n1,3.14,true"
    result = CsvParser.parse_csv(source)
    assert_equal "1", result[0]["id"]
    assert_equal "3.14", result[0]["score"]
    assert_equal "true", result[0]["flag"]
    assert_instance_of String, result[0]["id"]
    assert_instance_of String, result[0]["score"]
  end

  def test_trailing_newline_produces_no_extra_row
    # A source ending with '\n' should NOT produce a spurious empty row.
    source = "a,b\n1,2\n"
    result = CsvParser.parse_csv(source)
    assert_equal 1, result.length
    assert_equal [{"a" => "1", "b" => "2"}], result
  end

  def test_crlf_line_endings
    # Windows-style \r\n line endings.
    source = "name,age\r\nAlice,30\r\nBob,25"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice", "age" => "30"},
      {"name" => "Bob", "age" => "25"}
    ], result
  end

  def test_cr_only_line_endings
    # Old Mac-style bare \r line endings.
    source = "name,age\rAlice,30\rBob,25"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice", "age" => "30"},
      {"name" => "Bob", "age" => "25"}
    ], result
  end

  # ============================================================================
  # 2. QUOTED FIELD TESTS
  # ============================================================================

  def test_quoted_field_with_comma
    # A comma inside a quoted field is NOT a delimiter.
    source = "product,description\nWidget,\"A small, round widget\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"product" => "Widget", "description" => "A small, round widget"}], result
  end

  def test_quoted_field_at_start_of_row
    source = "name,city\n\"Alice Smith\",London"
    result = CsvParser.parse_csv(source)
    assert_equal [{"name" => "Alice Smith", "city" => "London"}], result
  end

  def test_all_fields_quoted
    source = "\"name\",\"age\"\n\"Alice\",\"30\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"name" => "Alice", "age" => "30"}], result
  end

  def test_quoted_field_with_embedded_newline
    # Spec Example 3: a quoted field spanning two physical lines.
    source = "id,note\n1,\"Line one\nLine two\"\n2,Single line"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"id" => "1", "note" => "Line one\nLine two"},
      {"id" => "2", "note" => "Single line"}
    ], result
  end

  def test_quoted_field_with_crlf_inside
    # A \r\n inside a quoted field is preserved literally.
    source = "id,note\n1,\"Line one\r\nLine two\""
    result = CsvParser.parse_csv(source)
    assert_equal "Line one\r\nLine two", result[0]["note"]
  end

  def test_escaped_double_quote
    # Spec Example 4: "" inside a quoted field → one literal "
    source = "id,value\n1,\"She said \"\"hello\"\"\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"id" => "1", "value" => "She said \"hello\""}], result
  end

  def test_escaped_double_quote_multiple
    # Multiple escaped quotes in one field.
    source = "x\n\"a \"\"b\"\" c\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"x" => "a \"b\" c"}], result
  end

  def test_quoted_empty_field
    # An empty quoted field ("") produces an empty string.
    source = "a,b,c\n1,\"\",3"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "1", "b" => "", "c" => "3"}], result
  end

  def test_quoted_field_with_multiple_commas
    source = "name,csv_data\nAlice,\"1,2,3,4,5\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"name" => "Alice", "csv_data" => "1,2,3,4,5"}], result
  end

  def test_quoted_field_in_header
    source = "\"full name\",age\nAlice,30"
    result = CsvParser.parse_csv(source)
    assert_equal [{"full name" => "Alice", "age" => "30"}], result
  end

  # ============================================================================
  # 3. EDGE CASE TESTS
  # ============================================================================

  def test_empty_string
    assert_equal [], CsvParser.parse_csv("")
  end

  def test_header_only
    assert_equal [], CsvParser.parse_csv("name,age,city")
  end

  def test_header_only_with_trailing_newline
    assert_equal [], CsvParser.parse_csv("name,age\n")
  end

  def test_empty_fields_middle
    # Spec Example 5 (partial): empty unquoted field between two delimiters.
    source = "a,b,c\n1,,3"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "1", "b" => "", "c" => "3"}], result
  end

  def test_empty_fields_leading_and_trailing
    source = "a,b,c\n,2,"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "", "b" => "2", "c" => ""}], result
  end

  def test_all_empty_fields
    source = "a,b,c\n,,"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "", "b" => "", "c" => ""}], result
  end

  def test_single_field_single_row
    source = "x\n42"
    result = CsvParser.parse_csv(source)
    assert_equal [{"x" => "42"}], result
  end

  def test_whitespace_is_significant
    # Spaces around unquoted fields are NOT trimmed.
    source = "a,b\n  hello  ,  world  "
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "  hello  ", "b" => "  world  "}], result
  end

  def test_numeric_header_names
    source = "1,2,3\na,b,c"
    result = CsvParser.parse_csv(source)
    assert_equal [{"1" => "a", "2" => "b", "3" => "c"}], result
  end

  # ============================================================================
  # 4. RAGGED ROW TESTS
  # ============================================================================

  def test_short_row_padded_with_empty_strings
    # A row shorter than the header has missing fields filled with "".
    source = "a,b,c\n1,2"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "1", "b" => "2", "c" => ""}], result
  end

  def test_very_short_row
    source = "a,b,c\nonly_a"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "only_a", "b" => "", "c" => ""}], result
  end

  def test_long_row_truncated_to_header_length
    # A row longer than the header has extra fields discarded.
    source = "a,b\n1,2,3,4,5"
    result = CsvParser.parse_csv(source)
    assert_equal [{"a" => "1", "b" => "2"}], result
  end

  def test_mixed_ragged_rows
    source = "a,b,c\n1,2,3\n4,5\n6,7,8,9"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"a" => "1", "b" => "2", "c" => "3"}, # exact
      {"a" => "4", "b" => "5", "c" => ""},  # too short → padded
      {"a" => "6", "b" => "7", "c" => "8"}  # too long → truncated
    ], result
  end

  # ============================================================================
  # 5. CUSTOM DELIMITER TESTS
  # ============================================================================

  def test_tab_delimited_tsv
    # Spec Example 6
    source = "name\tage\nAlice\t30"
    result = CsvParser.parse_csv(source, delimiter: "\t")
    assert_equal [{"name" => "Alice", "age" => "30"}], result
  end

  def test_semicolon_delimiter
    source = "name;age\nAlice;30\nBob;25"
    result = CsvParser.parse_csv(source, delimiter: ";")
    assert_equal [
      {"name" => "Alice", "age" => "30"},
      {"name" => "Bob", "age" => "25"}
    ], result
  end

  def test_pipe_delimiter
    source = "a|b|c\n1|2|3"
    result = CsvParser.parse_csv(source, delimiter: "|")
    assert_equal [{"a" => "1", "b" => "2", "c" => "3"}], result
  end

  def test_custom_delimiter_inside_quotes
    # The custom delimiter inside a quoted field is still literal.
    source = "name;description\nWidget;\"a;b;c\""
    result = CsvParser.parse_csv(source, delimiter: ";")
    assert_equal [{"name" => "Widget", "description" => "a;b;c"}], result
  end

  def test_comma_in_tsv_unquoted_is_literal
    # When delimiter is tab, a comma in an unquoted field is just a character.
    source = "a\tb\nfoo,bar\tbaz"
    result = CsvParser.parse_csv(source, delimiter: "\t")
    assert_equal [{"a" => "foo,bar", "b" => "baz"}], result
  end

  # ============================================================================
  # 6. ERROR TESTS
  # ============================================================================

  def test_unclosed_quote_raises_error
    source = "a,b\n\"unclosed,value"
    assert_raises(CodingAdventures::CsvParser::UnclosedQuoteError) do
      CsvParser.parse_csv(source)
    end
  end

  def test_unclosed_quote_on_first_row
    # Unclosed quote in the header row also raises UnclosedQuoteError.
    source = "\"unclosed header"
    assert_raises(CodingAdventures::CsvParser::UnclosedQuoteError) do
      CsvParser.parse_csv(source)
    end
  end

  def test_unclosed_quote_error_is_argument_error
    # UnclosedQuoteError inherits from ArgumentError (Ruby convention).
    assert_raises(ArgumentError) do
      CsvParser.parse_csv("\"never closed")
    end
  end

  def test_unclosed_quote_error_message
    err = assert_raises(CodingAdventures::CsvParser::UnclosedQuoteError) do
      CsvParser.parse_csv("\"not closed")
    end
    assert_match(/[Uu]nclosed/, err.message)
  end

  # ============================================================================
  # 7. WHITESPACE HANDLING
  # ============================================================================

  def test_leading_spaces_in_unquoted_field
    source = "a\n   value"
    result = CsvParser.parse_csv(source)
    assert_equal "   value", result[0]["a"]
  end

  def test_trailing_spaces_in_unquoted_field
    source = "a\nvalue   "
    result = CsvParser.parse_csv(source)
    assert_equal "value   ", result[0]["a"]
  end

  def test_spaces_inside_quoted_field
    source = "a\n\"  spaced  \""
    result = CsvParser.parse_csv(source)
    assert_equal "  spaced  ", result[0]["a"]
  end

  def test_space_as_entire_field
    source = "a,b\n , "
    result = CsvParser.parse_csv(source)
    assert_equal " ", result[0]["a"]
    assert_equal " ", result[0]["b"]
  end

  # ============================================================================
  # 8. NO STDLIB USAGE CHECK
  # ============================================================================

  def test_parser_does_not_use_ruby_csv_class
    # Read the parser source file and assert there is no 'require "csv"'
    parser_file = File.expand_path(
      "../lib/coding_adventures/csv_parser/parser.rb",
      __dir__
    )
    content = File.read(parser_file)
    refute_match(/require\s+["']csv["']/, content,
      "Parser must not require Ruby's standard library CSV class")
  end

  def test_main_module_does_not_use_ruby_csv_class
    main_file = File.expand_path(
      "../lib/coding_adventures/csv_parser.rb",
      __dir__
    )
    content = File.read(main_file)
    refute_match(/require\s+["']csv["']/, content,
      "Main module must not require Ruby's standard library CSV class")
  end

  # ============================================================================
  # 9. SPEC EXAMPLE TESTS
  # ============================================================================

  def test_spec_example_1_simple_table
    source = "name,age,city\nAlice,30,New York\nBob,25,London"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"name" => "Alice", "age" => "30", "city" => "New York"},
      {"name" => "Bob", "age" => "25", "city" => "London"}
    ], result
  end

  def test_spec_example_2_quoted_with_comma
    source = "product,price,description\n" \
             "Widget,9.99,\"A small, round widget\"\n" \
             "Gadget,19.99,Electronic device"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"product" => "Widget", "price" => "9.99", "description" => "A small, round widget"},
      {"product" => "Gadget", "price" => "19.99", "description" => "Electronic device"}
    ], result
  end

  def test_spec_example_3_quoted_with_newline
    source = "id,note\n1,\"Line one\nLine two\"\n2,Single line"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"id" => "1", "note" => "Line one\nLine two"},
      {"id" => "2", "note" => "Single line"}
    ], result
  end

  def test_spec_example_4_escaped_quote
    source = "id,value\n1,\"She said \"\"hello\"\"\""
    result = CsvParser.parse_csv(source)
    assert_equal [{"id" => "1", "value" => "She said \"hello\""}], result
  end

  def test_spec_example_5_empty_fields
    source = "a,b,c\n1,,3\n,2,"
    result = CsvParser.parse_csv(source)
    assert_equal [
      {"a" => "1", "b" => "", "c" => "3"},
      {"a" => "", "b" => "2", "c" => ""}
    ], result
  end

  def test_spec_example_6_tsv
    source = "name\tage\nAlice\t30"
    result = CsvParser.parse_csv(source, delimiter: "\t")
    assert_equal [{"name" => "Alice", "age" => "30"}], result
  end
end
