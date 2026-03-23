# frozen_string_literal: true

# test_unexpand.rb -- Tests for the Ruby unexpand tool
# =====================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../unexpand_tool"

module UnexpandTestHelper
  UNEXPAND_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "unexpand.json")

  def parse_unexpand_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(UNEXPAND_TEST_SPEC, ["unexpand"] + argv).parse
  end
end

class TestUnexpandCliIntegration < Minitest::Test
  include UnexpandTestHelper

  def test_basic_parse
    result = parse_unexpand_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_unexpand_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_unexpand_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_all_flag
    result = parse_unexpand_argv(["-a"])
    assert result.flags["all"]
  end

  def test_tabs_flag
    result = parse_unexpand_argv(["-t", "4"])
    assert_equal "4", result.flags["tabs"]
  end
end

class TestUnexpandIsTabStop < Minitest::Test
  def test_uniform_at_zero
    assert unexpand_is_tab_stop(0, 8)
  end

  def test_uniform_at_eight
    assert unexpand_is_tab_stop(8, 8)
  end

  def test_uniform_at_three
    refute unexpand_is_tab_stop(3, 8)
  end

  def test_explicit_stop
    assert unexpand_is_tab_stop(4, [4, 8, 12])
  end

  def test_explicit_non_stop
    refute unexpand_is_tab_stop(5, [4, 8, 12])
  end
end

class TestUnexpandUnexpandLine < Minitest::Test
  def test_no_spaces
    assert_equal "hello\n", unexpand_unexpand_line("hello\n", 8, convert_all: false)
  end

  def test_leading_spaces_converted
    result = unexpand_unexpand_line("        hello\n", 8, convert_all: false)
    assert_equal "\thello\n", result
  end

  def test_fewer_spaces_not_converted
    result = unexpand_unexpand_line("   hello\n", 8, convert_all: false)
    assert_equal "   hello\n", result
  end

  def test_empty_line
    assert_equal "\n", unexpand_unexpand_line("\n", 8, convert_all: false)
  end

  def test_custom_tab_width
    result = unexpand_unexpand_line("    hello\n", 4, convert_all: false)
    assert_equal "\thello\n", result
  end
end
