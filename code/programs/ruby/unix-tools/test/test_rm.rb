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

require "stringio"

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

  def test_verbose_flag
    result = parse_rm_argv(["-v", "file.txt"])
    assert result.flags["verbose"]
  end

  def test_dir_flag
    result = parse_rm_argv(["-d", "dir"])
    assert result.flags["dir"]
  end
end

class TestRmConfirm < Minitest::Test
  def test_confirm_yes
    old_stdin = $stdin
    $stdin = StringIO.new("y\n")
    old_stderr = $stderr
    $stderr = StringIO.new
    assert rm_confirm("remove? ")
  ensure
    $stdin = old_stdin
    $stderr = old_stderr
  end

  def test_confirm_yes_full
    old_stdin = $stdin
    $stdin = StringIO.new("yes\n")
    old_stderr = $stderr
    $stderr = StringIO.new
    assert rm_confirm("remove? ")
  ensure
    $stdin = old_stdin
    $stderr = old_stderr
  end

  def test_confirm_no
    old_stdin = $stdin
    $stdin = StringIO.new("n\n")
    old_stderr = $stderr
    $stderr = StringIO.new
    refute rm_confirm("remove? ")
  ensure
    $stdin = old_stdin
    $stderr = old_stderr
  end

  def test_confirm_eof
    old_stdin = $stdin
    $stdin = StringIO.new("")
    old_stderr = $stderr
    $stderr = StringIO.new
    refute rm_confirm("remove? ")
  ensure
    $stdin = old_stdin
    $stderr = old_stderr
  end
end

class TestRmRemoveFile < Minitest::Test
  def test_remove_regular_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "content")
      assert rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_remove_nonexistent_without_force
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      _out, err = capture_io do
        refute rm_remove_file(path, force: false, interactive: false, recursive: false,
                              verbose: false, dir_flag: false, preserve_root: true)
      end
      assert_includes err, "No such file or directory"
    end
  end

  def test_remove_nonexistent_with_force_succeeds
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      # With force=true, nonexistent files return true (silently succeed)
      assert rm_remove_file(path, force: true, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
    end
  end

  def test_remove_nonexistent_with_force_no_warning
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent")
      _out, err = capture_io do
        rm_remove_file(path, force: true, interactive: false, recursive: false,
                       verbose: false, dir_flag: false, preserve_root: true)
      end
      refute_includes err, "No such file or directory"
    end
  end

  def test_remove_directory_without_recursive_or_dir
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      _out, err = capture_io do
        refute rm_remove_file(path, force: false, interactive: false, recursive: false,
                              verbose: false, dir_flag: false, preserve_root: true)
      end
      assert_includes err, "Is a directory"
    end
  end

  def test_remove_directory_recursive
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      File.write(File.join(path, "child.txt"), "data")
      assert rm_remove_file(path, force: false, interactive: false, recursive: true,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_remove_directory_recursive_verbose
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      out, _err = capture_io do
        rm_remove_file(path, force: false, interactive: false, recursive: true,
                       verbose: true, dir_flag: false, preserve_root: true)
      end
      assert_includes out, "removed directory"
    end
  end

  def test_remove_empty_directory_with_dir_flag
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "emptydir")
      Dir.mkdir(path)
      assert rm_remove_file(path, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: true, preserve_root: true)
      refute File.exist?(path)
    end
  end

  def test_remove_nonempty_directory_with_dir_flag
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonemptydir")
      Dir.mkdir(path)
      File.write(File.join(path, "child.txt"), "data")
      _out, err = capture_io do
        refute rm_remove_file(path, force: false, interactive: false, recursive: false,
                              verbose: false, dir_flag: true, preserve_root: true)
      end
      assert_includes err, "Directory not empty"
    end
  end

  def test_remove_dir_flag_verbose
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "emptydir2")
      Dir.mkdir(path)
      out, _err = capture_io do
        rm_remove_file(path, force: false, interactive: false, recursive: false,
                       verbose: true, dir_flag: true, preserve_root: true)
      end
      assert_includes out, "removed directory"
    end
  end

  def test_remove_file_verbose
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      out, _err = capture_io do
        rm_remove_file(path, force: false, interactive: false, recursive: false,
                       verbose: true, dir_flag: false, preserve_root: true)
      end
      assert_includes out, "removed '#{path}'"
    end
  end

  def test_preserve_root
    _out, err = capture_io do
      refute rm_remove_file("/", force: false, interactive: false, recursive: true,
                            verbose: false, dir_flag: false, preserve_root: true)
    end
    assert_includes err, "dangerous to operate"
  end

  def test_remove_symlink
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "data")
      link = File.join(tmp, "link.txt")
      File.symlink(target, link)
      assert rm_remove_file(link, force: false, interactive: false, recursive: false,
                            verbose: false, dir_flag: false, preserve_root: true)
      refute File.symlink?(link)
      assert File.exist?(target) # target still exists
    end
  end

  def test_interactive_confirm_yes
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_stdin = $stdin
      $stdin = StringIO.new("y\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      result = rm_remove_file(path, force: false, interactive: true, recursive: false,
                              verbose: false, dir_flag: false, preserve_root: true)
      assert result
      refute File.exist?(path)
    ensure
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end

  def test_interactive_confirm_no
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_stdin = $stdin
      $stdin = StringIO.new("n\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      result = rm_remove_file(path, force: false, interactive: true, recursive: false,
                              verbose: false, dir_flag: false, preserve_root: true)
      # When user says "no", rm_confirm returns false, but the unless block
      # means the file ISN'T removed and we return true (skip).
      assert result
      assert File.exist?(path)
    ensure
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end

  def test_interactive_recursive_directory
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      old_stdin = $stdin
      $stdin = StringIO.new("y\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      result = rm_remove_file(path, force: false, interactive: true, recursive: true,
                              verbose: false, dir_flag: false, preserve_root: true)
      assert result
      refute File.exist?(path)
    ensure
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end

  def test_interactive_recursive_directory_decline
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "mydir")
      Dir.mkdir(path)
      old_stdin = $stdin
      $stdin = StringIO.new("n\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      result = rm_remove_file(path, force: false, interactive: true, recursive: true,
                              verbose: false, dir_flag: false, preserve_root: true)
      assert result # returns true when declined
      assert File.exist?(path) # dir still exists
    ensure
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end
end

