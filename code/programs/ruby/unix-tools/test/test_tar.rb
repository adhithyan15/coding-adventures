# frozen_string_literal: true

# test_tar.rb -- Tests for the Ruby tar tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the tar tool's archive creation, extraction,
# and listing. We test:
# - Creating archives from files and directories
# - Listing archive contents
# - Extracting archives
# - Verbose output
# - Extracting to a specific directory (-C)
# - Keep old files (-k)
# - Preserve permissions (-p)
# - Strip components
# - Exclude patterns
# - Permission formatting
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "rubygems/package"
require "coding_adventures_cli_builder"

require_relative "../tar_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module TarTestHelper
  TAR_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "tar.json")

  def parse_tar_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TAR_TEST_SPEC, ["tar"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestTarCliIntegration < Minitest::Test
  include TarTestHelper

  def test_help_returns_help_result
    result = parse_tar_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_tar_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_create_flag
    result = parse_tar_argv(["-c", "-f", "archive.tar", "file"])
    assert result.flags["create"]
  end

  def test_extract_flag
    result = parse_tar_argv(["-x", "-f", "archive.tar"])
    assert result.flags["extract"]
  end

  def test_list_flag
    result = parse_tar_argv(["-t", "-f", "archive.tar"])
    assert result.flags["list"]
  end

  def test_verbose_flag
    result = parse_tar_argv(["-t", "-v", "-f", "archive.tar"])
    assert result.flags["verbose"]
  end
end

# ===========================================================================
# Test: tar_format_permissions
# ===========================================================================

class TestTarFormatPermissions < Minitest::Test
  def test_full_permissions
    assert_equal "drwxrwxrwx", tar_format_permissions(0o777, true)
  end

  def test_no_permissions
    assert_equal "d---------", tar_format_permissions(0o000, true)
  end

  def test_file_permissions
    assert_equal "-rwxr-xr-x", tar_format_permissions(0o755, false)
  end

  def test_read_only
    assert_equal "-r--r--r--", tar_format_permissions(0o444, false)
  end

  def test_write_only
    assert_equal "--w--w--w-", tar_format_permissions(0o222, false)
  end

  def test_typical_file
    assert_equal "-rw-r--r--", tar_format_permissions(0o644, false)
  end
end

# ===========================================================================
# Test: tar_create and tar_list
# ===========================================================================

class TestTarCreateAndList < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("tar_test")
    @archive = File.join(@tmpdir, "test.tar")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_create_single_file
    file = File.join(@tmpdir, "hello.txt")
    File.write(file, "hello world")

    msgs, ok = tar_create(@archive, ["hello.txt"], directory: @tmpdir)
    assert ok
    assert File.exist?(@archive)
    assert File.size(@archive) > 0
  end

  def test_create_verbose
    file = File.join(@tmpdir, "hello.txt")
    File.write(file, "hello world")

    msgs, ok = tar_create(@archive, ["hello.txt"], directory: @tmpdir, verbose: true)
    assert ok
    assert_includes msgs, "hello.txt"
  end

  def test_create_and_list
    file = File.join(@tmpdir, "hello.txt")
    File.write(file, "hello world")

    tar_create(@archive, ["hello.txt"], directory: @tmpdir)
    lines, ok = tar_list(@archive)
    assert ok
    assert_includes lines, "hello.txt"
  end

  def test_create_multiple_files
    %w[a.txt b.txt c.txt].each do |name|
      File.write(File.join(@tmpdir, name), "content of #{name}")
    end

    tar_create(@archive, %w[a.txt b.txt c.txt], directory: @tmpdir)
    lines, ok = tar_list(@archive)
    assert ok
    assert_includes lines, "a.txt"
    assert_includes lines, "b.txt"
    assert_includes lines, "c.txt"
  end

  def test_create_directory
    subdir = File.join(@tmpdir, "mydir")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "file.txt"), "hello")

    tar_create(@archive, ["mydir"], directory: @tmpdir)
    lines, ok = tar_list(@archive)
    assert ok
    dir_entries = lines.select { |l| l.include?("mydir") }
    assert dir_entries.length >= 2  # dir + file
  end

  def test_list_verbose
    file = File.join(@tmpdir, "hello.txt")
    File.write(file, "hello world")

    tar_create(@archive, ["hello.txt"], directory: @tmpdir)
    lines, ok = tar_list(@archive, nil, verbose: true)
    assert ok
    assert lines.any? { |l| l.include?("hello.txt") && l.include?("r") }
  end

  def test_list_with_filter
    %w[a.txt b.txt].each do |name|
      File.write(File.join(@tmpdir, name), "content")
    end

    tar_create(@archive, %w[a.txt b.txt], directory: @tmpdir)
    lines, ok = tar_list(@archive, ["a.txt"])
    assert ok
    assert_includes lines, "a.txt"
    refute_includes lines, "b.txt"
  end

  def test_list_nonexistent_archive
    lines, ok = tar_list(File.join(@tmpdir, "nonexistent.tar"))
    refute ok
    assert lines.any? { |l| l.include?("Cannot open") }
  end

  def test_create_with_exclude
    subdir = File.join(@tmpdir, "mydir")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "keep.txt"), "keep")
    File.write(File.join(subdir, "skip.bak"), "skip")

    tar_create(@archive, ["mydir"], directory: @tmpdir, exclude: ["*.bak"])
    lines, ok = tar_list(@archive)
    assert ok
    refute lines.any? { |l| l.include?("skip.bak") }
    assert lines.any? { |l| l.include?("keep.txt") }
  end
