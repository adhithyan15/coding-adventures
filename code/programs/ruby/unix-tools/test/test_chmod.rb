# frozen_string_literal: true

# test_chmod.rb -- Tests for the Ruby chmod tool
# ================================================
#
# === What These Tests Verify ===
#
# These tests exercise the chmod tool's permission parsing and
# application. We test:
# - Octal mode parsing (755, 0644, etc.)
# - Symbolic mode parsing (u+rwx, go-w, a=r, etc.)
# - Combined symbolic modes (u+rw,go+r)
# - Special bits (setuid, setgid, sticky)
# - X permission (conditional execute)
# - Applying modes to files
# - Recursive mode changes (-R)
# - Verbose and changes output (-v, -c)
# - Silent mode (-f)
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../chmod_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module ChmodTestHelper
  CHMOD_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "chmod.json")

  def parse_chmod_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CHMOD_TEST_SPEC, ["chmod"] + argv).parse
  end

  # Get file permissions as an octal integer (lower 12 bits)
  def file_mode(path)
    File.stat(path).mode & 0o7777
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestChmodCliIntegration < Minitest::Test
  include ChmodTestHelper

  def test_help_returns_help_result
    result = parse_chmod_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_chmod_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_recursive_flag
    result = parse_chmod_argv(["-R", "755", "dir"])
    assert result.flags["recursive"]
  end

  def test_verbose_flag
    result = parse_chmod_argv(["-v", "755", "file"])
    assert result.flags["verbose"]
  end
end

# ===========================================================================
# Test: Octal mode parsing
# ===========================================================================

class TestChmodParseOctal < Minitest::Test
  def test_simple_octal
    mode, valid = chmod_parse_octal("755")
    assert valid
    assert_equal 0o755, mode
  end

  def test_four_digit_octal
    mode, valid = chmod_parse_octal("0644")
    assert valid
    assert_equal 0o644, mode
  end

  def test_minimal_octal
    mode, valid = chmod_parse_octal("0")
    assert valid
    assert_equal 0, mode
  end

  def test_max_octal
    mode, valid = chmod_parse_octal("7777")
    assert valid
    assert_equal 0o7777, mode
  end

  def test_invalid_octal_digit
    _mode, valid = chmod_parse_octal("789")
    refute valid
  end

  def test_non_numeric
    _mode, valid = chmod_parse_octal("abc")
    refute valid
  end

  def test_too_long
    _mode, valid = chmod_parse_octal("77777")
    refute valid
  end

  def test_empty
    _mode, valid = chmod_parse_octal("")
    refute valid
  end
end

# ===========================================================================
# Test: Symbolic mode parsing
# ===========================================================================

class TestChmodParseSymbolic < Minitest::Test
  def test_user_add_execute
    # Start with 0644 (rw-r--r--), add execute for user
    result = chmod_parse_symbolic("u+x", 0o644)
    assert_equal 0o744, result
  end

  def test_group_remove_write
    # Start with 0666, remove write for group
    result = chmod_parse_symbolic("g-w", 0o666)
    assert_equal 0o646, result
  end

  def test_other_set_read_only
    # Start with 0777, set other to read only
    result = chmod_parse_symbolic("o=r", 0o777)
    assert_equal 0o774, result
  end

  def test_all_add_read
    result = chmod_parse_symbolic("a+r", 0o000)
    assert_equal 0o444, result
  end

  def test_all_add_execute
    result = chmod_parse_symbolic("a+x", 0o000)
    assert_equal 0o111, result
  end

  def test_user_group_add_write
    result = chmod_parse_symbolic("ug+w", 0o444)
    assert_equal 0o664, result
  end

  def test_comma_separated_clauses
    # u+rwx,go+rx on 0000
    result = chmod_parse_symbolic("u+rwx,go+rx", 0o000)
    assert_equal 0o755, result
  end

  def test_remove_all_permissions
    result = chmod_parse_symbolic("a-rwx", 0o777)
    assert_equal 0o000, result
  end

  def test_set_exact_permissions
    result = chmod_parse_symbolic("u=rw,g=r,o=r", 0o777)
    assert_equal 0o644, result
  end

  def test_default_who_is_all
    # No who specified means "a" (all)
    result = chmod_parse_symbolic("+x", 0o644)
    assert_equal 0o755, result
  end

  def test_capital_x_on_directory
    result = chmod_parse_symbolic("a+X", 0o644, is_directory: true)
    assert_equal 0o755, result
  end

  def test_capital_x_on_file_without_execute
    # X on a file that doesn't already have execute: no change
    result = chmod_parse_symbolic("a+X", 0o644, is_directory: false)
    assert_equal 0o644, result
  end

  def test_capital_x_on_file_with_execute
    # X on a file that already has some execute bit: adds x
    result = chmod_parse_symbolic("a+X", 0o744, is_directory: false)
    assert_equal 0o755, result
  end

  def test_setuid
    result = chmod_parse_symbolic("u+s", 0o755)
    assert_equal 0o4755, result
  end

  def test_setgid
    result = chmod_parse_symbolic("g+s", 0o755)
    assert_equal 0o2755, result
  end

  def test_sticky_bit
    result = chmod_parse_symbolic("+t", 0o755)
    assert_equal 0o1755, result
  end
