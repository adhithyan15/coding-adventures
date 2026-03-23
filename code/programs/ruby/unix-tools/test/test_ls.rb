# frozen_string_literal: true

# test_ls.rb -- Tests for the Ruby ls tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the ls tool's CLI Builder integration and
# business logic. We test:
# - Listing directory contents
# - Hidden file filtering (-a, -A)
# - Long format (-l) with permissions, owner, size, date
# - Human-readable sizes (-h)
# - Sorting modes (-S, -t, -X, -U)
# - Reverse sort (-r)
# - Recursive listing (-R)
# - File classification (-F)
# - Entry formatting

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../ls_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for ls tests
# ---------------------------------------------------------------------------

module LsTestHelper
  LS_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "ls.json")

  def parse_ls_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(LS_TEST_SPEC, ["ls"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestLsCliIntegration < Minitest::Test
  include LsTestHelper

  def test_help_returns_help_result
    result = parse_ls_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_ls_argv(["--help"])
    assert_includes result.text, "ls"
  end

  def test_version_returns_version_result
    result = parse_ls_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse_no_args
    result = parse_ls_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_all_flag
    result = parse_ls_argv(["-a"])
    assert result.flags["all"]
  end

  def test_long_flag
    result = parse_ls_argv(["-l"])
    assert result.flags["long"]
  end

  def test_recursive_flag
    result = parse_ls_argv(["-R"])
    assert result.flags["recursive"]
  end
end

# ===========================================================================
# Test: ls_human_readable_size
# ===========================================================================

class TestLsHumanReadableSize < Minitest::Test
  def test_bytes_below_threshold
    assert_equal "500", ls_human_readable_size(500)
  end

  def test_kilobytes
    result = ls_human_readable_size(1536)
    assert_equal "1.5K", result
  end

  def test_megabytes
    result = ls_human_readable_size(1048576)
    assert_equal "1.0M", result
  end

  def test_gigabytes
    result = ls_human_readable_size(1073741824)
    assert_equal "1.0G", result
  end

  def test_si_mode
    # SI uses powers of 1000.
    result = ls_human_readable_size(1500, si: true)
    assert_equal "1.5kB", result
  end

  def test_zero_bytes
    assert_equal "0", ls_human_readable_size(0)
  end

  def test_exactly_1024
    result = ls_human_readable_size(1024)
    assert_equal "1.0K", result
  end
end

# ===========================================================================
# Test: ls_format_mode
# ===========================================================================

class TestLsFormatMode < Minitest::Test
  def test_regular_file_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "test.txt")
      File.write(path, "data")
      File.chmod(0o644, path)
      stat = File.lstat(path)
      mode = ls_format_mode(stat)
      assert_equal "-rw-r--r--", mode
    end
  end

  def test_executable_file_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "script.sh")
      File.write(path, "#!/bin/sh")
      File.chmod(0o755, path)
      stat = File.lstat(path)
      mode = ls_format_mode(stat)
      assert_equal "-rwxr-xr-x", mode
    end
  end

  def test_directory_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      stat = File.lstat(path)
      mode = ls_format_mode(stat)
      assert mode.start_with?("d"), "directory should start with 'd'"
    end
  end

  def test_symlink_mode
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "data")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      stat = File.lstat(link)
      mode = ls_format_mode(stat)
      assert mode.start_with?("l"), "symlink should start with 'l'"
    end
  end
end

# ===========================================================================
# Test: ls_classify_char
# ===========================================================================

class TestLsClassifyChar < Minitest::Test
  def test_directory_gets_slash
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      assert_equal "/", ls_classify_char(path)
    end
  end

  def test_executable_gets_star
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "script.sh")
      File.write(path, "#!/bin/sh")
      File.chmod(0o755, path)
      assert_equal "*", ls_classify_char(path)
    end
  end

  def test_symlink_gets_at
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "data")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      assert_equal "@", ls_classify_char(link)
    end
  end

  def test_regular_file_gets_nothing
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      assert_equal "", ls_classify_char(path)
    end
  end
end

# ===========================================================================
# Test: ls_format_entry
# ===========================================================================

