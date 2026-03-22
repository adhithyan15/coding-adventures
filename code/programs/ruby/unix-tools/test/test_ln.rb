# frozen_string_literal: true

# test_ln.rb -- Tests for the Ruby ln tool
# ==========================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
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
end

class TestLnMakeLink < Minitest::Test
  def test_hard_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "hardlink.txt")
      assert ln_make_link(target, link_name, symbolic: false, force: false,
                          verbose: false, relative: false, no_dereference: true)
      assert File.exist?(link_name)
      assert_equal File.stat(target).ino, File.stat(link_name).ino
    end
  end

  def test_symbolic_link
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "symlink.txt")
      assert ln_make_link(target, link_name, symbolic: true, force: false,
                          verbose: false, relative: false, no_dereference: true)
      assert File.symlink?(link_name)
    end
  end

  def test_force_overwrites
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "existing.txt")
      File.write(link_name, "old")
      assert ln_make_link(target, link_name, symbolic: true, force: true,
                          verbose: false, relative: false, no_dereference: true)
      assert File.symlink?(link_name)
    end
  end

  def test_fails_if_exists
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "existing.txt")
      File.write(link_name, "old")
      refute ln_make_link(target, link_name, symbolic: true, force: false,
                          verbose: false, relative: false, no_dereference: true)
    end
  end

  def test_verbose_output
    Dir.mktmpdir do |tmp|
      target = File.join(tmp, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "verboselink.txt")
      output = capture_io do
        ln_make_link(target, link_name, symbolic: true, force: false,
                     verbose: true, relative: false, no_dereference: true)
      end[0]
      assert_includes output, "->"
    end
  end

  def test_relative_symlink
    Dir.mktmpdir do |tmp|
      subdir = File.join(tmp, "subdir")
      Dir.mkdir(subdir)
      target = File.join(subdir, "target.txt")
      File.write(target, "hello")
      link_name = File.join(tmp, "rellink.txt")
      assert ln_make_link(target, link_name, symbolic: true, force: false,
                          verbose: false, relative: true, no_dereference: true)
      assert File.symlink?(link_name)
      link_target = File.readlink(link_name)
      refute link_target.start_with?("/")
    end
  end
end
