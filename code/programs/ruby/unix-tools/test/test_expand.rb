# frozen_string_literal: true

# test_expand.rb -- Tests for the Ruby expand tool
# ==================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../expand_tool"

module ExpandTestHelper
  EXPAND_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "expand.json")

  def parse_expand_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(EXPAND_TEST_SPEC, ["expand"] + argv).parse
  end
end

class TestExpandCliIntegration < Minitest::Test
  include ExpandTestHelper

  def test_basic_parse
    result = parse_expand_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_expand_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_expand_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_tabs_flag
    result = parse_expand_argv(["-t", "4"])
    assert_equal "4", result.flags["tabs"]
  end

  def test_initial_flag
    result = parse_expand_argv(["-i"])
    assert result.flags["initial"]
  end
end

class TestExpandParseTabStops < Minitest::Test
  def test_default
    assert_equal 8, expand_parse_tab_stops(nil)
  end

  def test_single_number
    assert_equal 4, expand_parse_tab_stops("4")
  end

  def test_comma_list
    assert_equal [4, 8, 12], expand_parse_tab_stops("4,8,12")
  end
end

class TestExpandSpacesToNextStop < Minitest::Test
  def test_uniform_at_zero
    assert_equal 8, expand_spaces_to_next_stop(0, 8)
  end

  def test_uniform_at_three
    assert_equal 5, expand_spaces_to_next_stop(3, 8)
  end

  def test_explicit_stops
    assert_equal 4, expand_spaces_to_next_stop(0, [4, 8, 12])
    assert_equal 4, expand_spaces_to_next_stop(4, [4, 8, 12])
  end

  def test_past_explicit_stops
    assert_equal 1, expand_spaces_to_next_stop(15, [4, 8])
  end
end

class TestExpandExpandLine < Minitest::Test
  def test_no_tabs
    assert_equal "hello\n", expand_expand_line("hello\n", 8, initial_only: false)
  end

  def test_single_tab_at_start
    assert_equal "        hello\n", expand_expand_line("\thello\n", 8, initial_only: false)
  end

  def test_tab_at_column_3
    assert_equal "abc     def\n", expand_expand_line("abc\tdef\n", 8, initial_only: false)
  end

  def test_custom_tab_width
    assert_equal "    hello\n", expand_expand_line("\thello\n", 4, initial_only: false)
  end

  def test_initial_only_preserves_inner_tabs
    result = expand_expand_line("\thello\tworld\n", 8, initial_only: true)
    assert result.start_with?("        hello")
    assert_includes result, "\t"
  end

  def test_empty_line
    assert_equal "\n", expand_expand_line("\n", 8, initial_only: false)
  end
end