class TestLsFormatEntry < Minitest::Test
  def test_short_format
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      result = ls_format_entry(path, "file.txt")
      assert_equal "file.txt", result
    end
  end

  def test_short_format_with_classify
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      result = ls_format_entry(path, "mydir", classify: true)
      assert_equal "mydir/", result
    end
  end

  def test_long_format_contains_permissions
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      result = ls_format_entry(path, "file.txt", long: true)
      assert_match(/^-rw/, result)
    end
  end

  def test_long_format_contains_size
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "hello")
      result = ls_format_entry(path, "file.txt", long: true)
      assert_includes result, "5"
    end
  end

  def test_long_format_human_readable
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "x" * 2048)
      result = ls_format_entry(path, "file.txt", long: true, human_readable: true)
      assert_includes result, "K"
    end
  end

  def test_long_format_no_group
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      result_with = ls_format_entry(path, "file.txt", long: true)
      result_without = ls_format_entry(path, "file.txt", long: true, no_group: true)
      # The no-group version should have fewer fields.
      assert result_without.split.length < result_with.split.length
    end
  end

  def test_inode_included
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      stat = File.lstat(path)
      result = ls_format_entry(path, "file.txt", inode: true)
      assert_includes result, stat.ino.to_s
    end
  end
end

# ===========================================================================
# Test: ls_list
# ===========================================================================

class TestLsList < Minitest::Test
  def test_list_directory_contents
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "a.txt"), "a")
      File.write(File.join(tmp, "b.txt"), "b")

      result = ls_list(tmp)

      assert_includes result, "a.txt"
      assert_includes result, "b.txt"
    end
  end

  def test_hides_dotfiles_by_default
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, ".hidden"), "secret")
      File.write(File.join(tmp, "visible.txt"), "public")

      result = ls_list(tmp)

      refute result.any? { |e| e.include?(".hidden") }
      assert result.any? { |e| e.include?("visible.txt") }
    end
  end

  def test_show_all_includes_dotfiles
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, ".hidden"), "secret")
      File.write(File.join(tmp, "visible.txt"), "public")

      result = ls_list(tmp, all: true)

      assert result.any? { |e| e.include?(".hidden") }
      assert result.any? { |e| e == "." }
      assert result.any? { |e| e == ".." }
    end
  end

  def test_almost_all_excludes_dot_and_dotdot
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, ".hidden"), "secret")

      result = ls_list(tmp, almost_all: true)

      assert result.any? { |e| e.include?(".hidden") }
      refute result.any? { |e| e == "." }
      refute result.any? { |e| e == ".." }
    end
  end

  def test_sort_by_size
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "small.txt"), "x")
      File.write(File.join(tmp, "large.txt"), "x" * 1000)

      result = ls_list(tmp, sort_by_size: true)

      large_idx = result.index { |e| e.include?("large.txt") }
      small_idx = result.index { |e| e.include?("small.txt") }
      assert large_idx < small_idx, "larger file should come first"
    end
  end

  def test_reverse_sort
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "aaa.txt"), "a")
      File.write(File.join(tmp, "zzz.txt"), "z")

      result = ls_list(tmp, reverse: true)

      z_idx = result.index { |e| e.include?("zzz.txt") }
      a_idx = result.index { |e| e.include?("aaa.txt") }
      assert z_idx < a_idx, "reverse should put z before a"
    end
  end

  def test_unsorted_returns_entries
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "b.txt"), "b")
      File.write(File.join(tmp, "a.txt"), "a")

      result = ls_list(tmp, unsorted: true)

      assert_equal 2, result.length
    end
  end

  def test_recursive_listing
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "top.txt"), "top")
      sub = File.join(tmp, "subdir")
      Dir.mkdir(sub)
      File.write(File.join(sub, "nested.txt"), "nested")

      result = ls_list(tmp, recursive: true)

      assert result.any? { |e| e.include?("top.txt") }
      assert result.any? { |e| e.include?("nested.txt") }
    end
  end

  def test_list_single_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")

      result = ls_list(path)

      assert_equal 1, result.length
      assert_includes result[0], "file.txt"
    end
  end

  def test_directory_flag
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "file.txt"), "data")

      result = ls_list(tmp, directory: true)

      assert_equal 1, result.length
      assert_includes result[0], tmp
    end
  end

  def test_sort_by_extension
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "file.rb"), "ruby")
      File.write(File.join(tmp, "file.py"), "python")
      File.write(File.join(tmp, "file.go"), "go")

      result = ls_list(tmp, sort_by_ext: true)

      go_idx = result.index { |e| e.include?("file.go") }
      py_idx = result.index { |e| e.include?("file.py") }
      rb_idx = result.index { |e| e.include?("file.rb") }
      assert go_idx < py_idx
      assert py_idx < rb_idx
    end
  end

  def test_sort_by_time
    Dir.mktmpdir do |tmp|
      old_file = File.join(tmp, "old.txt")
      new_file = File.join(tmp, "new.txt")
      File.write(old_file, "old")
      FileUtils.touch(old_file, mtime: Time.now - 100)
      File.write(new_file, "new")

      result = ls_list(tmp, sort_by_time: true)

      new_idx = result.index { |e| e.include?("new.txt") }
      old_idx = result.index { |e| e.include?("old.txt") }
      assert new_idx < old_idx, "newer file should come first"
    end
  end
