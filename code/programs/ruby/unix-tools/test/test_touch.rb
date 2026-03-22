# frozen_string_literal: true

# test_touch.rb -- Tests for the Ruby touch tool
# ================================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "tmpdir"
require "coding_adventures_cli_builder"

require_relative "../touch_tool"

module TouchTestHelper
  TOUCH_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "touch.json")

  def parse_touch_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TOUCH_TEST_SPEC, ["touch"] + argv).parse
  end
end

class TestTouchCliIntegration < Minitest::Test
  include TouchTestHelper

  def test_basic_parse
    result = parse_touch_argv(["file.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_touch_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_touch_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_no_create_flag
    result = parse_touch_argv(["-c", "file.txt"])
    assert result.flags["no_create"]
  end

  def test_access_only_flag
    result = parse_touch_argv(["-a", "file.txt"])
    assert result.flags["access_only"]
  end

  def test_modify_only_flag
    result = parse_touch_argv(["-m", "file.txt"])
    assert result.flags["modify_only"]
  end
end

class TestTouchParseTimestamp < Minitest::Test
  def test_mmddhhmm
    result = touch_parse_timestamp("01151030")
    refute_nil result
  end

  def test_ccyymmddhhmm
    result = touch_parse_timestamp("202401151030")
    refute_nil result
  end

  def test_invalid
    result = touch_parse_timestamp("abc")
    assert_nil result
  end
end

class TestTouchTouchFile < Minitest::Test
  def test_create_new_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "newfile.txt")
      assert touch_touch_file(path, no_create: false, access_only: false,
                              modify_only: false, timestamp: nil)
      assert File.exist?(path)
    end
  end

  def test_no_create_skips
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nonexistent.txt")
      assert touch_touch_file(path, no_create: true, access_only: false,
                              modify_only: false, timestamp: nil)
      refute File.exist?(path)
    end
  end

  def test_update_existing_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "existing.txt")
      File.write(path, "content")
      old_mtime = File.stat(path).mtime
      sleep 0.05
      touch_touch_file(path, no_create: false, access_only: false,
                       modify_only: false, timestamp: nil)
      new_mtime = File.stat(path).mtime
      assert new_mtime >= old_mtime
    end
  end

  def test_specific_timestamp
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "timestamped.txt")
      File.write(path, "data")
      target_time = Time.at(1_000_000_000)
      touch_touch_file(path, no_create: false, access_only: false,
                       modify_only: false, timestamp: target_time)
      stat = File.stat(path)
      assert_in_delta target_time.to_f, stat.mtime.to_f, 1.0
    end
  end
end
