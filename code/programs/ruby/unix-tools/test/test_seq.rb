# frozen_string_literal: true

# test_seq.rb -- Tests for the Ruby seq tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the seq tool's CLI Builder integration and
# business logic. We test:
# - Single argument (seq LAST)
# - Two arguments (seq FIRST LAST)
# - Three arguments (seq FIRST INCR LAST)
# - Equal width mode (-w)
# - Custom separator (-s)
# - Custom format (-f)
# - The parse_seq_args and format_seq_number functions directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the seq_tool module so we can test the business logic functions.
require_relative "../seq_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for seq tests
# ---------------------------------------------------------------------------

module SeqTestHelper
  SEQ_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "seq.json")

  def parse_seq_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(SEQ_TEST_SPEC, ["seq"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestSeqCliIntegration < Minitest::Test
  include SeqTestHelper

  def test_help_returns_help_result
    result = parse_seq_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "seq"
  end

  def test_version_returns_version_result
    result = parse_seq_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_single_arg
    result = parse_seq_argv(["5"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["5"], result.arguments["numbers"]
  end

  def test_two_args
    result = parse_seq_argv(["3", "7"])
    assert_equal ["3", "7"], result.arguments["numbers"]
  end

  def test_equal_width_flag
    result = parse_seq_argv(["-w", "10"])
    assert result.flags["equal_width"]
  end

  def test_separator_flag
    result = parse_seq_argv(["-s", ", ", "3"])
    assert_equal ", ", result.flags["separator"]
  end
end

# ===========================================================================
# Test: parse_seq_args function
# ===========================================================================

class TestParseSeqArgs < Minitest::Test
  def test_single_arg
    first, incr, last = parse_seq_args(["5"])
    assert_equal 1.0, first
    assert_equal 1.0, incr
    assert_equal 5.0, last
  end

  def test_two_args_ascending
    first, incr, last = parse_seq_args(["3", "7"])
    assert_equal 3.0, first
    assert_equal 1.0, incr
    assert_equal 7.0, last
  end

  def test_two_args_descending
    first, incr, last = parse_seq_args(["7", "3"])
    assert_equal 7.0, first
    assert_equal(-1.0, incr)
    assert_equal 3.0, last
  end

  def test_three_args
    first, incr, last = parse_seq_args(["1", "2", "10"])
    assert_equal 1.0, first
    assert_equal 2.0, incr
    assert_equal 10.0, last
  end

  def test_float_args
    first, incr, last = parse_seq_args(["0.5", "0.5", "2.5"])
    assert_in_delta 0.5, first
    assert_in_delta 0.5, incr
    assert_in_delta 2.5, last
  end

  def test_invalid_count_raises
    assert_raises(ArgumentError) { parse_seq_args([]) }
  end
end

# ===========================================================================
# Test: format_seq_number function
# ===========================================================================

class TestFormatSeqNumber < Minitest::Test
  def test_integer_value
    assert_equal "5", format_seq_number(5.0, nil, 0)
  end

  def test_float_value
    assert_equal "2.5", format_seq_number(2.5, nil, 0)
  end

  def test_zero_padded_integer
    assert_equal "05", format_seq_number(5.0, nil, 2)
  end

  def test_zero_padded_wider
    assert_equal "005", format_seq_number(5.0, nil, 3)
  end

  def test_custom_format
    assert_equal "005", format_seq_number(5.0, "%03g", 0)
  end

  def test_negative_integer
    assert_equal "-3", format_seq_number(-3.0, nil, 0)
  end
end

# ===========================================================================
# Test: compute_pad_width function
# ===========================================================================

class TestComputePadWidth < Minitest::Test
  def test_single_digit_range
    assert_equal 1, compute_pad_width(1.0, 9.0)
  end

  def test_double_digit_range
    assert_equal 2, compute_pad_width(1.0, 10.0)
  end

  def test_triple_digit
    assert_equal 3, compute_pad_width(1.0, 100.0)
  end

  def test_negative_range
    assert_equal 2, compute_pad_width(-5.0, 5.0)
  end

  def test_same_width
    assert_equal 1, compute_pad_width(1.0, 5.0)
  end

  def test_first_wider_than_last
    assert_equal 3, compute_pad_width(100.0, 1.0)
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestSeqMainFunction < Minitest::Test
  include SeqTestHelper

  def test_main_seq_5
    old_argv = ARGV.dup
    ARGV.replace(["5"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal 5, lines.length
    assert_equal "1", lines[0]
    assert_equal "5", lines[4]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_seq_2_to_4
    old_argv = ARGV.dup
    ARGV.replace(["2", "4"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal 3, lines.length
    assert_equal "2", lines[0]
    assert_equal "4", lines[2]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_seq_with_increment
    old_argv = ARGV.dup
    ARGV.replace(["1", "2", "7"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal 4, lines.length
    assert_equal "1", lines[0]
    assert_equal "7", lines[3]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { seq_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { seq_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_custom_separator
    old_argv = ARGV.dup
    ARGV.replace(["-s", ", ", "3"])
    output = capture_io { seq_main }[0]
    assert_equal "1, 2, 3\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_descending_two_args
    # Two-arg form: seq FIRST LAST where FIRST > LAST triggers auto-decrement.
    old_argv = ARGV.dup
    ARGV.replace(["5", "1"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal 5, lines.length
    assert_equal "5", lines[0]
    assert_equal "1", lines[4]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_equal_width
    old_argv = ARGV.dup
    ARGV.replace(["-w", "8", "10"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal "08", lines[0]
    assert_equal "09", lines[1]
    assert_equal "10", lines[2]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_zero_increment_exits
    old_argv = ARGV.dup
    ARGV.replace(["1", "0", "5"])
    err = assert_raises(SystemExit) do
      capture_io { seq_main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_format_flag
    old_argv = ARGV.dup
    ARGV.replace(["-f", "%05g", "3"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal "00001", lines[0]
    assert_equal "00003", lines[2]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_float_sequence
    old_argv = ARGV.dup
    ARGV.replace(["1", "0.5", "3"])
    output = capture_io { seq_main }[0]
    lines = output.strip.split("\n")
    assert_equal 5, lines.length
    assert_equal "1", lines[0]
    assert_equal "1.5", lines[1]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_single_number
    old_argv = ARGV.dup
    ARGV.replace(["1"])
    output = capture_io { seq_main }[0]
    assert_equal "1\n", output
  ensure
    ARGV.replace(old_argv)
  end
end
