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

  def test_section_delimiters
    lines = ["\\:\\:\\:", "header line", "\\:\\:", "body line", "\\:", "footer line"]
    result = nl_number_lines(lines, body_style: "a", header_style: "a",
                             footer_style: "a", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    # Section delimiter lines produce empty strings in the output
    assert_equal "", result[0]
    assert_includes result[1], "1" # header line numbered starting from 1
    assert_equal "", result[2]
    assert_includes result[3], "body line"
    assert_equal "", result[4]
    assert_includes result[5], "footer line"
  end

  def test_header_resets_number
    lines = ["\\:\\:\\:", "h1", "\\:\\:\\:", "h2"]
    result = nl_number_lines(lines, body_style: "a", header_style: "a",
                             footer_style: "a", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    # Each header section resets numbering
    assert_includes result[1], "1"
    assert_includes result[3], "1"
  end

  def test_non_numbered_body_line
    lines = ["hello", ""]
    result = nl_number_lines(lines, body_style: "t", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    # Empty line should not be numbered (style "t")
    assert_includes result[0], "1"
    refute_includes result[1], "2"
  end

  def test_left_justified_format
    lines = %w[hello world]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "ln", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert result[0].start_with?("1")
  end

  def test_right_zero_format
    lines = %w[hello]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rz", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "000001"
  end

  def test_custom_start_number
    lines = %w[hello]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 10, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "10"
  end

  def test_custom_separator
    lines = %w[hello]
    result = nl_number_lines(lines, body_style: "a", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: ": ", section_delimiter: "\\:")
    assert_includes result[0], ": "
  end

  def test_regex_numbering_style
    lines = ["ERROR: fail", "info: ok", "ERROR: another"]
    result = nl_number_lines(lines, body_style: "pERROR", header_style: "n",
                             footer_style: "n", start_number: 1, increment: 1,
                             number_format: "rn", number_width: 6,
                             separator: "\t", section_delimiter: "\\:")
    assert_includes result[0], "1"
    refute_includes result[1], "2" # info line should not be numbered
    assert_includes result[2], "2"
  end
end

class TestNlShouldNumberEdgeCases < Minitest::Test
  def test_unknown_style
    refute nl_should_number("hello", "x")
  end

  def test_regex_no_match
    refute nl_should_number("hello world", "p^ERROR")
  end
end

class TestNlMainIntegration < Minitest::Test
  def test_main_with_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "hello\nworld\n")
      old_argv = ARGV.dup
      ARGV.replace([path])
      out, _err = capture_io { nl_main }
      assert_includes out, "hello"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_nonexistent_file
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file.txt"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { nl_main }
      assert_equal 1, e.status
    end
    assert_includes err, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { nl_main }
      assert_equal 0, e.status
    end
    assert_includes out, "nl"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { nl_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_with_body_numbering
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "hello\n\nworld\n")
      old_argv = ARGV.dup
      ARGV.replace(["-b", "a", path])
      out, _err = capture_io { nl_main }
      assert_includes out, "1"
    ensure
      ARGV.replace(old_argv)
    end
  end
end
