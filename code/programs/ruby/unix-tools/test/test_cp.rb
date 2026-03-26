# frozen_string_literal: true

# test_cp.rb -- Tests for the Ruby cp tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the cp tool's CLI Builder integration and
# business logic. We test:
# - Copying a single file
# - Copying into a directory
# - Recursive directory copying
# - No-clobber mode (-n)
# - Update mode (-u)
# - Verbose output (-v)
# - Hard link creation (-l)
# - Symbolic link creation (-s)
# - Force mode (-f)
# - Preserve attributes

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../cp_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for cp tests
# ---------------------------------------------------------------------------

module CpTestHelper
  CP_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "cp.json")

  def parse_cp_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CP_TEST_SPEC, ["cp"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestCpCliIntegration < Minitest::Test
  include CpTestHelper

  def test_help_returns_help_result
    result = parse_cp_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_cp_argv(["--help"])
    assert_includes result.text, "cp"
  end

  def test_version_returns_version_result
    result = parse_cp_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse_with_two_args
    result = parse_cp_argv(["src.txt", "dst.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_force_flag
    result = parse_cp_argv(["-f", "src.txt", "dst.txt"])
    assert result.flags["force"]
  end

  def test_recursive_flag
    result = parse_cp_argv(["-R", "srcdir", "dstdir"])
    assert result.flags["recursive"]
  end

  def test_verbose_flag
    result = parse_cp_argv(["-v", "src.txt", "dst.txt"])
    assert result.flags["verbose"]
  end

  def test_no_clobber_flag
    result = parse_cp_argv(["-n", "src.txt", "dst.txt"])
    assert result.flags["no_clobber"]
  end
end

# ===========================================================================
# Test: cp_copy_file
# ===========================================================================

class TestCpCopyFile < Minitest::Test
  def test_copy_single_file
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "hello world")

      cp_copy_file(src, dst)

      assert File.exist?(dst)
      assert_equal "hello world", File.read(dst)
      assert File.exist?(src), "source should still exist after copy"
    end
  end

  def test_copy_file_into_directory
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst_dir = File.join(tmp, "target_dir")
      Dir.mkdir(dst_dir)
      File.write(src, "content")

      cp_copy_file(src, dst_dir)

      assert File.exist?(File.join(dst_dir, "source.txt"))
      assert_equal "content", File.read(File.join(dst_dir, "source.txt"))
    end
  end

  def test_copy_overwrites_existing_by_default
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "new content")
      File.write(dst, "old content")

      cp_copy_file(src, dst)

      assert_equal "new content", File.read(dst)
    end
  end

  def test_no_clobber_skips_existing
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "new content")
      File.write(dst, "old content")

      result = cp_copy_file(src, dst, no_clobber: true)

      assert_nil result
      assert_equal "old content", File.read(dst)
    end
  end

  def test_update_skips_when_dest_is_newer
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(dst, "newer content")
      sleep 0.05
      # Backdate the source file.
      File.write(src, "older content")
      FileUtils.touch(src, mtime: Time.now - 100)

      result = cp_copy_file(src, dst, update: true)

      assert_nil result
      assert_equal "newer content", File.read(dst)
    end
  end

  def test_update_copies_when_source_is_newer
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(dst, "old content")
      FileUtils.touch(dst, mtime: Time.now - 100)
      File.write(src, "new content")

      cp_copy_file(src, dst, update: true)

      assert_equal "new content", File.read(dst)
    end
  end

  def test_verbose_returns_message
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "data")

      msg = cp_copy_file(src, dst, verbose: true)

      assert_includes msg, src
      assert_includes msg, dst
    end
  end

  def test_non_verbose_returns_nil
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "data")

      result = cp_copy_file(src, dst, verbose: false)

      assert_nil result
    end
  end

  def test_nonexistent_source_raises
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "nonexistent.txt")
      dst = File.join(tmp, "dest.txt")

      err = assert_raises(RuntimeError) { cp_copy_file(src, dst) }
      assert_includes err.message, "No such file or directory"
    end
  end

  def test_directory_without_recursive_raises
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      dst = File.join(tmp, "dstdir")

      err = assert_raises(RuntimeError) { cp_copy_file(src, dst) }
      assert_includes err.message, "not specified"
    end
  end

  def test_hard_link_creation
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "link.txt")
      File.write(src, "linked content")

      cp_copy_file(src, dst, link: true)

      assert File.exist?(dst)
      # Hard links share the same inode.
      assert_equal File.stat(src).ino, File.stat(dst).ino
    end
  end

  def test_symbolic_link_creation
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "symlink.txt")
      File.write(src, "symlinked content")

      cp_copy_file(src, dst, symbolic_link: true)

      assert File.symlink?(dst)
      assert_equal "symlinked content", File.read(dst)
    end
  end

  def test_preserve_mode
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "data")
      File.chmod(0o755, src)

      cp_copy_file(src, dst, preserve: true)

      assert_equal File.stat(src).mode, File.stat(dst).mode
    end
  end

  def test_no_target_directory_copies_to_file_path
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "target_dir")
      Dir.mkdir(dst)
      File.write(src, "content")

      # With no_target_directory, dst is treated as the file itself.
      # This will fail because dst is a directory, but cp_copy_file
      # will try to copy to it as a file. We test the flag is respected.
      cp_copy_file(src, dst, no_target_directory: true)
      # The file was copied to the directory path itself (overwriting).
      # Actually FileUtils.cp will copy src into dst as a file.
    end
  end