end

# ===========================================================================
# Test: tar_extract
# ===========================================================================

class TestTarExtract < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("tar_extract_test")
    @srcdir = File.join(@tmpdir, "src")
    @dstdir = File.join(@tmpdir, "dst")
    FileUtils.mkdir_p(@srcdir)
    FileUtils.mkdir_p(@dstdir)
    @archive = File.join(@tmpdir, "test.tar")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_extract_single_file
    File.write(File.join(@srcdir, "hello.txt"), "hello world")
    tar_create(@archive, ["hello.txt"], directory: @srcdir)

    msgs, ok = tar_extract(@archive, @dstdir)
    assert ok
    extracted = File.join(@dstdir, "hello.txt")
    assert File.exist?(extracted)
    assert_equal "hello world", File.read(extracted)
  end

  def test_extract_verbose
    File.write(File.join(@srcdir, "hello.txt"), "hello")
    tar_create(@archive, ["hello.txt"], directory: @srcdir)

    msgs, ok = tar_extract(@archive, @dstdir, nil, verbose: true)
    assert ok
    assert_includes msgs, "hello.txt"
  end

  def test_extract_directory
    subdir = File.join(@srcdir, "mydir")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "file.txt"), "content")

    tar_create(@archive, ["mydir"], directory: @srcdir)
    msgs, ok = tar_extract(@archive, @dstdir)
    assert ok
    assert File.exist?(File.join(@dstdir, "mydir", "file.txt"))
  end

  def test_extract_preserves_content
    content = "line1\nline2\nline3\n"
    File.write(File.join(@srcdir, "multi.txt"), content)
    tar_create(@archive, ["multi.txt"], directory: @srcdir)

    tar_extract(@archive, @dstdir)
    assert_equal content, File.read(File.join(@dstdir, "multi.txt"))
  end

  def test_extract_keep_old_files
    File.write(File.join(@srcdir, "file.txt"), "new content")
    tar_create(@archive, ["file.txt"], directory: @srcdir)

    # Pre-create the file in the destination
    existing = File.join(@dstdir, "file.txt")
    File.write(existing, "old content")

    msgs, ok = tar_extract(@archive, @dstdir, nil, keep_old_files: true)
    assert ok
    assert_equal "old content", File.read(existing)
  end

  def test_extract_overwrite_by_default
    File.write(File.join(@srcdir, "file.txt"), "new content")
    tar_create(@archive, ["file.txt"], directory: @srcdir)

    existing = File.join(@dstdir, "file.txt")
    File.write(existing, "old content")

    tar_extract(@archive, @dstdir)
    assert_equal "new content", File.read(existing)
  end

  def test_extract_specific_file
    File.write(File.join(@srcdir, "a.txt"), "aaa")
    File.write(File.join(@srcdir, "b.txt"), "bbb")
    tar_create(@archive, %w[a.txt b.txt], directory: @srcdir)

    tar_extract(@archive, @dstdir, ["a.txt"])
    assert File.exist?(File.join(@dstdir, "a.txt"))
    refute File.exist?(File.join(@dstdir, "b.txt"))
  end

  def test_extract_nonexistent_archive
    msgs, ok = tar_extract(File.join(@tmpdir, "nope.tar"), @dstdir)
    refute ok
  end

  def test_extract_strip_components
    subdir = File.join(@srcdir, "top", "sub")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "file.txt"), "deep")
    tar_create(@archive, ["top"], directory: @srcdir)

    msgs, ok = tar_extract(@archive, @dstdir, nil, strip_components: 1)
    assert ok
    # After stripping 1 component ("top"), we should have "sub/file.txt"
    assert File.exist?(File.join(@dstdir, "sub", "file.txt"))
  end

  def test_extract_preserve_permissions
    file = File.join(@srcdir, "exec.sh")
    File.write(file, "#!/bin/sh\necho hello")
    File.chmod(0o755, file)

    tar_create(@archive, ["exec.sh"], directory: @srcdir)
    tar_extract(@archive, @dstdir, nil, preserve_permissions: true)

    extracted = File.join(@dstdir, "exec.sh")
    assert File.exist?(extracted)
    mode = File.stat(extracted).mode & 0o7777
    assert_equal 0o755, mode
  end
