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

  def test_invalid_mode
    assert_nil mkdir_parse_mode("xyz")
  end
end
