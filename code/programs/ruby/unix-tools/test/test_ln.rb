# frozen_string_literal: true

# test_ln.rb -- Tests for the Ruby ln tool
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

require_relative "../ln_tool"

module LnTestHelper
  LN_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "ln.json")

  def parse_ln_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(LN_TEST_SPEC, ["ln"] + argv).parse
  end
end

class TestLnCliIntegration < Minitest::Test
  include LnTestHelper

  def test_basic_parse
    result = parse_ln_argv(["target", "link"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_ln_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_ln_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_symbolic_flag
    result = parse_ln_argv(["-s", "target", "link"])
    assert result.flags["symbolic"]
  end

  def test_force_flag
    result = parse_ln_argv(["-f", "target", "link"])
    assert result.flags["force"]
  end

  def test_verbose_flag
    result = parse_ln_argv(["-v", "target", "link"])
    assert result.flags["verbose"]
  end
end

class TestLnMakeLink < Minitest::Test
  def test_create_hard_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "hardlink.txt")
      assert ln_make_link(target, link, symbolic: false, force: false,
                          verbose: false, relative: false, no_dereference: false)
      assert File.exist?(link)
      assert_equal File.stat(target).ino, File.stat(link).ino
    end
  end

  def test_create_symbolic_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "symlink.txt")
      assert ln_make_link(target, link, symbolic: true, force: false,
                          verbose: false, relative: false, no_dereference: false)
      assert File.symlink?(link)
      assert_equal target, File.readlink(link)
    end
  end

  def test_create_link_in_directory
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      dest_dir = File.join(tmp, "destdir")
      Dir.mkdir(dest_dir)
      assert ln_make_link(target, dest_dir, symbolic: true, force: false,
                          verbose: false, relative: false, no_dereference: false)
      link_path = File.join(dest_dir, "target.txt")
      assert File.symlink?(link_path)
    end
  end

  def test_no_dereference_prevents_dir_expansion
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      dest_dir = File.join(tmp, "destdir")
      Dir.mkdir(dest_dir)
      # With no_dereference, link_name stays as the dir path itself
      # This will fail with EEXIST since destdir already exists
      _out, err = capture_io do
        refute ln_make_link(target, dest_dir, symbolic: true, force: false,
                            verbose: false, relative: false, no_dereference: true)
      end
      assert_includes err, "File exists"
    end
  end

  def test_force_removes_existing
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      File.write(link, "old")
      assert ln_make_link(target, link, symbolic: true, force: true,
                          verbose: false, relative: false, no_dereference: false)
      assert File.symlink?(link)
    end
  end

  def test_existing_file_without_force_fails
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      File.write(link, "old")
      _out, err = capture_io do
        refute ln_make_link(target, link, symbolic: true, force: false,
                            verbose: false, relative: false, no_dereference: false)
      end
      assert_includes err, "File exists"
    end
  end

  def test_hard_link_eexist_error
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      File.write(link, "existing")
      _out, err = capture_io do
        refute ln_make_link(target, link, symbolic: false, force: false,
                            verbose: false, relative: false, no_dereference: false)
      end
      assert_includes err, "hard link"
      assert_includes err, "File exists"
    end
  end

  def test_enoent_error
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "nonexistent.txt")
      link = File.join(tmp, "noparent", "link.txt")
      _out, err = capture_io do
        refute ln_make_link(target, link, symbolic: false, force: false,
                            verbose: false, relative: false, no_dereference: false)
      end
      assert_includes err, "No such file or directory"
    end
  end

  def test_verbose_hard_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      out, _err = capture_io do
        ln_make_link(target, link, symbolic: false, force: false,
                     verbose: true, relative: false, no_dereference: false)
      end
      assert_includes out, "=>"
    end
  end

  def test_verbose_symbolic_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      out, _err = capture_io do
        ln_make_link(target, link, symbolic: true, force: false,
                     verbose: true, relative: false, no_dereference: false)
      end
      assert_includes out, " -> "
    end
  end

  def test_relative_symbolic_link
    Dir.mktmpdir do |tmp|
      subdir = File.join(tmp, "sub")
      Dir.mkdir(subdir)
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(subdir, "link.txt")
      assert ln_make_link(target, link, symbolic: true, force: false,
                          verbose: false, relative: true, no_dereference: false)
      assert File.symlink?(link)
      readlink = File.readlink(link)
      assert_includes readlink, ".."
    end
  end

  def test_force_remove_symlink
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      # Force replace existing symlink
      assert ln_make_link(target, link, symbolic: true, force: true,
                          verbose: false, relative: false, no_dereference: false)
      assert File.symlink?(link)
    end
  end
end

class TestLnMainIntegration < Minitest::Test
  def test_main_single_target
    Dir.mktmpdir do |tmp|
      # Target is in a different directory so basename won't conflict
      subdir = File.join(tmp, "sub")
      Dir.mkdir(subdir)
      target = File.join(subdir, "target.txt")
      File.write(target, "content")
      old_argv = ARGV.dup
      old_dir = Dir.pwd
      Dir.chdir(tmp)
      ARGV.replace(["-s", target])
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
      assert File.symlink?(File.join(tmp, "target.txt"))
    ensure
      Dir.chdir(old_dir)
      ARGV.replace(old_argv)
    end
  end

  def test_main_two_targets
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      old_argv = ARGV.dup
      ARGV.replace(["-s", target, link])
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
      assert File.symlink?(link)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_multiple_targets_into_directory
    Dir.mktmpdir do |tmp|
      t1 = File.join(tmp, "t1.txt")
      t2 = File.join(tmp, "t2.txt")
      File.write(t1, "one")
      File.write(t2, "two")
      dest = File.join(tmp, "dest")
      Dir.mkdir(dest)
      old_argv = ARGV.dup
      ARGV.replace(["-s", t1, t2, dest])
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
      assert File.symlink?(File.join(dest, "t1.txt"))
      assert File.symlink?(File.join(dest, "t2.txt"))
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_target_not_directory_error
    Dir.mktmpdir do |tmp|
      t1 = File.join(tmp, "t1.txt")
      t2 = File.join(tmp, "t2.txt")
      t3 = File.join(tmp, "t3.txt")
      File.write(t1, "one")
      File.write(t2, "two")
      File.write(t3, "three")
      old_argv = ARGV.dup
      ARGV.replace(["-s", t1, t2, t3])
      _out, err = capture_io do
        e = assert_raises(SystemExit) { ln_main }
        assert_equal 1, e.status
      end
      assert_includes err, "is not a directory"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_no_target_dir_flag
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "content")
      link = File.join(tmp, "link.txt")
      old_argv = ARGV.dup
      ARGV.replace(["-s", "-T", target, link])
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
      assert File.symlink?(link)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
    end
    assert_includes out, "ln"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_missing_operand
    old_argv = ARGV.dup
    ARGV.replace(["-s"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { ln_main }
      assert_equal 1, e.status
    end
    # CLI builder reports the error with its own format
    assert_includes err, "ln:"
  ensure
    ARGV.replace(old_argv)
  end
end
