# frozen_string_literal: true

# test_realpath.rb -- Tests for the Ruby realpath tool
# =====================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
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
      real_tmp = File.realpath(tmp)
      resolved = "#{real_tmp}/sub/file.txt"
      result = realpath_make_relative(resolved, relative_to: nil, relative_base: real_tmp)
      refute result.start_with?("/")
    end
  end

  def test_relative_base_outside
    result = realpath_make_relative("/completely/different/path",
                                   relative_to: nil, relative_base: "/some/base")
    assert result.start_with?("/")
  end

  def test_no_relative
    result = realpath_make_relative("/a/b/c", relative_to: nil, relative_base: nil)
    assert_equal "/a/b/c", result
  end

  def test_relative_base_exact_match
    result = realpath_make_relative("/some/base", relative_to: nil, relative_base: "/some/base")
    assert_equal ".", result
  end
end

class TestRealpathResolveEdgeCases < Minitest::Test
  def test_default_mode_nonexistent_leaf
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent_file.txt")
      resolved = realpath_resolve(path, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: false)
      refute_nil resolved
      assert_includes resolved, "nonexistent_file.txt"
    end
  end

  def test_default_mode_nonexistent_parent
    resolved = realpath_resolve("/nonexistent_parent/nonexistent_child",
                                canonicalize_existing: false,
                                canonicalize_missing: false, no_symlinks: false)
    refute_nil resolved
  end

  def test_canonicalize_missing_with_existing_prefix
    Dir.mktmpdir do |tmp|
      resolved = realpath_resolve("#{tmp}/missing/part",
                                  canonicalize_existing: false,
                                  canonicalize_missing: true, no_symlinks: false)
      refute_nil resolved
      assert_includes resolved, "missing/part"
    end
  end

  def test_canonicalize_missing_all_missing
    resolved = realpath_resolve("/nonexistent/path/here",
                                canonicalize_existing: false,
                                canonicalize_missing: true, no_symlinks: false)
    refute_nil resolved
  end

  def test_resolve_dot_components
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, ".", "file.txt")
      File.write(File.join(tmp, "file.txt"), "data")
      resolved = realpath_resolve(path, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: false)
      refute_includes resolved, "/."
    end
  end

  def test_resolve_symlink_chain
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "real.txt")
      File.write(target, "data")
      link1 = File.join(tmp, "link1.txt")
      File.symlink(target, link1)
      link2 = File.join(tmp, "link2.txt")
      File.symlink(link1, link2)
      resolved = realpath_resolve(link2, canonicalize_existing: false,
                                  canonicalize_missing: false, no_symlinks: false)
      assert_equal File.realpath(target), resolved
    end
  end
end

class TestRealpathMainIntegration < Minitest::Test
  def test_main_resolve_path
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace([path])
      out, _err = capture_io do
        e = assert_raises(SystemExit) { realpath_main }
        assert_equal 0, e.status
      end
      refute out.strip.empty?
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_canonicalize_existing_fail
    old_argv = ARGV.dup
    ARGV.replace(["-e", "/nonexistent/path"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 1, e.status
    end
    assert_includes err, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_quiet_suppresses_error
    old_argv = ARGV.dup
    ARGV.replace(["-e", "-q", "/nonexistent/path"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 1, e.status
    end
    refute_includes err, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_canonicalize_missing
    old_argv = ARGV.dup
    ARGV.replace(["-m", "/nonexistent/path"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 0, e.status
    end
    refute out.strip.empty?
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_no_symlinks
    old_argv = ARGV.dup
    ARGV.replace(["-s", "/tmp"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 0, e.status
    end
    refute out.strip.empty?
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_relative_to
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace(["--relative-to=#{tmp}", path])
      out, _err = capture_io do
        e = assert_raises(SystemExit) { realpath_main }
        assert_equal 0, e.status
      end
      refute out.strip.start_with?("/")
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 0, e.status
    end
    assert_includes out, "realpath"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { realpath_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_zero_terminator
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace(["-z", path])
      out, _err = capture_io do
        e = assert_raises(SystemExit) { realpath_main }
        assert_equal 0, e.status
      end
      assert out.include?("\0")
    ensure
      ARGV.replace(old_argv)
    end
  end
end
