# frozen_string_literal: true

# test_cmp.rb -- Tests for the Ruby cmp tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the cmp tool's byte-by-byte comparison engine.
# We test:
# - Identical files (exit code 0)
# - Different files (exit code 1, first difference reported)
# - Silent mode (-s, no output)
# - List mode (-l, all differences listed)
# - Print bytes mode (-b, show byte values)
# - Skip bytes (-i)
# - Max bytes (-n)
# - EOF handling (one file shorter)
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "stringio"
require "coding_adventures_cli_builder"

require_relative "../cmp_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module CmpTestHelper
  CMP_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "cmp.json")

  def parse_cmp_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CMP_TEST_SPEC, ["cmp"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestCmpCliIntegration < Minitest::Test
  include CmpTestHelper

  def test_help_returns_help_result
    result = parse_cmp_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_cmp_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse
    result = parse_cmp_argv(["file1", "file2"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_silent_flag
    result = parse_cmp_argv(["-s", "file1", "file2"])
    assert result.flags["silent"]
  end

  def test_list_flag
    result = parse_cmp_argv(["-l", "file1", "file2"])
    assert result.flags["list"]
  end
end

# ===========================================================================
# Test: parse_skip
# ===========================================================================

class TestCmpParseSkip < Minitest::Test
  def test_nil_skip
    assert_equal [0, 0], cmp_parse_skip(nil)
  end

  def test_single_number
    assert_equal [10, 10], cmp_parse_skip("10")
  end

  def test_two_numbers
    assert_equal [5, 15], cmp_parse_skip("5:15")
  end

  def test_zero
    assert_equal [0, 0], cmp_parse_skip("0")
  end
end

# ===========================================================================
# Test: cmp_compare
# ===========================================================================

class TestCmpCompare < Minitest::Test
  def test_identical_files
    io_a = StringIO.new("hello world")
    io_b = StringIO.new("hello world")

    output, code = cmp_compare(io_a, io_b, "a", "b")
    assert_equal 0, code
    assert_empty output
  end

  def test_different_files_first_byte
    io_a = StringIO.new("abc")
    io_b = StringIO.new("xbc")

    output, code = cmp_compare(io_a, io_b, "a", "b")
    assert_equal 1, code
    assert_equal 1, output.length
    assert_includes output[0], "differ: byte 1"
  end

  def test_different_files_later_byte
    io_a = StringIO.new("abc")
    io_b = StringIO.new("axc")

    output, code = cmp_compare(io_a, io_b, "a", "b")
    assert_equal 1, code
    assert_includes output[0], "byte 2"
  end

  def test_line_tracking
    # Newline at position 1, difference at position 3 (line 2)
    io_a = StringIO.new("a\nbc")
    io_b = StringIO.new("a\nxc")

    output, code = cmp_compare(io_a, io_b, "a", "b")
    assert_equal 1, code
    assert_includes output[0], "line 2"
  end

  def test_silent_mode
    io_a = StringIO.new("abc")
    io_b = StringIO.new("xyz")

    output, code = cmp_compare(io_a, io_b, "a", "b", silent: true)
    assert_equal 1, code
    assert_empty output
  end

  def test_list_mode
    io_a = StringIO.new("abc")
    io_b = StringIO.new("axc")

    output, code = cmp_compare(io_a, io_b, "a", "b", list: true)
    assert_equal 1, code
    # Should show byte number and octal values
    assert_match(/2\s+/, output[0])
  end

  def test_list_mode_multiple_differences
    io_a = StringIO.new("abc")
    io_b = StringIO.new("xyz")

    output, code = cmp_compare(io_a, io_b, "a", "b", list: true)
    assert_equal 1, code
    assert_equal 3, output.length
  end

  def test_print_bytes_mode
    io_a = StringIO.new("a")
    io_b = StringIO.new("b")

    output, code = cmp_compare(io_a, io_b, "a", "b", list: true, print_bytes: true)
    assert_equal 1, code
    assert_includes output[0], "a"
    assert_includes output[0], "b"
  end

  def test_eof_first_file_shorter
    io_a = StringIO.new("ab")
    io_b = StringIO.new("abc")

    output, code = cmp_compare(io_a, io_b, "fileA", "fileB")
    assert_equal 1, code
    assert_includes output[0], "EOF on fileA"
  end

  def test_eof_second_file_shorter
    io_a = StringIO.new("abc")
    io_b = StringIO.new("ab")

    output, code = cmp_compare(io_a, io_b, "fileA", "fileB")
    assert_equal 1, code
    assert_includes output[0], "EOF on fileB"
  end

  def test_eof_silent_mode
    io_a = StringIO.new("ab")
    io_b = StringIO.new("abc")

    output, code = cmp_compare(io_a, io_b, "a", "b", silent: true)
    assert_equal 1, code
    assert_empty output
  end

  def test_skip_bytes
    io_a = StringIO.new("XXXabc")
    io_b = StringIO.new("YYYabc")

    output, code = cmp_compare(io_a, io_b, "a", "b", skip: "3")
    assert_equal 0, code
  end

  def test_skip_different_amounts
    io_a = StringIO.new("XXabc")
    io_b = StringIO.new("YYYabc")

    output, code = cmp_compare(io_a, io_b, "a", "b", skip: "2:3")
    assert_equal 0, code
  end

  def test_max_bytes
    io_a = StringIO.new("abcXXX")
    io_b = StringIO.new("abcYYY")

    output, code = cmp_compare(io_a, io_b, "a", "b", max_bytes: 3)
    assert_equal 0, code
  end

  def test_max_bytes_with_difference
    io_a = StringIO.new("aXcdef")
    io_b = StringIO.new("aYcdef")

    output, code = cmp_compare(io_a, io_b, "a", "b", max_bytes: 3)
    assert_equal 1, code
  end

  def test_empty_files
    io_a = StringIO.new("")
    io_b = StringIO.new("")

    output, code = cmp_compare(io_a, io_b, "a", "b")
    assert_equal 0, code
    assert_empty output
  end
end

# ===========================================================================
# Test: printable_char helper
# ===========================================================================

class TestCmpPrintableChar < Minitest::Test
  def test_printable_ascii
    assert_equal "a", printable_char("a")
  end

  def test_space
    assert_equal " ", printable_char(" ")
  end

  def test_null_byte
    result = printable_char("\x00")
    assert_match(/\\000/, result)
  end

  def test_newline
    result = printable_char("\n")
    assert_match(/\\012/, result)
  end
end

# ===========================================================================
# Test: File-based integration
# ===========================================================================

class TestCmpFileIntegration < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("cmp_test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_compare_identical_files
    f1 = File.join(@tmpdir, "a.bin")
    f2 = File.join(@tmpdir, "b.bin")
    File.binwrite(f1, "hello world")
    File.binwrite(f2, "hello world")

    io_a = File.open(f1, "rb")
    io_b = File.open(f2, "rb")
    output, code = cmp_compare(io_a, io_b, f1, f2)
    io_a.close
    io_b.close

    assert_equal 0, code
  end

  def test_compare_different_binary_files
    f1 = File.join(@tmpdir, "a.bin")
    f2 = File.join(@tmpdir, "b.bin")
    File.binwrite(f1, "\x00\x01\x02\x03")
    File.binwrite(f2, "\x00\x01\xFF\x03")

    io_a = File.open(f1, "rb")
    io_b = File.open(f2, "rb")
    output, code = cmp_compare(io_a, io_b, f1, f2)
    io_a.close
    io_b.close

    assert_equal 1, code
    assert_includes output[0], "byte 3"
  end

  def test_list_with_print_bytes_for_nonprintable
    f1 = File.join(@tmpdir, "a.bin")
    f2 = File.join(@tmpdir, "b.bin")
    File.binwrite(f1, "\x00")
    File.binwrite(f2, "\x01")

    io_a = File.open(f1, "rb")
    io_b = File.open(f2, "rb")
    output, code = cmp_compare(io_a, io_b, "a", "b", list: true, print_bytes: true)
    io_a.close
    io_b.close

    assert_equal 1, code
    # Should contain octal representations
    assert_match(/\\/, output[0])
  end

  def test_identical_files_with_max_bytes
    f1 = File.join(@tmpdir, "a.bin")
    f2 = File.join(@tmpdir, "b.bin")
    File.binwrite(f1, "hello")
    File.binwrite(f2, "hello")

    io_a = File.open(f1, "rb")
    io_b = File.open(f2, "rb")
    output, code = cmp_compare(io_a, io_b, "a", "b", max_bytes: 5)
    io_a.close
    io_b.close

    assert_equal 0, code
  end

  def test_skip_and_max_bytes_combined
    f1 = File.join(@tmpdir, "a.bin")
    f2 = File.join(@tmpdir, "b.bin")
    File.binwrite(f1, "XXXhello")
    File.binwrite(f2, "YYYhello")

    io_a = File.open(f1, "rb")
    io_b = File.open(f2, "rb")
    output, code = cmp_compare(io_a, io_b, "a", "b", skip: "3", max_bytes: 5)
    io_a.close
    io_b.close

    assert_equal 0, code
  end
end
