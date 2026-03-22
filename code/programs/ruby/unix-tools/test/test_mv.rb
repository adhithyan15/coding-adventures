# frozen_string_literal: true

# test_mv.rb -- Tests for the Ruby mv tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the mv tool's CLI Builder integration and
# business logic. We test:
# - Moving/renaming a single file
# - Moving into a directory
# - No-clobber mode (-n)
# - Update mode (-u)
# - Verbose output (-v)
# - Force mode (-f)
# - Moving directories
# - Error handling for nonexistent sources

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../mv_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for mv tests
# ---------------------------------------------------------------------------

module MvTestHelper
  MV_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "mv.json")

  def parse_mv_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(MV_TEST_SPEC, ["mv"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestMvCliIntegration < Minitest::Test
  include MvTestHelper

  def test_help_returns_help_result
    result = parse_mv_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_mv_argv(["--help"])
    assert_includes result.text, "mv"
  end

  def test_version_returns_version_result
    result = parse_mv_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse
    result = parse_mv_argv(["src.txt", "dst.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_force_flag
    result = parse_mv_argv(["-f", "src.txt", "dst.txt"])
    assert result.flags["force"]
  end

  def test_no_clobber_flag
    result = parse_mv_argv(["-n", "src.txt", "dst.txt"])
    assert result.flags["no_clobber"]
  end

  def test_verbose_flag
    result = parse_mv_argv(["-v", "src.txt", "dst.txt"])
    assert result.flags["verbose"]
  end

  def test_update_flag
    result = parse_mv_argv(["-u", "src.txt", "dst.txt"])
    assert result.flags["update"]
  end
end

# ===========================================================================
# Test: mv_move
# ===========================================================================

class TestMvMove < Minitest::Test
  def test_rename_file
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "old.txt")
      dst = File.join(tmp, "new.txt")
      File.write(src, "content")

      mv_move(src, dst)

      refute File.exist?(src), "source should be gone after move"
      assert File.exist?(dst)
      assert_equal "content", File.read(dst)
    end
  end

  def test_move_file_into_directory
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "file.txt")
      dst_dir = File.join(tmp, "target_dir")
      Dir.mkdir(dst_dir)
      File.write(src, "data")

      mv_move(src, dst_dir)

      refute File.exist?(src)
      assert File.exist?(File.join(dst_dir, "file.txt"))
      assert_equal "data", File.read(File.join(dst_dir, "file.txt"))
    end
  end

  def test_move_overwrites_by_default
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "new content")
      File.write(dst, "old content")

      mv_move(src, dst)

      assert_equal "new content", File.read(dst)
      refute File.exist?(src)
    end
  end

  def test_no_clobber_skips_existing
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "new content")
      File.write(dst, "old content")

      result = mv_move(src, dst, no_clobber: true)

      assert_nil result
      assert_equal "old content", File.read(dst)
      assert File.exist?(src), "source should still exist with -n"
    end
  end

  def test_update_skips_when_dest_is_newer
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(dst, "newer content")
      File.write(src, "older content")
      FileUtils.touch(src, mtime: Time.now - 100)

      result = mv_move(src, dst, update: true)

      assert_nil result
      assert_equal "newer content", File.read(dst)
      assert File.exist?(src)
    end
  end

  def test_update_moves_when_source_is_newer
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(dst, "old content")
      FileUtils.touch(dst, mtime: Time.now - 100)
      File.write(src, "new content")

      mv_move(src, dst, update: true)

      assert_equal "new content", File.read(dst)
      refute File.exist?(src)
    end
  end

  def test_verbose_returns_message
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "data")

      msg = mv_move(src, dst, verbose: true)

      assert_includes msg, "renamed"
      assert_includes msg, src
    end
  end

  def test_non_verbose_returns_nil
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "dest.txt")
      File.write(src, "data")

      result = mv_move(src, dst, verbose: false)

      assert_nil result
    end
  end

  def test_nonexistent_source_raises
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "nonexistent.txt")
      dst = File.join(tmp, "dest.txt")

      err = assert_raises(RuntimeError) { mv_move(src, dst) }
      assert_includes err.message, "No such file or directory"
    end
  end

  def test_move_directory
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "srcdir")
      Dir.mkdir(src)
      File.write(File.join(src, "file.txt"), "nested")
      dst = File.join(tmp, "dstdir")

      mv_move(src, dst)

      refute File.exist?(src)
      assert File.directory?(dst)
      assert_equal "nested", File.read(File.join(dst, "file.txt"))
    end
  end

  def test_no_target_directory_flag
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "source.txt")
      dst = File.join(tmp, "target_dir")
      Dir.mkdir(dst)
      File.write(src, "content")

      # With no_target_directory, the file replaces the directory path.
      # FileUtils.mv handles this by moving src to dst.
      mv_move(src, dst, no_target_directory: true, force: true)
    end
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestMvMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { mv_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { mv_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_moves_file
    Dir.mktmpdir do |tmp|
      src = File.join(tmp, "src.txt")
      dst = File.join(tmp, "dst.txt")
      File.write(src, "main test")

      old_argv = ARGV.dup
      ARGV.replace([src, dst])
      assert_raises(SystemExit) do
        capture_io { mv_main }
      end
      assert_equal "main test", File.read(dst)
      refute File.exist?(src)
    ensure
      ARGV.replace(old_argv)
    end
  end
end
