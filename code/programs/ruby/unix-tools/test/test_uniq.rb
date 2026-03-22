# frozen_string_literal: true

# test_uniq.rb -- Tests for the Ruby uniq tool
# ==============================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../uniq_tool"

module UniqTestHelper
  UNIQ_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "uniq.json")

  def parse_uniq_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(UNIQ_TEST_SPEC, ["uniq"] + argv).parse
  end
end

class TestUniqCliIntegration < Minitest::Test
  include UniqTestHelper

  def test_basic_parse
    result = parse_uniq_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_uniq_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_uniq_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_count_flag
    result = parse_uniq_argv(["-c"])
    assert result.flags["count"]
  end

  def test_repeated_flag
    result = parse_uniq_argv(["-d"])
    assert result.flags["repeated"]
  end

  def test_ignore_case_flag
    result = parse_uniq_argv(["-i"])
    assert result.flags["ignore_case"]
  end
end

class TestUniqComparisonKey < Minitest::Test
  def test_basic
    key = uniq_comparison_key("hello", skip_fields: 0, skip_chars: 0,
                              check_chars: nil, ignore_case: false)
    assert_equal "hello", key
  end

  def test_ignore_case
    key = uniq_comparison_key("Hello", skip_fields: 0, skip_chars: 0,
                              check_chars: nil, ignore_case: true)
    assert_equal "hello", key
  end

  def test_skip_chars
    key = uniq_comparison_key("hello", skip_fields: 0, skip_chars: 2,
                              check_chars: nil, ignore_case: false)
    assert_equal "llo", key
  end

  def test_check_chars
    key = uniq_comparison_key("hello", skip_fields: 0, skip_chars: 0,
                              check_chars: 3, ignore_case: false)
    assert_equal "hel", key
  end
end

class TestUniqFilterLines < Minitest::Test
  def test_no_duplicates
    result = uniq_filter_lines(%w[a b c], count: false, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[a b c], result
  end

  def test_adjacent_duplicates
    result = uniq_filter_lines(%w[a a b b c], count: false, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[a b c], result
  end

  def test_count_flag
    result = uniq_filter_lines(%w[a a b], count: true, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_includes result[0], "2"
    assert_includes result[0], "a"
  end

  def test_repeated_flag
    result = uniq_filter_lines(%w[a a b c c], count: false, repeated: true, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[a c], result
  end

  def test_unique_flag
    result = uniq_filter_lines(%w[a a b c c], count: false, repeated: false, unique: true,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[b], result
  end

  def test_ignore_case
    result = uniq_filter_lines(%w[Hello hello HELLO], count: false, repeated: false,
                               unique: false, ignore_case: true, skip_fields: 0,
                               skip_chars: 0, check_chars: nil)
    assert_equal 1, result.length
  end

  def test_empty_input
    result = uniq_filter_lines([], count: false, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal [], result
  end
end
