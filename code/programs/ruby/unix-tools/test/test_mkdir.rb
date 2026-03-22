# frozen_string_literal: true

# test_mkdir.rb -- Tests for the Ruby mkdir tool
# ================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../mkdir_tool"

module MkdirTestHelper
  MKDIR_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "mkdir.json")

  def parse_mkdir_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(MKDIR_TEST_SPEC, ["mkdir"] + argv).parse
  end
end

class TestMkdirCliIntegration < Minitest::Test
  include MkdirTestHelper

  def test_basic_parse
    result = parse_mkdir_argv(["testdir"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_mkdir_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "mkdir"
  end

  def test_version
    result = parse_mkdir_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_parents_flag
    result = parse_mkdir_argv(["-p", "a/b/c"])
    assert result.flags["parents"]
  end

  def test_verbose_flag
    result = parse_mkdir_argv(["-v", "testdir"])
    assert result.flags["verbose"]
  end

  def test_mode_flag
    result = parse_mkdir_argv(["-m", "755", "testdir"])
    assert_equal "755", result.flags["mode"]
  end
end

class TestMkdirCreateDirectory < Minitest::Test
  def test_create_single_directory
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "newdir")
      assert mkdir_create_directory(path, parents: false, mode: nil, verbose: false)
      assert File.directory?(path)
    end
  end

  def test_create_with_parents
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "a", "b", "c")
      assert mkdir_create_directory(path, parents: true, mode: nil, verbose: false)
      assert File.directory?(path)
    end
  end

  def test_create_fails_without_parents
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "x", "y", "z")
      refute mkdir_create_directory(path, parents: false, mode: nil, verbose: false)
    end
  end

  def test_create_existing_fails_without_parents
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "existing")
      Dir.mkdir(path)
      refute mkdir_create_directory(path, parents: false, mode: nil, verbose: false)
    end
  end

  def test_verbose_output
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "verbosedir")
      output = capture_io { mkdir_create_directory(path, parents: false, mode: nil, verbose: true) }[0]
      assert_includes output, "created directory"
    end
  end
end

class TestMkdirParseMode < Minitest::Test
  def test_octal_755
    assert_equal 0o755, mkdir_parse_mode("755")
  end

  def test_octal_700
    assert_equal 0o700, mkdir_parse_mode("700")
  end

  def test_invalid_mode
    assert_nil mkdir_parse_mode("xyz")
  end
end

class TestMkdirCreateDirectoryEdgeCases < Minitest::Test
  def test_create_with_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "modedir")
      assert mkdir_create_directory(path, parents: false, mode: 0o700, verbose: false)
      assert File.directory?(path)
      actual_mode = File.stat(path).mode & 0o777
      assert_equal 0o700, actual_mode
    end
  end

  def test_create_with_parents_and_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "a", "b", "c")
      assert mkdir_create_directory(path, parents: true, mode: 0o755, verbose: false)
      assert File.directory?(path)
    end
  end

  def test_verbose_with_parents
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "verbosep")
      out, _err = capture_io do
        mkdir_create_directory(path, parents: true, mode: nil, verbose: true)
      end
      assert_includes out, "created directory"
    end
  end

  def test_existing_dir_with_parents_succeeds
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "existing")
      Dir.mkdir(path)
      # parents mode silently succeeds for existing dirs
      assert mkdir_create_directory(path, parents: true, mode: nil, verbose: false)
    end
  end
end

class TestMkdirMainIntegration < Minitest::Test
  def test_main_creates_directory
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "maindir")
      old_argv = ARGV.dup
      ARGV.replace([path])
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 0, e.status
      assert File.directory?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_parents
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "a", "b", "c")
      old_argv = ARGV.dup
      ARGV.replace(["-p", path])
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 0, e.status
      assert File.directory?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_mode
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "modedir")
      old_argv = ARGV.dup
      ARGV.replace(["-m", "700", path])
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 0, e.status
      assert File.directory?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_invalid_mode
    old_argv = ARGV.dup
    ARGV.replace(["-m", "xyz", "somedir"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 1, e.status
    end
    assert_includes err, "invalid mode"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 0, e.status
    end
    assert_includes out, "mkdir"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { mkdir_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end
end