class TestRmMainIntegration < Minitest::Test
  def test_main_removes_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace([path])
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
      refute File.exist?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_recursive
    Dir.mktmpdir do |tmp|
      dir = File.join(tmp, "mydir")
      Dir.mkdir(dir)
      File.write(File.join(dir, "child.txt"), "data")
      old_argv = ARGV.dup
      ARGV.replace(["-r", dir])
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
      refute File.exist?(dir)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_force_nonexistent
    old_argv = ARGV.dup
    ARGV.replace(["-f", "/nonexistent/file"])
    e = assert_raises(SystemExit) { rm_main }
    assert_equal 0, e.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
    end
    assert_includes out, "rm"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_interactive_once_bulk
    Dir.mktmpdir do |tmp|
      files = (1..4).map do |i|
        p = File.join(tmp, "file#{i}.txt")
        File.write(p, "data")
        p
      end
      old_argv = ARGV.dup
      old_stdin = $stdin
      $stdin = StringIO.new("y\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      ARGV.replace(["-I"] + files)
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
    ensure
      ARGV.replace(old_argv)
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end

  def test_main_interactive_once_decline
    Dir.mktmpdir do |tmp|
      files = (1..4).map do |i|
        p = File.join(tmp, "file#{i}.txt")
        File.write(p, "data")
        p
      end
      old_argv = ARGV.dup
      old_stdin = $stdin
      $stdin = StringIO.new("n\n")
      old_stderr = $stderr
      $stderr = StringIO.new
      ARGV.replace(["-I"] + files)
      e = assert_raises(SystemExit) { rm_main }
      assert_equal 0, e.status
      # Files should still exist since we declined
      files.each { |f| assert File.exist?(f) }
    ensure
      ARGV.replace(old_argv)
      $stdin = old_stdin
      $stderr = old_stderr
    end
  end

  def test_main_verbose
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "file.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace(["-v", path])
      out, _err = capture_io do
        e = assert_raises(SystemExit) { rm_main }
        assert_equal 0, e.status
      end
      assert_includes out, "removed"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_dir_fails_on_directory
    Dir.mktmpdir do |tmp|
      dir = File.join(tmp, "mydir")
      Dir.mkdir(dir)
      old_argv = ARGV.dup
      ARGV.replace([dir])
      _out, err = capture_io do
        e = assert_raises(SystemExit) { rm_main }
        assert_equal 1, e.status
      end
      assert_includes err, "Is a directory"
    ensure
      ARGV.replace(old_argv)
    end
  end
end