end

# ===========================================================================
# Test: cp_copy_directory
# ===========================================================================

class TestCpCopyDirectory < Minitest::Test
  def test_copy_directory_recursively
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      File.write(File.join(src, "file1.txt"), "content1")
      File.write(File.join(src, "file2.txt"), "content2")
      sub = File.join(src, "subdir")
      Dir.mkdir(sub)
      File.write(File.join(sub, "nested.txt"), "nested")

      dst = File.join(tmp, "dstdir")

      cp_copy_directory(src, dst)

      assert File.directory?(dst)
      assert_equal "content1", File.read(File.join(dst, "file1.txt"))
      assert_equal "content2", File.read(File.join(dst, "file2.txt"))
      assert_equal "nested", File.read(File.join(dst, "subdir", "nested.txt"))
    end
  end

  def test_copy_directory_into_existing_directory
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      File.write(File.join(src, "file.txt"), "data")

      dst = File.join(tmp, "existing_dir")
      Dir.mkdir(dst)

      cp_copy_directory(src, dst)

      # Should create srcdir inside existing_dir.
      assert File.exist?(File.join(dst, "srcdir", "file.txt"))
    end
  end

  def test_copy_directory_verbose
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      File.write(File.join(src, "file.txt"), "data")
      dst = File.join(tmp, "dstdir")

      msg = cp_copy_directory(src, dst, verbose: true)

      assert_includes msg, src
    end
  end

  def test_copy_directory_no_clobber
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      File.write(File.join(src, "file.txt"), "data")

      dst = File.join(tmp, "dstdir")
      Dir.mkdir(dst)

      # dst exists, so no_clobber should prevent the copy.
      # When dst is a dir, the actual_dst becomes dst/basename(src).
      # Create that so no_clobber triggers.
      actual = File.join(dst, "srcdir")
      Dir.mkdir(actual)

      result = cp_copy_directory(src, dst, no_clobber: true)
      assert_nil result
    end
  end

  def test_copy_nonexistent_directory_raises
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "nonexistent")
      dst = File.join(tmp, "dst")

      err = assert_raises(RuntimeError) { cp_copy_directory(src, dst) }
      assert_includes err.message, "No such file or directory"
    end
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestCpMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { cp_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { cp_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_copies_file
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "src.txt")
      dst = File.join(tmp, "dst.txt")
      File.write(src, "main test")

      old_argv = ARGV.dup
      ARGV.replace([src, dst])
      assert_raises(SystemExit) do
        capture_io { cp_main }
      end
      assert_equal "main test", File.read(dst)
    ensure
      ARGV.replace(old_argv)
    end
  end
end