end

# ===========================================================================
# Test: chmod_parse_mode
# ===========================================================================

class TestChmodParseMode < Minitest::Test
  def test_octal_mode
    mode, valid = chmod_parse_mode("755")
    assert valid
    assert_equal 0o755, mode
  end

  def test_symbolic_mode
    mode, valid = chmod_parse_mode("u+x", 0o644)
    assert valid
    assert_equal 0o744, mode
  end

  def test_invalid_mode
    _mode, valid = chmod_parse_mode("xyz")
    refute valid
  end
end

# ===========================================================================
# Test: chmod_apply
# ===========================================================================

class TestChmodApply < Minitest::Test
  include ChmodTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chmod_test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_apply_octal_mode
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")
    File.chmod(0o644, path)

    msg, ok = chmod_apply(path, "755")
    assert ok
    assert_equal 0o755, file_mode(path)
  end

  def test_apply_symbolic_mode
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")
    File.chmod(0o644, path)

    msg, ok = chmod_apply(path, "u+x")
    assert ok
    assert_equal 0o744, file_mode(path)
  end

  def test_verbose_output
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")
    File.chmod(0o644, path)

    msg, ok = chmod_apply(path, "755", verbose: true)
    assert ok
    assert_includes msg, "mode of"
    assert_includes msg, "0644"
    assert_includes msg, "0755"
  end

  def test_changes_output_when_changed
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")
    File.chmod(0o644, path)

    msg, ok = chmod_apply(path, "755", changes: true)
    assert ok
    refute_nil msg
  end

  def test_changes_output_when_unchanged
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")
    File.chmod(0o755, path)

    msg, ok = chmod_apply(path, "755", changes: true)
    assert ok
    assert_nil msg
  end

  def test_nonexistent_file
    path = File.join(@tmpdir, "nonexistent.txt")
    msg, ok = chmod_apply(path, "755")
    refute ok
    assert_includes msg, "No such file"
  end

  def test_nonexistent_file_silent
    path = File.join(@tmpdir, "nonexistent.txt")
    msg, ok = chmod_apply(path, "755", silent: true)
    refute ok
    assert_nil msg
  end

  def test_invalid_mode
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")

    msg, ok = chmod_apply(path, "xyz")
    refute ok
    assert_includes msg, "invalid mode"
  end
end

# ===========================================================================
# Test: chmod_recursive
# ===========================================================================

class TestChmodRecursive < Minitest::Test
  include ChmodTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chmod_rec_test")
    @dir = File.join(@tmpdir, "testdir")
    FileUtils.mkdir_p(@dir)
    @file1 = File.join(@dir, "file1.txt")
    @file2 = File.join(@dir, "file2.txt")
    File.write(@file1, "hello")
    File.write(@file2, "world")
    File.chmod(0o644, @file1)
    File.chmod(0o644, @file2)
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_recursive_changes_all_files
    msgs, ok = chmod_recursive(@dir, "755")
    assert ok
    assert_equal 0o755, file_mode(@file1)
    assert_equal 0o755, file_mode(@file2)
  end

  def test_recursive_verbose
    msgs, ok = chmod_recursive(@dir, "755", verbose: true)
    assert ok
    refute_empty msgs
  end

  def test_recursive_on_single_file
    msgs, ok = chmod_recursive(@file1, "600")
    assert ok
    assert_equal 0o600, file_mode(@file1)
  end

  def test_recursive_with_subdirectory
    subdir = File.join(@dir, "sub")
    FileUtils.mkdir_p(subdir)
    subfile = File.join(subdir, "deep.txt")
    File.write(subfile, "deep")
    File.chmod(0o644, subfile)

    msgs, ok = chmod_recursive(@dir, "755")
    assert ok
    assert_equal 0o755, file_mode(subfile)
  end
end