end

# ===========================================================================
# Test: ls_sort_entries
# ===========================================================================

class TestLsSortEntries < Minitest::Test
  def test_default_alphabetical_sort
    Dir.mktmpdir do |tmp|
      entries = %w[charlie alice bob]
      result = ls_sort_entries(entries, tmp)
      assert_equal %w[alice bob charlie], result
    end
  end

  def test_case_insensitive_default_sort
    Dir.mktmpdir do |tmp|
      entries = %w[Charlie alice Bob]
      result = ls_sort_entries(entries, tmp)
      assert_equal %w[alice Bob Charlie], result
    end
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestLsMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { ls_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { ls_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_lists_directory
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "test_file.txt"), "data")
      old_argv = ARGV.dup
      ARGV.replace([tmp])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, "test_file.txt"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_long_format
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "file.txt"), "data")
      old_argv = ARGV.dup
      ARGV.replace(["-l", tmp])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_match(/^-[rwx-]{9}/, out)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_nonexistent_path_warns
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/path/xyz"])
    _, err = capture_io do
      begin
        ls_main
      rescue SystemExit
        # ls_main may exit
      end
    end
    assert_includes err, "cannot access"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_multiple_directories
    Dir.mktmpdir do |tmp|
      dir1 = File.join(tmp, "dir1")
      dir2 = File.join(tmp, "dir2")
      Dir.mkdir(dir1)
      Dir.mkdir(dir2)
      File.write(File.join(dir1, "a.txt"), "a")
      File.write(File.join(dir2, "b.txt"), "b")
      old_argv = ARGV.dup
      ARGV.replace([dir1, dir2])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, "a.txt"
      assert_includes out, "b.txt"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_classify_flag
    Dir.mktmpdir do |tmp|
      subdir = File.join(tmp, "mydir")
      Dir.mkdir(subdir)
      old_argv = ARGV.dup
      ARGV.replace(["-F", tmp])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, "mydir/"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_recursive_flag
    Dir.mktmpdir do |tmp|
      sub = File.join(tmp, "sub")
      Dir.mkdir(sub)
      File.write(File.join(sub, "nested.txt"), "n")
      old_argv = ARGV.dup
      ARGV.replace(["-R", tmp])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, "nested.txt"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_directory_flag
    Dir.mktmpdir do |tmp|
      File.write(File.join(tmp, "file.txt"), "data")
      old_argv = ARGV.dup
      ARGV.replace(["-d", tmp])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, tmp
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_single_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "single.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace([path])
      out, = capture_io do
        begin
          ls_main
        rescue SystemExit
          # ls_main may exit
        end
      end
      assert_includes out, "single.txt"
    ensure
      ARGV.replace(old_argv)
    end
  end
end

# ===========================================================================
# Test: ls_list additional coverage
# ===========================================================================

class TestLsListAdditional < Minitest::Test
  def test_long_format_with_numeric_uid_gid
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      result = ls_list(tmp, long: true, numeric_uid_gid: true)
      uid = File.lstat(path).uid.to_s
      assert result.any? { |line| line.include?(uid) }
    end
  end

  def test_long_format_with_classify
    Dir.mktmpdir do |tmp|
      subdir = File.join(tmp, "mydir")
      Dir.mkdir(subdir)
      File.write(File.join(tmp, "file.txt"), "data")
      result = ls_list(tmp, long: true, classify: true)
      assert result.any? { |line| line.include?("mydir/") }
    end
  end

  def test_long_format_with_si
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "x" * 2000)
      result = ls_list(tmp, long: true, human_readable: true, si: true)
      assert result.any? { |line| line.include?("kB") }
    end
  end
end
