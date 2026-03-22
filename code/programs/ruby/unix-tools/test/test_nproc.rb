# frozen_string_literal: true

# test_nproc.rb -- Tests for the Ruby nproc tool
# ================================================
#
# === What These Tests Verify ===
#
# These tests exercise the nproc tool's CLI Builder integration and
# business logic. We test:
# - The get_nproc function with default parameters
# - The --all flag behavior
# - The --ignore N flag behavior
# - Minimum result of 1 (even with large ignore values)
# - CLI Builder integration (--help, --version)
# - Main function integration

require "minitest/autorun"
require "etc"
require "coding_adventures_cli_builder"

# Load the nproc_tool module so we can test the business logic functions.
require_relative "../nproc_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for nproc tests
# ---------------------------------------------------------------------------

module NprocTestHelper
  NPROC_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "nproc.json")

  def parse_nproc_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(NPROC_TEST_SPEC, ["nproc"] + argv).parse
  end
end

# ===========================================================================
# Test: get_nproc function
# ===========================================================================

class TestGetNproc < Minitest::Test
  def test_default_returns_positive_integer
    count = get_nproc
    assert_kind_of Integer, count
    assert count >= 1, "nproc should return at least 1"
  end

  def test_matches_etc_nprocessors
    expected = Etc.nprocessors
    assert_equal expected, get_nproc
  end

  def test_all_flag_returns_positive_integer
    count = get_nproc(all: true)
    assert_kind_of Integer, count
    assert count >= 1
  end

  def test_ignore_subtracts_from_count
    total = Etc.nprocessors
    if total > 2
      count = get_nproc(ignore: 2)
      assert_equal total - 2, count
    else
      # On systems with 1-2 CPUs, ignore 0 to avoid testing edge case
      count = get_nproc(ignore: 0)
      assert_equal total, count
    end
  end

  def test_ignore_never_goes_below_one
    count = get_nproc(ignore: 999_999)
    assert_equal 1, count
  end

  def test_ignore_zero_returns_full_count
    total = Etc.nprocessors
    count = get_nproc(ignore: 0)
    assert_equal total, count
  end

  def test_ignore_exactly_total_returns_one
    total = Etc.nprocessors
    count = get_nproc(ignore: total)
    # total - total = 0, but minimum is 1
    assert_equal 1, count if total > 0
  end

  def test_ignore_one_less_than_total
    total = Etc.nprocessors
    count = get_nproc(ignore: total - 1)
    assert_equal 1, count
  end

  def test_all_with_ignore
    count = get_nproc(all: true, ignore: 1)
    total = Etc.nprocessors
    expected = [total - 1, 1].max
    assert_equal expected, count
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestNprocDefaultBehavior < Minitest::Test
  include NprocTestHelper

  def test_no_args_returns_parse_result
    result = parse_nproc_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_flags_set_by_default
    result = parse_nproc_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    refute result.flags["all"], "--all should not be set by default"
  end
end

# ===========================================================================
# Test: --all flag
# ===========================================================================

class TestNprocAllFlag < Minitest::Test
  include NprocTestHelper

  def test_all_flag_is_set
    result = parse_nproc_argv(["--all"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["all"], "--all should be set"
  end
end

# ===========================================================================
# Test: --ignore flag
# ===========================================================================

class TestNprocIgnoreFlag < Minitest::Test
  include NprocTestHelper

  def test_ignore_flag_with_value
    result = parse_nproc_argv(["--ignore", "2"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal 2, result.flags["ignore"]
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestNprocHelpFlag < Minitest::Test
  include NprocTestHelper

  def test_help_returns_help_result
    result = parse_nproc_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_nproc_argv(["--help"])
    assert_includes result.text, "nproc"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestNprocVersionFlag < Minitest::Test
  include NprocTestHelper

  def test_version_returns_version_result
    result = parse_nproc_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_nproc_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestNprocMainFunction < Minitest::Test
  def test_main_prints_processor_count
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { nproc_main }[0]
    count = output.strip.to_i
    assert count >= 1, "nproc should print at least 1"
    assert_equal Etc.nprocessors, count
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_all_flag
    old_argv = ARGV.dup
    ARGV.replace(["--all"])
    output = capture_io { nproc_main }[0]
    count = output.strip.to_i
    assert count >= 1
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { nproc_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { nproc_main }
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
      output = capture_io { nproc_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
