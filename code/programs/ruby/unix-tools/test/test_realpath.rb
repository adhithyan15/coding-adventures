# frozen_string_literal: true

# test_realpath.rb -- Tests for the Ruby realpath tool
# =====================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "coding_adventures_cli_builder"

require_relative "../realpath_tool"

module RealpathTestHelper
  REALPATH_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "realpath.json")

  def parse_realpath_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(REALPATH_TEST_SPEC, ["realpath"] + argv).parse
  end
end

class TestRealpathCliIntegration < Minitest::Test
  include RealpathTestHelper

  def test_basic_parse
    result = parse_realpath_argv(["/tmp"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_realpath_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_realpath_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_canonicalize_existing_flag
    result = parse_realpath_argv(["-e", "/tmp"])
    assert result.flags["canonicalize_existing"]
  end

  def test_no_symlinks_flag
    result = parse_realpath_argv(["-s", "/tmp"])
    assert result.flags["no_symlinks"]
  end
end

class TestRealpathResolve < Minitest::Test
  def test_resolve_existing_path
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      resolved = realpath_resolve(path, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: false)
      refute_nil resolved
      assert File.absolute_path?(resolved) || resolved.start_with?("/")
    end
  end

  def test_resolve_symlink
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "data")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      resolved = realpath_resolve(link, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: false)
      assert_equal File.realpath(target), resolved
    end
  end

  def test_no_symlinks_mode
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "data")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      resolved = realpath_resolve(link, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: true)
      assert_equal File.expand_path(link), resolved
    end
  end

  def test_canonicalize_existing_fails
    resolved = realpath_resolve("/this/path/does/not/exist",
                                canonicalize_existing: true,
                                canonicalize_missing: false, no_symlinks: false)
    assert_nil resolved
  end

  def test_canonicalize_missing_succeeds
    resolved = realpath_resolve("/this/path/does/not/exist",
                                canonicalize_existing: false,
                                canonicalize_missing: true, no_symlinks: false)
    refute_nil resolved
  end
end

class TestRealpathMakeRelative < Minitest::Test
  def test_relative_to
    result = realpath_make_relative("/a/b/c", relative_to: "/a", relative_base: nil)
    assert_equal "b/c", result
  end

  def test_relative_base_under
    Dir.mktmpdir do |tmp|
      resolved = "#{tmp}/sub/file.txt"
      result = realpath_make_relative(resolved, relative_to: nil, relative_base: tmp)
      refute result.start_with?("/")
    end
  end

  def test_relative_base_outside
    result = realpath_make_relative("/completely/different/path",
                                   relative_to: nil, relative_base: "/some/base")
    assert result.start_with?("/")
  end
end
