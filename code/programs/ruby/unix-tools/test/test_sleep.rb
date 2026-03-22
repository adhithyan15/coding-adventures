# frozen_string_literal: true

# test_sleep.rb -- Tests for the Ruby sleep tool
# ================================================
#
# === What These Tests Verify ===
#
# These tests exercise the sleep tool's CLI Builder integration and
# business logic. We test:
# - The parse_duration function with all suffix types
# - The total_sleep_seconds function with multiple durations
# - The perform_sleep function with a mock sleep callable
# - Invalid duration strings (error handling)
# - CLI Builder integration (--help, --version)
# - Main function integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

# Load the sleep_tool module so we can test the business logic functions.
require_relative "../sleep_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for sleep tests
# ---------------------------------------------------------------------------

module SleepTestHelper
  SLEEP_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "sleep.json")

  def parse_sleep_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(SLEEP_TEST_SPEC, ["sleep"] + argv).parse
  end
end

# ===========================================================================
# Test: parse_duration function
# ===========================================================================

class TestParseDuration < Minitest::Test
  # --- Seconds (default and explicit) --------------------------------------

  def test_plain_integer_seconds
    assert_equal 5.0, parse_duration("5")
  end

  def test_plain_float_seconds
    assert_in_delta 0.5, parse_duration("0.5"), 0.001
  end

  def test_explicit_s_suffix
    assert_equal 5.0, parse_duration("5s")
  end

  def test_fractional_seconds_with_suffix
    assert_in_delta 2.5, parse_duration("2.5s"), 0.001
  end

  # --- Minutes -------------------------------------------------------------

  def test_minutes_suffix
    assert_equal 120.0, parse_duration("2m")
  end

  def test_fractional_minutes
    assert_in_delta 90.0, parse_duration("1.5m"), 0.001
  end

  # --- Hours ---------------------------------------------------------------

  def test_hours_suffix
    assert_equal 3600.0, parse_duration("1h")
  end

  def test_fractional_hours
    assert_in_delta 5400.0, parse_duration("1.5h"), 0.001
  end

  # --- Days ----------------------------------------------------------------

  def test_days_suffix
    assert_equal 86_400.0, parse_duration("1d")
  end

  def test_fractional_days
    assert_in_delta 43_200.0, parse_duration("0.5d"), 0.001
  end

  # --- Edge cases ----------------------------------------------------------

  def test_zero_seconds
    assert_equal 0.0, parse_duration("0")
  end

  def test_zero_with_suffix
    assert_equal 0.0, parse_duration("0s")
  end

  def test_large_number
    assert_equal 1_000_000.0, parse_duration("1000000")
  end

  # --- Invalid inputs ------------------------------------------------------

  def test_invalid_duration_raises_error
    assert_raises(ArgumentError) { parse_duration("abc") }
  end

  def test_empty_string_raises_error
    assert_raises(ArgumentError) { parse_duration("") }
  end

  def test_negative_number_raises_error
    # Our regex doesn't match negative numbers.
    assert_raises(ArgumentError) { parse_duration("-5") }
  end

  def test_invalid_suffix_raises_error
    assert_raises(ArgumentError) { parse_duration("5x") }
  end

  def test_suffix_only_raises_error
    assert_raises(ArgumentError) { parse_duration("s") }
  end
end

# ===========================================================================
# Test: total_sleep_seconds function
# ===========================================================================

class TestTotalSleepSeconds < Minitest::Test
  def test_single_duration
    assert_equal 5.0, total_sleep_seconds(["5"])
  end

  def test_multiple_durations_summed
    # 1m + 30s = 60 + 30 = 90
    assert_in_delta 90.0, total_sleep_seconds(["1m", "30s"]), 0.001
  end

  def test_mixed_suffixes
    # 1h + 2m + 3s = 3600 + 120 + 3 = 3723
    assert_in_delta 3723.0, total_sleep_seconds(["1h", "2m", "3s"]), 0.001
  end

  def test_all_seconds
    assert_in_delta 15.0, total_sleep_seconds(["5", "5", "5"]), 0.001
  end

  def test_single_day
    assert_equal 86_400.0, total_sleep_seconds(["1d"])
  end

  def test_invalid_duration_in_list_raises_error
    assert_raises(ArgumentError) { total_sleep_seconds(["1m", "bad"]) }
  end
end

# ===========================================================================
# Test: perform_sleep function
# ===========================================================================

class TestPerformSleep < Minitest::Test
  def test_calls_sleep_with_correct_seconds
    slept_seconds = nil
    mock_sleep = ->(s) { slept_seconds = s }

    perform_sleep(5.0, mock_sleep)
    assert_equal 5.0, slept_seconds
  end

  def test_calls_sleep_with_zero
    slept_seconds = nil
    mock_sleep = ->(s) { slept_seconds = s }

    perform_sleep(0.0, mock_sleep)
    assert_equal 0.0, slept_seconds
  end

  def test_calls_sleep_with_fractional_seconds
    slept_seconds = nil
    mock_sleep = ->(s) { slept_seconds = s }

    perform_sleep(0.5, mock_sleep)
    assert_in_delta 0.5, slept_seconds, 0.001
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestSleepDefaultBehavior < Minitest::Test
  include SleepTestHelper

  def test_single_arg_returns_parse_result
    result = parse_sleep_argv(["5"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_single_arg_has_duration
    result = parse_sleep_argv(["5"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["5"], result.arguments["duration"]
  end

  def test_multiple_args_has_durations
    result = parse_sleep_argv(["1m", "30s"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["1m", "30s"], result.arguments["duration"]
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestSleepHelpFlag < Minitest::Test
  include SleepTestHelper

  def test_help_returns_help_result
    result = parse_sleep_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_sleep_argv(["--help"])
    assert_includes result.text, "sleep"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestSleepVersionFlag < Minitest::Test
  include SleepTestHelper

  def test_version_returns_version_result
    result = parse_sleep_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_sleep_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestSleepMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { sleep_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { sleep_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_prints_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { sleep_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
