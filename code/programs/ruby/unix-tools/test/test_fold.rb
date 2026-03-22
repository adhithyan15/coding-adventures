# frozen_string_literal: true

# test_fold.rb -- Tests for the Ruby fold tool
# ==============================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../fold_tool"

module FoldTestHelper
  FOLD_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "fold.json")

  def parse_fold_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(FOLD_TEST_SPEC, ["fold"] + argv).parse
  end
end

class TestFoldCliIntegration < Minitest::Test
  include FoldTestHelper

  def test_basic_parse
    result = parse_fold_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_fold_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_fold_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_width_flag
    result = parse_fold_argv(["-w", "40"])
    assert_equal 40, result.flags["width"]
  end

  def test_spaces_flag
    result = parse_fold_argv(["-s"])
    assert result.flags["spaces"]
  end

  def test_bytes_flag
    result = parse_fold_argv(["-b"])
    assert result.flags["bytes"]
  end
end

class TestFoldFoldLine < Minitest::Test
  def test_short_line_unchanged
    assert_equal "hello", fold_fold_line("hello", 80, break_at_spaces: false, count_bytes: false)
  end

  def test_exact_width
    assert_equal "a" * 10, fold_fold_line("a" * 10, 10, break_at_spaces: false, count_bytes: false)
  end

  def test_long_line_wrapped
    result = fold_fold_line("a" * 20, 10, break_at_spaces: false, count_bytes: false)
    assert_includes result, "\n"
    lines = result.split("\n")
    assert_equal 10, lines[0].length
  end

  def test_empty_string
    assert_equal "", fold_fold_line("", 80, break_at_spaces: false, count_bytes: false)
  end

  def test_width_1
    assert_equal "a\nb\nc", fold_fold_line("abc", 1, break_at_spaces: false, count_bytes: false)
  end

  def test_no_break_needed
    assert_equal "short", fold_fold_line("short", 100, break_at_spaces: false, count_bytes: false)
  end
end
