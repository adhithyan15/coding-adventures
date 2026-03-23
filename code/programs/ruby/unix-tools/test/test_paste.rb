# frozen_string_literal: true

# test_paste.rb -- Tests for the Ruby paste tool
# =================================================
#
# === What These Tests Verify ===
#
# These tests exercise the paste tool's parallel and serial merging,
# delimiter parsing, and CLI Builder integration. We test:
# - Parallel mode (default): zip lines from multiple files
# - Serial mode (-s): join lines within each file
# - Custom delimiters (-d): cyclic delimiter list
# - Escape sequences in delimiter list

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tempfile"
require "stringio"
require "coding_adventures_cli_builder"

require_relative "../paste_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module PasteTestHelper
  PASTE_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "paste.json")

  def parse_paste_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(PASTE_TEST_SPEC, ["paste"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("paste_test")
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

class TestPasteCliIntegration < Minitest::Test
  include PasteTestHelper

  def test_help_returns_help_result
    result = parse_paste_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "paste"
  end

  def test_version_returns_version_result
    result = parse_paste_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_serial_flag
    result = parse_paste_argv(["-s", "/dev/null"])
    assert result.flags["serial"]
  end

  def test_delimiters_flag
    result = parse_paste_argv(["-d", ",", "/dev/null"])
    assert_equal ",", result.flags["delimiters"]
  end
end

# ===========================================================================
# Test: parse_delimiters
# ===========================================================================

class TestParseDelimiters < Minitest::Test
  def test_default_tab
    assert_equal ["\t"], parse_delimiters(nil)
  end

  def test_empty_string
    assert_equal ["\t"], parse_delimiters("")
  end

  def test_single_char
    assert_equal [","], parse_delimiters(",")
  end

  def test_multiple_chars
    assert_equal [",", ":"], parse_delimiters(",:")
  end

  def test_escape_tab
    assert_equal ["\t"], parse_delimiters("\\t")
  end

  def test_escape_newline
    assert_equal ["\n"], parse_delimiters("\\n")
  end

  def test_escape_backslash
    assert_equal ["\\"], parse_delimiters("\\\\")
  end

  def test_escape_zero
    assert_equal [""], parse_delimiters("\\0")
  end

  def test_mixed_escapes
    assert_equal [",", "\t", "\n"], parse_delimiters(",\\t\\n")
  end
end

# ===========================================================================
# Test: paste_parallel
# ===========================================================================

class TestPasteParallel < Minitest::Test
  def test_two_files_equal_length
    io1 = StringIO.new("a\nb\nc\n")
    io2 = StringIO.new("1\n2\n3\n")
    result = paste_parallel([io1, io2], ["\t"], "\n")
    assert_equal ["a\t1", "b\t2", "c\t3"], result
  end

  def test_unequal_length_files
    io1 = StringIO.new("a\nb\n")
    io2 = StringIO.new("1\n2\n3\n")
    result = paste_parallel([io1, io2], ["\t"], "\n")
    assert_equal ["a\t1", "b\t2", "\t3"], result
  end

  def test_single_file
    io1 = StringIO.new("a\nb\n")
    result = paste_parallel([io1], ["\t"], "\n")
    assert_equal ["a", "b"], result
  end

  def test_custom_delimiter
    io1 = StringIO.new("a\nb\n")
    io2 = StringIO.new("1\n2\n")
    result = paste_parallel([io1, io2], [","], "\n")
    assert_equal ["a,1", "b,2"], result
  end

  def test_cyclic_delimiters
    io1 = StringIO.new("a\nb\n")
    io2 = StringIO.new("1\n2\n")
    io3 = StringIO.new("x\ny\n")
    result = paste_parallel([io1, io2, io3], [",", ":"], "\n")
    assert_equal ["a,1:x", "b,2:y"], result
  end

  def test_empty_input
    result = paste_parallel([], ["\t"], "\n")
    assert_equal [], result
  end

  def test_three_files
    io1 = StringIO.new("a\nb\n")
    io2 = StringIO.new("1\n2\n")
    io3 = StringIO.new("x\ny\n")
    result = paste_parallel([io1, io2, io3], ["\t"], "\n")
    assert_equal ["a\t1\tx", "b\t2\ty"], result
  end
end

# ===========================================================================
# Test: paste_serial
# ===========================================================================

class TestPasteSerial < Minitest::Test
  def test_single_file
    io1 = StringIO.new("a\nb\nc\n")
    result = paste_serial([io1], ["\t"], "\n")
    assert_equal ["a\tb\tc"], result
  end

  def test_two_files
    io1 = StringIO.new("a\nb\n")
    io2 = StringIO.new("1\n2\n")
    result = paste_serial([io1, io2], ["\t"], "\n")
    assert_equal ["a\tb", "1\t2"], result
  end

  def test_custom_delimiter
    io1 = StringIO.new("a\nb\nc\n")
    result = paste_serial([io1], [","], "\n")
    assert_equal ["a,b,c"], result
  end

  def test_cyclic_delimiters
    io1 = StringIO.new("a\nb\nc\nd\n")
    result = paste_serial([io1], [",", ":"], "\n")
    assert_equal ["a,b:c,d"], result
  end

  def test_empty_input
    io1 = StringIO.new("")
    result = paste_serial([io1], ["\t"], "\n")
    assert_equal [""], result
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestPasteMainFunction < Minitest::Test
  include PasteTestHelper

  def test_main_parallel_two_files
    with_tempfile("a\nb\n") do |path1|
      with_tempfile("1\n2\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { paste_main }[0]
        assert_equal "a\t1\nb\t2\n", output
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_serial_mode
    with_tempfile("a\nb\nc\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-s", path])
      output = capture_io { paste_main }[0]
      assert_equal "a\tb\tc\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_custom_delimiter
    with_tempfile("a\nb\n") do |path1|
      with_tempfile("1\n2\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace(["-d", ",", path1, path2])
        output = capture_io { paste_main }[0]
        assert_equal "a,1\nb,2\n", output
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { paste_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { paste_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
