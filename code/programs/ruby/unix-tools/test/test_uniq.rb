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

  def test_single_line
    result = uniq_filter_lines(%w[hello], count: false, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[hello], result
  end

  def test_all_same
    result = uniq_filter_lines(%w[a a a a], count: false, repeated: false, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal %w[a], result
  end

  def test_count_and_repeated
    result = uniq_filter_lines(%w[a a b c c], count: true, repeated: true, unique: false,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal 2, result.length
    assert_includes result[0], "2"
    assert_includes result[1], "2"
  end

  def test_count_and_unique
    result = uniq_filter_lines(%w[a a b c c], count: true, repeated: false, unique: true,
                               ignore_case: false, skip_fields: 0, skip_chars: 0,
                               check_chars: nil)
    assert_equal 1, result.length
    assert_includes result[0], "1"
    assert_includes result[0], "b"
  end
end

class TestUniqComparisonKeyEdgeCases < Minitest::Test
  def test_skip_fields
    key = uniq_comparison_key("  field1  field2  field3", skip_fields: 2, skip_chars: 0,
                              check_chars: nil, ignore_case: false)
    assert_includes key, "field3"
  end

  def test_skip_fields_more_than_available
    key = uniq_comparison_key("one two", skip_fields: 5, skip_chars: 0,
                              check_chars: nil, ignore_case: false)
    assert_equal "", key
  end

  def test_skip_chars_more_than_length
    key = uniq_comparison_key("hi", skip_fields: 0, skip_chars: 10,
                              check_chars: nil, ignore_case: false)
    assert_equal "", key
  end

  def test_combined_skip_fields_and_chars
    # After skipping 1 field from "field1 hello", remaining is " hello"
    # Then skip 2 chars from " hello" gives "ello"
    key = uniq_comparison_key("field1 hello", skip_fields: 1, skip_chars: 2,
                              check_chars: nil, ignore_case: false)
    assert_equal "ello", key
  end

  def test_check_chars_with_skip
    key = uniq_comparison_key("abcdefgh", skip_fields: 0, skip_chars: 2,
                              check_chars: 3, ignore_case: false)
    assert_equal "cde", key
  end
end

class TestUniqMainIntegration < Minitest::Test
  def test_main_with_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "hello\nhello\nworld\n")
      old_argv = ARGV.dup
      ARGV.replace([path])
      out, _err = capture_io { uniq_main }
      assert_includes out, "hello"
      assert_includes out, "world"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_nonexistent_file
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file.txt"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { uniq_main }
      assert_equal 1, e.status
    end
    assert_includes err, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_with_output_file
    Dir.mktmpdir do |tmp|
      input = File.join(tmp, "input.txt")
      output = File.join(tmp, "output.txt")
      File.write(input, "a\na\nb\n")
      old_argv = ARGV.dup
      ARGV.replace([input, output])
      capture_io { uniq_main }
      content = File.read(output)
      assert_includes content, "a"
      assert_includes content, "b"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { uniq_main }
      assert_equal 0, e.status
    end
    assert_includes out, "uniq"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { uniq_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_count_flag
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "a\na\nb\n")
      old_argv = ARGV.dup
      ARGV.replace(["-c", path])
      out, _err = capture_io { uniq_main }
      assert_includes out, "2"
    ensure
      ARGV.replace(old_argv)
    end
  end
end
