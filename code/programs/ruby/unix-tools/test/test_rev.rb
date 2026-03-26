# frozen_string_literal: true

# test_rev.rb -- Tests for the Ruby rev tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the rev tool's CLI Builder integration and
# business logic. We test:
# - Simple line reversal
# - Multi-line reversal
# - Empty lines
# - File reading
# - The rev_stream function directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the rev_tool module so we can test the business logic functions.
require_relative "../rev_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for rev tests
# ---------------------------------------------------------------------------

module RevTestHelper
  REV_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "rev.json")

  def parse_rev_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(REV_TEST_SPEC, ["rev"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("rev_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestRevCliIntegration < Minitest::Test
  include RevTestHelper

  def test_no_flags_returns_parse_result
    result = parse_rev_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_rev_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "rev"
  end

  def test_version_returns_version_result
    result = parse_rev_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_file_arguments
    result = parse_rev_argv(["file1.txt", "file2.txt"])
    assert_equal ["file1.txt", "file2.txt"], result.arguments["files"]
  end

  def test_no_flags_defined
    result = parse_rev_argv([])
    # rev has no custom flags, only builtin help/version
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_single_file_argument
    result = parse_rev_argv(["myfile.txt"])
    assert_equal ["myfile.txt"], result.arguments["files"]
  end
end

# ===========================================================================
# Test: rev_stream function
# ===========================================================================

class TestRevStream < Minitest::Test
  def test_simple_word
    io = StringIO.new("hello\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "olleh\n", output
  end

  def test_multiple_lines
    io = StringIO.new("abc\ndef\nghi\n")
    output = capture_io { rev_stream(io) }[0]
    lines = output.split("\n")
    assert_equal "cba", lines[0]
    assert_equal "fed", lines[1]
    assert_equal "ihg", lines[2]
  end

  def test_empty_line
    io = StringIO.new("\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "\n", output
  end

  def test_spaces_preserved
    io = StringIO.new("hello world\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "dlrow olleh\n", output
  end

  def test_palindrome
    io = StringIO.new("racecar\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "racecar\n", output
  end

  def test_numbers
    io = StringIO.new("12345\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "54321\n", output
  end

  def test_single_character
    io = StringIO.new("x\n")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "x\n", output
  end

  def test_empty_input
    io = StringIO.new("")
    output = capture_io { rev_stream(io) }[0]
    assert_equal "", output
  end

  def test_mixed_content
    io = StringIO.new("abc 123\n!@#\n")
    output = capture_io { rev_stream(io) }[0]
    lines = output.split("\n")
    assert_equal "321 cba", lines[0]
    assert_equal "#@!", lines[1]
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestRevMainFunction < Minitest::Test
  include RevTestHelper

  def test_main_reads_file
    with_tempfile("hello\nworld\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { rev_main }[0]
      lines = output.split("\n")
      assert_equal "olleh", lines[0]
      assert_equal "dlrow", lines[1]
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_multiple_files
    with_tempfile("abc\n") do |path1|
      with_tempfile("def\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { rev_main }[0]
        lines = output.split("\n")
        assert_equal "cba", lines[0]
        assert_equal "fed", lines[1]
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file_prints_error
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz"])
    _stdout, stderr = capture_io { rev_main }
    assert_includes stderr, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { rev_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { rev_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_stdin_mode
    old_stdin = $stdin
    $stdin = StringIO.new("test\n")
    old_argv = ARGV.dup
    ARGV.replace(["-"])
    output = capture_io { rev_main }[0]
    assert_equal "tset\n", output
  ensure
    $stdin = old_stdin
    ARGV.replace(old_argv)
  end
end
