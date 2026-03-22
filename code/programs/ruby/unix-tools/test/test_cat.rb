# frozen_string_literal: true

# test_cat.rb -- Tests for the Ruby cat tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the cat tool's CLI Builder integration and
# business logic. We test:
# - File reading and concatenation
# - Line numbering (-n)
# - Non-blank line numbering (-b, overrides -n)
# - Blank line squeezing (-s)
# - Tab display (-T)
# - End-of-line markers (-E)
# - Non-printing character display (-v)
# - Show all (-A, equivalent to -vET)
# - The cat_stream and process_nonprinting functions directly

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

# Load the cat_tool module so we can test the business logic functions.
require_relative "../cat_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for cat tests
# ---------------------------------------------------------------------------

module CatTestHelper
  CAT_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "cat.json")

  def parse_cat_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CAT_TEST_SPEC, ["cat"] + argv).parse
  end

  # Create a temporary file with given content, yield its path, then clean up.
  def with_tempfile(content)
    f = Tempfile.new("cat_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end

  # Run cat_stream with given flags and input, return captured stdout.
  def run_cat_stream(input_text, flags = {})
    io = StringIO.new(input_text)
    output = capture_io { cat_stream(io, flags, 1) }[0]
    output
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestCatCliIntegration < Minitest::Test
  include CatTestHelper

  def test_no_flags_returns_parse_result
    result = parse_cat_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_cat_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "cat"
  end

  def test_version_returns_version_result
    result = parse_cat_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_n_flag_is_set
    result = parse_cat_argv(["-n"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["number"]
  end

  def test_b_flag_is_set
    result = parse_cat_argv(["-b"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["number_nonblank"]
  end

  def test_s_flag_is_set
    result = parse_cat_argv(["-s"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["squeeze_blank"]
  end

  def test_show_tabs_flag
    result = parse_cat_argv(["-T"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["show_tabs"]
  end

  def test_show_ends_flag
    result = parse_cat_argv(["-E"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["show_ends"]
  end

  def test_show_all_flag
    result = parse_cat_argv(["-A"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["show_all"]
  end
end

# ===========================================================================
# Test: process_nonprinting function
# ===========================================================================

class TestCatProcessNonprinting < Minitest::Test
  def test_normal_char_unchanged
    assert_equal "a", process_nonprinting("a")
  end

  def test_tab_unchanged
    assert_equal "\t", process_nonprinting("\t")
  end

  def test_null_byte
    assert_equal "^@", process_nonprinting("\x00")
  end

  def test_control_a
    assert_equal "^A", process_nonprinting("\x01")
  end

  def test_escape_char
    assert_equal "^[", process_nonprinting("\x1b")
  end

  def test_del_char
    assert_equal "^?", process_nonprinting("\x7f")
  end

  def test_high_byte
    # 0x80 + 0 = M-^@ (128 < 128+32)
    assert_equal "M-^@", process_nonprinting("\x80".b)
  end

  def test_high_byte_printable
    # 0xC0 = 192 = 128+64, which is >= 128+32 and != 128+127
    # So it should be M- followed by chr(192-128) = chr(64) = '@'
    assert_equal "M-@", process_nonprinting("\xC0".b)
  end

  def test_high_del
    # 0xFF = 128+127 = M-^?
    assert_equal "M-^?", process_nonprinting("\xFF".b)
  end
end

# ===========================================================================
# Test: cat_stream basic output
# ===========================================================================

class TestCatStreamBasic < Minitest::Test
  include CatTestHelper

  def test_simple_text
    output = run_cat_stream("hello\nworld\n")
    assert_equal "hello\nworld\n", output
  end

  def test_empty_input
    output = run_cat_stream("")
    assert_equal "", output
  end

  def test_single_line_no_newline
    output = run_cat_stream("hello")
    assert_equal "hello\n", output
  end
end

# ===========================================================================
# Test: cat_stream with -n (number all lines)
# ===========================================================================

class TestCatStreamNumbering < Minitest::Test
  include CatTestHelper

  def test_number_lines
    output = run_cat_stream("alpha\nbeta\ngamma\n", { "number" => true })
    lines = output.split("\n")
    assert_equal 3, lines.length
    assert_match(/^\s+1\t/, lines[0])
    assert_match(/^\s+2\t/, lines[1])
    assert_match(/^\s+3\t/, lines[2])
  end

  def test_number_includes_blank_lines
    output = run_cat_stream("alpha\n\nbeta\n", { "number" => true })
    lines = output.split("\n")
    assert_equal 3, lines.length
    assert_match(/^\s+1\t/, lines[0])
    assert_match(/^\s+2\t/, lines[1])
    assert_match(/^\s+3\t/, lines[2])
  end
end

# ===========================================================================
# Test: cat_stream with -b (number non-blank lines)
# ===========================================================================

class TestCatStreamNumberNonblank < Minitest::Test
  include CatTestHelper

  def test_number_nonblank_lines
    output = run_cat_stream("alpha\n\nbeta\n", { "number_nonblank" => true })
    lines = output.split("\n")
    # Line 1: "     1\talpha"
    assert_match(/^\s+1\talpha/, lines[0])
    # Line 2: "" (blank, not numbered)
    assert_equal "", lines[1]
    # Line 3: "     2\tbeta"
    assert_match(/^\s+2\tbeta/, lines[2])
  end

  def test_b_overrides_n
    # -b should override -n: blank lines should NOT be numbered.
    output = run_cat_stream("alpha\n\nbeta\n", { "number" => true, "number_nonblank" => true })
    lines = output.split("\n")
    assert_equal "", lines[1]
  end
end

# ===========================================================================
# Test: cat_stream with -s (squeeze blank)
# ===========================================================================

class TestCatStreamSqueeze < Minitest::Test
  include CatTestHelper

  def test_squeeze_multiple_blanks
    output = run_cat_stream("alpha\n\n\n\nbeta\n", { "squeeze_blank" => true })
    lines = output.split("\n")
    assert_equal 3, lines.length
    assert_equal "alpha", lines[0]
    assert_equal "", lines[1]
    assert_equal "beta", lines[2]
  end

  def test_no_squeeze_single_blank
    output = run_cat_stream("alpha\n\nbeta\n", { "squeeze_blank" => true })
    lines = output.split("\n")
    assert_equal 3, lines.length
  end
end

# ===========================================================================
# Test: cat_stream with -T (show tabs)
# ===========================================================================

class TestCatStreamShowTabs < Minitest::Test
  include CatTestHelper

  def test_tabs_shown_as_caret_I
    output = run_cat_stream("hello\tworld\n", { "show_tabs" => true })
    assert_equal "hello^Iworld\n", output
  end
end

# ===========================================================================
# Test: cat_stream with -E (show ends)
# ===========================================================================

class TestCatStreamShowEnds < Minitest::Test
  include CatTestHelper

  def test_dollar_at_end_of_line
    output = run_cat_stream("hello\nworld\n", { "show_ends" => true })
    lines = output.split("\n")
    assert_equal "hello$", lines[0]
    assert_equal "world$", lines[1]
  end
end

# ===========================================================================
# Test: cat_stream with -A (show all = -vET)
# ===========================================================================

class TestCatStreamShowAll < Minitest::Test
  include CatTestHelper

  def test_show_all_enables_tabs_and_ends
    output = run_cat_stream("hello\tworld\n", { "show_all" => true })
    # -A enables -v, -E, and -T
    assert_includes output, "^I"   # tabs shown
    assert_includes output, "$"    # ends shown
  end
end

# ===========================================================================
# Test: Main function with file reading
# ===========================================================================

class TestCatMainFunction < Minitest::Test
  include CatTestHelper

  def test_main_reads_file
    with_tempfile("hello world\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { cat_main }[0]
      assert_equal "hello world\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_reads_multiple_files
    with_tempfile("aaa\n") do |path1|
      with_tempfile("bbb\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { cat_main }[0]
        assert_equal "aaa\nbbb\n", output
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file_prints_error
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz"])
    _stdout, stderr = capture_io { cat_main }
    assert_includes stderr, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { cat_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { cat_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_with_n_flag
    with_tempfile("alpha\nbeta\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-n", path])
      output = capture_io { cat_main }[0]
      assert_match(/^\s+1\talpha/, output)
      assert_match(/^\s+2\tbeta/, output)
    ensure
      ARGV.replace(old_argv)
    end
  end
end
