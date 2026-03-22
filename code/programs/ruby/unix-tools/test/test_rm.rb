# frozen_string_literal: true

# test_rm.rb -- Tests for the Ruby rm tool
# ==========================================

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

require_relative "../rm_tool"

module RmTestHelper
  RM_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "rm.json")

  def parse_rm_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(RM_TEST_SPEC, ["rm"] + argv).parse
  end
end

class TestRmCliIntegration < Minitest::Test
  include RmTestHelper

  def test_basic_parse
    result = parse_rm_argv(["file.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_rm_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_rm_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_force_flag
    result = parse_rm_argv(["-f", "file.txt"])
    assert result.flags["force"]
  end

  def test_recursive_flag
    result = parse_rm_argv(["-r", "dir"])
    assert result.flags["recursive"]
  end
end

class TestRmRemoveFile < Minitest::Test
  def test_remove_regular_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      assert rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_remove_nonexistent_fails
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      refute rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
    end
  end

  def test_remove_nonexistent_force_ok
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      assert rm_remove_file(path, force: true, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
    end
  end

  def test_remove_directory_fails_without_r
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      refute rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
    end
  end

  def test_remove_directory_recursive
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      File.write(File.join(path, "file.txt"), "data")
      assert rm_remove_file(path, force: false, interactive: false, recursive: true,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_remove_empty_dir_with_d_flag
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "emptydir")
      Dir.mkdir(path)
      assert rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: true, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_verbose_output
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "verbose.txt")
      File.write(path, "data")
      output = capture_io do
        rm_remove_file(path, force: false, interactive: false, recursive: false,
                       verbose: true, dir_flag: false, preserve_root: true)
      end[0]
      assert_includes output, "removed"
    end
  end

  def test_preserve_root
    refute rm_remove_file("/", force: false, interactive: false, recursive: true,
                          verbose: false, dir_flag: false, preserve_root: true)
  end

  def test_remove_symlink
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      assert rm_remove_file(link, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.symlink?(link)
      assert File.exist?(target) # Target should still exist.
    end
  end
end
