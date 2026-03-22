# frozen_string_literal: true

# test_nl.rb -- Tests for the Ruby nl tool
# ==========================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../nl_tool"

module NlTestHelper
  NL_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "nl.json")

  def parse_nl_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(NL_TEST_SPEC, ["nl"] + argv).parse
  end
end

class TestNlCliIntegration < Minitest::Test
  include NlTestHelper

  def test_basic_parse
    result = parse_nl_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_nl_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_nl_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_body_numbering_flag
    result = parse_nl_argv(["-b", "a"])
    assert_equal "a", result.flags["body_numbering"]
  end

  def test_number_format_flag
    result = parse_nl_argv(["-n", "rz"])
    assert_equal "rz", result.flags["number_format"]
  end

  def test_width_flag
    result = parse_nl_argv(["-w", "3"])
    assert_equal 3, result.flags["number_width"]
  end
end

class TestNlShouldNumber < Minitest::Test
  def test_all_style
    assert nl_should_number("hello", "a")
    assert nl_should_number("", "a")
  end

  def test_non_empty_style
    assert nl_should_number("hello", "t")
    refute nl_should_number("", "t")
    refute nl_should_number("   ", "t")
  end

  def test_none_style
    refute nl_should_number("hello", "n")
  end

  def test_regex_style
    assert nl_should_number("ERROR: something", "pERROR")
    refute nl_should_number("info: ok", "pERROR")
  end
end

class TestNlFormatNumber < Minitest::Test
  def test_right_justified
    assert_equal "     1", nl_format_number(1, "rn", 6)
  end

  def test_left_justified
    assert_equal "1     ", nl_format_number(1, "ln", 6)
  end

  def test_right_zero
    assert_equal "000001", nl_format_number(1, "rz", 6)
  end
end

class TestNlDetectSection < Minitest::Test
  def test_header
    assert_equal "header", nl_detect_section("\\:\\:\\:", "\\:")
  end

  def test_body
    assert_equal "body", nl_detect_section("\\:\\:", "\\:")
  end

  def test_footer
    assert_equal "footer", nl_detect_section("\\:", "\\:")
  end

  def test_not_section
    assert_nil nl_detect_section("hello", "\\:")
  end
end

class TestNlNumberLines < Minitest::Test
  def test_default_numbering
    lines = %w[hello world] + [""]
    result = nl_number_lines(lines, body_style: "t", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "1"
    assert_includes result[1], "2"
  end

  def test_all_lines_numbered
    lines = ["hello", "", "world"]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "1"
    assert_includes result[1], "2"
    assert_includes result[2], "3"
  end

  def test_custom_increment
    lines = %w[a b c]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 5,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "1"
    assert_includes result[1], "6"
    assert_includes result[2], "11"
  end

  def test_empty_input
    result = nl_number_lines([], body_style: "t", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_equal [], result
  end
end
