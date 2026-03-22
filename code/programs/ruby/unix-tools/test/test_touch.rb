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

  def test_access_only
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "aonly.txt")
      File.write(path, "data")
      old_mtime = File.stat(path).mtime
      sleep 0.05
      target_time = Time.at(1_500_000_000)
      touch_touch_file(path, no_create: false, access_only: true,
                       modify_only: false, timestamp: target_time)
      stat = File.stat(path)
      assert_in_delta target_time.to_f, stat.atime.to_f, 1.0
      assert_in_delta old_mtime.to_f, stat.mtime.to_f, 1.0
    end
  end

  def test_modify_only
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "monly.txt")
      File.write(path, "data")
      old_atime = File.stat(path).atime
      target_time = Time.at(1_500_000_000)
      touch_touch_file(path, no_create: false, access_only: false,
                       modify_only: true, timestamp: target_time)
      stat = File.stat(path)
      assert_in_delta target_time.to_f, stat.mtime.to_f, 1.0
    end
  end

  def test_create_in_nonexistent_dir
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nodir", "file.txt")
      _out, err = capture_io do
        refute touch_touch_file(path, no_create: false, access_only: false,
                                modify_only: false, timestamp: nil)
      end
      assert_includes err, "No such file or directory"
    end
  end
end

class TestTouchParseDate < Minitest::Test
  def test_valid_date
    result = touch_parse_date("2024-01-15 10:30:00")
    refute_nil result
  end

  def test_invalid_date
    result = touch_parse_date("not-a-date-at-all")
    assert_nil result
  end
end

class TestTouchParseTimestampEdgeCases < Minitest::Test
  def test_yymmddhhmm
    result = touch_parse_timestamp("2401151030")
    refute_nil result
    assert_equal 2024, result.year
  end

  def test_yymmddhhmm_old_year
    result = touch_parse_timestamp("7001151030")
    refute_nil result
    assert_equal 1970, result.year
  end

  def test_with_seconds
    result = touch_parse_timestamp("01151030.30")
    refute_nil result
    assert_equal 30, result.sec
  end

  def test_unsupported_length
    result = touch_parse_timestamp("12345")
    assert_nil result
  end
end

class TestTouchMainIntegration < Minitest::Test
  def test_main_creates_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "maintest.txt")
      old_argv = ARGV.dup
      ARGV.replace([path])
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
      assert File.exist?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_no_create
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "nocreate.txt")
      old_argv = ARGV.dup
      ARGV.replace(["-c", path])
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
      refute File.exist?(path)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_reference
    Dir.mktmpdir do |tmp|
      ref = File.join(tmp, "ref.txt")
      File.write(ref, "ref")
      target = File.join(tmp, "target.txt")
      File.write(target, "target")
      old_argv = ARGV.dup
      ARGV.replace(["-r", ref, target])
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_reference_nonexistent
    Dir.mktmpdir do |tmp|
      old_argv = ARGV.dup
      ARGV.replace(["-r", "/nonexistent/ref", "file.txt"])
      _out, err = capture_io do
        e = assert_raises(SystemExit) { touch_main }
        assert_equal 1, e.status
      end
      assert_includes err, "failed to get attributes"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_timestamp
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "ts.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace(["-t", "202401151030", path])
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_invalid_timestamp
    old_argv = ARGV.dup
    ARGV.replace(["-t", "invalid", "file.txt"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 1, e.status
    end
    assert_includes err, "invalid date format"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_with_date
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "dated.txt")
      File.write(path, "data")
      old_argv = ARGV.dup
      ARGV.replace(["-d", "2024-01-15 10:30:00", path])
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_invalid_date
    old_argv = ARGV.dup
    ARGV.replace(["-d", "not-a-date-at-all", "file.txt"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 1, e.status
    end
    assert_includes err, "invalid date format"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
    end
    assert_includes out, "touch"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { touch_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end
end