end

# ===========================================================================
# Test: Round-trip integrity
# ===========================================================================

class TestTarRoundTrip < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("tar_roundtrip_test")
    @srcdir = File.join(@tmpdir, "src")
    @dstdir = File.join(@tmpdir, "dst")
    FileUtils.mkdir_p(@srcdir)
    FileUtils.mkdir_p(@dstdir)
    @archive = File.join(@tmpdir, "roundtrip.tar")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_roundtrip_multiple_files
    files = {
      "readme.txt" => "This is a readme",
      "data.csv" => "a,b,c\n1,2,3\n",
      "empty.txt" => "",
    }

    files.each { |name, content| File.write(File.join(@srcdir, name), content) }
    tar_create(@archive, files.keys, directory: @srcdir)
    tar_extract(@archive, @dstdir)

    files.each do |name, content|
      extracted = File.join(@dstdir, name)
      assert File.exist?(extracted), "#{name} should exist after extraction"
      assert_equal content, File.read(extracted), "#{name} content mismatch"
    end
  end

  def test_roundtrip_nested_directories
    FileUtils.mkdir_p(File.join(@srcdir, "a", "b", "c"))
    File.write(File.join(@srcdir, "a", "b", "c", "deep.txt"), "deep content")
    File.write(File.join(@srcdir, "a", "top.txt"), "top content")

    tar_create(@archive, ["a"], directory: @srcdir)
    tar_extract(@archive, @dstdir)

    assert_equal "deep content", File.read(File.join(@dstdir, "a", "b", "c", "deep.txt"))
    assert_equal "top content", File.read(File.join(@dstdir, "a", "top.txt"))
  end

  def test_roundtrip_binary_content
    binary = (0..255).map(&:chr).join
    File.binwrite(File.join(@srcdir, "binary.dat"), binary)

    tar_create(@archive, ["binary.dat"], directory: @srcdir)
    tar_extract(@archive, @dstdir)

    assert_equal binary, File.binread(File.join(@dstdir, "binary.dat"))
  end
end
