# frozen_string_literal: true

# test_xargs.rb -- Tests for the Ruby xargs tool
# =================================================
#
# === What These Tests Verify ===
#
# These tests exercise the xargs tool's input parsing, batch building,
# and command execution. We test:
# - Default whitespace splitting with quote handling
# - Null-delimited input (-0)
# - Custom delimiter (-d)
# - Max args batching (-n)
# - Replace mode (-I)
# - No-run-if-empty (-r)
# - Verbose mode (-t)
# - Command execution with exit codes
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

require_relative "../xargs_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module XargsTestHelper
  XARGS_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "xargs.json")

  def parse_xargs_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(XARGS_TEST_SPEC, ["xargs"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestXargsCliIntegration < Minitest::Test
  include XargsTestHelper

  def test_help_returns_help_result
    result = parse_xargs_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_xargs_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_null_flag
    result = parse_xargs_argv(["-0", "echo"])
    assert result.flags["null"]
  end

  def test_max_args_flag
    result = parse_xargs_argv(["-n", "3", "echo"])
    assert_equal 3, result.flags["max_args"]
  end
end

# ===========================================================================
# Test: Input parsing
# ===========================================================================

class TestXargsParseItems < Minitest::Test
  def test_default_whitespace_split
    items = xargs_parse_items("hello world foo")
    assert_equal %w[hello world foo], items
  end

  def test_newline_split
    items = xargs_parse_items("hello\nworld\nfoo")
    assert_equal %w[hello world foo], items
  end

  def test_mixed_whitespace
    items = xargs_parse_items("  hello   world\n\tfoo  ")
    assert_equal %w[hello world foo], items
  end

  def test_single_quoted_argument
    items = xargs_parse_items("hello 'world foo' bar")
    assert_equal ["hello", "world foo", "bar"], items
  end

  def test_double_quoted_argument
    items = xargs_parse_items('hello "world foo" bar')
    assert_equal ["hello", "world foo", "bar"], items
  end

  def test_null_delimited
    items = xargs_parse_items("hello\0world\0foo", null: true)
    assert_equal %w[hello world foo], items
  end

  def test_null_ignores_empty
    items = xargs_parse_items("hello\0\0world\0", null: true)
    assert_equal %w[hello world], items
  end

  def test_custom_delimiter
    items = xargs_parse_items("hello,world,foo", delimiter: ",")
    assert_equal %w[hello world foo], items
  end

  def test_custom_delimiter_newline_escape
    items = xargs_parse_items("hello\nworld\nfoo", delimiter: "\\n")
    assert_equal %w[hello world foo], items
  end

  def test_empty_input
    items = xargs_parse_items("")
    assert_empty items
  end

  def test_whitespace_only_input
    items = xargs_parse_items("   \n\t  ")
    assert_empty items
  end
end

# ===========================================================================
# Test: Batch building
# ===========================================================================

class TestXargsBuildBatches < Minitest::Test
  def test_default_single_batch
    batches = xargs_build_batches(%w[a b c], ["echo"])
    assert_equal 1, batches.length
    assert_equal ["echo", "a", "b", "c"], batches[0]
  end

  def test_max_args_splits_batches
    batches = xargs_build_batches(%w[a b c d e], ["echo"], max_args: 2)
    assert_equal 3, batches.length
    assert_equal ["echo", "a", "b"], batches[0]
    assert_equal ["echo", "c", "d"], batches[1]
    assert_equal ["echo", "e"], batches[2]
  end

  def test_replace_mode
    batches = xargs_build_batches(%w[a b], ["cp", "{}", "/backup/"], replace: "{}")
    assert_equal 2, batches.length
    assert_equal ["cp", "a", "/backup/"], batches[0]
    assert_equal ["cp", "b", "/backup/"], batches[1]
  end

  def test_default_command_is_echo
    batches = xargs_build_batches(%w[a b], nil)
    assert_equal 1, batches.length
    assert_equal ["/bin/echo", "a", "b"], batches[0]
  end

  def test_empty_command_defaults_to_echo
    batches = xargs_build_batches(%w[a b], [])
    assert_equal ["/bin/echo", "a", "b"], batches[0]
  end

  def test_empty_items_single_batch
    batches = xargs_build_batches([], ["echo"])
    assert_equal 1, batches.length
    assert_equal ["echo"], batches[0]
  end

  def test_max_args_one
    batches = xargs_build_batches(%w[a b c], ["echo"], max_args: 1)
    assert_equal 3, batches.length
    batches.each_with_index do |batch, i|
      assert_equal ["echo", %w[a b c][i]], batch
    end
  end
end

# ===========================================================================
# Test: Command execution
# ===========================================================================

class TestXargsExecute < Minitest::Test
  def test_successful_command
    batches = [["true"]]
    code = xargs_execute(batches, {})
    assert_equal 0, code
  end

  def test_failing_command
    batches = [["false"]]
    code = xargs_execute(batches, {})
    assert_equal 123, code
  end

  def test_nonexistent_command
    batches = [["nonexistent_command_12345"]]
    io_err = StringIO.new
    code = xargs_execute(batches, {}, io_err: io_err)
    assert_equal 127, code
  end

  def test_verbose_mode
    io_err = StringIO.new
    batches = [["true"]]
    xargs_execute(batches, { verbose: true }, io_err: io_err)
    assert_includes io_err.string, "true"
  end

  def test_empty_batches
    code = xargs_execute([], {})
    assert_equal 0, code
  end
end

# ===========================================================================
# Test: xargs_run integration
# ===========================================================================

class TestXargsRun < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("xargs_test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_run_with_echo
    io_err = StringIO.new
    # Use touch to create files -- verifiable
    file1 = File.join(@tmpdir, "file1")
    file2 = File.join(@tmpdir, "file2")
    code = xargs_run("#{file1}\n#{file2}", ["touch"], {}, io_err: io_err)
    assert_equal 0, code
    assert File.exist?(file1)
    assert File.exist?(file2)
  end

  def test_no_run_if_empty
    io_err = StringIO.new
    code = xargs_run("", ["false"], { no_run_if_empty: true }, io_err: io_err)
    assert_equal 0, code
  end

  def test_run_empty_without_no_run
    io_err = StringIO.new
    # Default behavior: still runs the command once with no args
    code = xargs_run("", ["true"], {}, io_err: io_err)
    assert_equal 0, code
  end

  def test_replace_with_empty_items
    io_err = StringIO.new
    code = xargs_run("", ["echo", "{}"], { replace: "{}" }, io_err: io_err)
    assert_equal 0, code
  end

  def test_null_delimited_input
    io_err = StringIO.new
    file1 = File.join(@tmpdir, "file1")
    code = xargs_run("#{file1}\0", ["touch"], { null: true }, io_err: io_err)
    assert_equal 0, code
    assert File.exist?(file1)
  end

  def test_max_args_batching
    io_err = StringIO.new
    f1 = File.join(@tmpdir, "f1")
    f2 = File.join(@tmpdir, "f2")
    f3 = File.join(@tmpdir, "f3")
    code = xargs_run("#{f1}\n#{f2}\n#{f3}", ["touch"], { max_args: 1 }, io_err: io_err)
    assert_equal 0, code
    assert File.exist?(f1)
    assert File.exist?(f2)
    assert File.exist?(f3)
  end

  def test_verbose_shows_commands
    io_err = StringIO.new
    code = xargs_run("hello", ["echo"], { verbose: true }, io_err: io_err)
    assert_equal 0, code
    assert_includes io_err.string, "echo"
  end

  def test_replace_mode_integration
    io_err = StringIO.new
    f1 = File.join(@tmpdir, "src")
    File.write(f1, "data")
    dest = File.join(@tmpdir, "dest")
    code = xargs_run(f1, ["cp", "{}", dest], { replace: "{}" }, io_err: io_err)
    assert_equal 0, code
    assert File.exist?(dest)
  end

  def test_custom_delimiter_integration
    io_err = StringIO.new
    f1 = File.join(@tmpdir, "d1")
    f2 = File.join(@tmpdir, "d2")
    code = xargs_run("#{f1},#{f2}", ["touch"], { delimiter: "," }, io_err: io_err)
    assert_equal 0, code
    assert File.exist?(f1)
    assert File.exist?(f2)
  end
end
