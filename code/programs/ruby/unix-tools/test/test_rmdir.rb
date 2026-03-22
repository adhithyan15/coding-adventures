# frozen_string_literal: true

# test_rmdir.rb -- Tests for the Ruby rmdir tool
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

require_relative "../rmdir_tool"

module RmdirTestHelper
  RMDIR_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "rmdir.json")

  def parse_rmdir_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(RMDIR_TEST_SPEC, ["rmdir"] + argv).parse
  end
end

class TestRmdirCliIntegration < Minitest::Test
  include RmdirTestHelper

  def test_basic_parse
    result = parse_rmdir_argv(["testdir"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_rmdir_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_rmdir_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_parents_flag
    result = parse_rmdir_argv(["-p", "a/b/c"])
    assert result.flags["parents"]
  end

  def test_ignore_non_empty_flag
    result = parse_rmdir_argv(["--ignore-fail-on-non-empty", "testdir"])
    assert result.flags["ignore_fail_on_non_empty"]
  end
end

class TestRmdirRemoveDirectory < Minitest::Test
  def test_remove_empty_directory
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "empty")
      Dir.mkdir(path)
      assert rmdir_remove_directory(path, verbose: false, ignore_non_empty: false)
      refute File.exist?(path)
    end
  end

  def test_remove_nonexistent_fails
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      refute rmdir_remove_directory(path, verbose: false, ignore_non_empty: false)
    end
  end

  def test_remove_nonempty_fails
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonempty")
      Dir.mkdir(path)
      File.write(File.join(path, "file.txt"), "content")
      refute rmdir_remove_directory(path, verbose: false, ignore_non_empty: false)
    end
  end

  def test_verbose_output
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "verbose")
      Dir.mkdir(path)
      output = capture_io { rmdir_remove_directory(path, verbose: true, ignore_non_empty: false) }[0]
      assert_includes output, "removing directory"
    end
  end
end

class TestRmdirRemoveWithParents < Minitest::Test
  def test_remove_chain
    Dir.mktmpdir do |tmp|
      chain = File.join(tmp, "a", "b", "c")
      FileUtils.mkdir_p(chain)
      assert rmdir_remove_with_parents(chain, verbose: false, ignore_non_empty: false)
      refute File.exist?(File.join(tmp, "a"))
    end
  end

  def test_remove_chain_stops_at_nonempty
    Dir.mktmpdir do |tmp|
      chain = File.join(tmp, "a", "b")
      FileUtils.mkdir_p(chain)
      File.write(File.join(tmp, "a", "sibling.txt"), "data")
      refute rmdir_remove_with_parents(chain, verbose: false, ignore_non_empty: false)
      refute File.exist?(File.join(tmp, "a", "b"))
      assert File.exist?(File.join(tmp, "a"))
    end
  end
end
