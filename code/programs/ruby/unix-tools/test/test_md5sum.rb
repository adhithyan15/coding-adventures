# frozen_string_literal: true

# test_md5sum.rb -- Tests for the Ruby md5sum tool
# ===================================================
#
# === What These Tests Verify ===
#
# These tests exercise the md5sum tool's digest computation, output
# formatting, and check mode. We test:
# - compute_md5 produces correct hex digests
# - format_checksum_line for text and binary modes
# - check_checksums verifies files against stored hashes
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tempfile"
require "stringio"
require "digest"
require "coding_adventures_cli_builder"

require_relative "../md5sum_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module Md5sumTestHelper
  MD5SUM_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "md5sum.json")

  def parse_md5sum_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(MD5SUM_TEST_SPEC, ["md5sum"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("md5sum_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestMd5sumCliIntegration < Minitest::Test
  include Md5sumTestHelper

  def test_no_flags_returns_parse_result
    result = parse_md5sum_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_md5sum_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "md5sum"
  end

  def test_version_returns_version_result
    result = parse_md5sum_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_check_flag
    result = parse_md5sum_argv(["-c", "/dev/null"])
    assert result.flags["check"]
  end

  def test_binary_flag
    result = parse_md5sum_argv(["-b"])
    assert result.flags["binary"]
  end
end

# ===========================================================================
# Test: compute_md5
# ===========================================================================

class TestComputeMd5 < Minitest::Test
  def test_empty_string
    io = StringIO.new("")
    hash = compute_md5(io)
    # MD5 of empty string is well-known
    assert_equal "d41d8cd98f00b204e9800998ecf8427e", hash
  end

  def test_hello_world
    io = StringIO.new("hello world")
    hash = compute_md5(io)
    expected = Digest::MD5.hexdigest("hello world")
    assert_equal expected, hash
  end

  def test_returns_32_char_hex_string
    io = StringIO.new("test data")
    hash = compute_md5(io)
    assert_equal 32, hash.length
    assert_match(/\A[0-9a-f]{32}\z/, hash)
  end

  def test_different_inputs_different_hashes
    hash1 = compute_md5(StringIO.new("abc"))
    hash2 = compute_md5(StringIO.new("def"))
    refute_equal hash1, hash2
  end

  def test_same_input_same_hash
    hash1 = compute_md5(StringIO.new("same"))
    hash2 = compute_md5(StringIO.new("same"))
    assert_equal hash1, hash2
  end

  def test_file_hash
    content = "file content for md5 test"
    expected = Digest::MD5.hexdigest(content)

    f = Tempfile.new("md5_file_test")
    f.write(content)
    f.rewind
    hash = compute_md5(f)
    f.close
    f.unlink

    assert_equal expected, hash
  end
end

# ===========================================================================
# Test: format_checksum_line
# ===========================================================================

class TestFormatChecksumLine < Minitest::Test
  def test_text_mode
    result = format_checksum_line("abc123", "file.txt", false)
    assert_equal "abc123  file.txt", result
  end

  def test_binary_mode
    result = format_checksum_line("abc123", "file.txt", true)
    assert_equal "abc123 *file.txt", result
  end

  def test_preserves_hash
    hash = "d41d8cd98f00b204e9800998ecf8427e"
    result = format_checksum_line(hash, "empty", false)
    assert_includes result, hash
  end
end

# ===========================================================================
# Test: check_checksums
# ===========================================================================

class TestCheckChecksums < Minitest::Test
  include Md5sumTestHelper

  def test_valid_checksum_passes
    with_tempfile("hello world") do |path|
      expected_hash = Digest::MD5.hexdigest("hello world")
      checksum_io = StringIO.new("#{expected_hash}  #{path}\n")
      output, _err = capture_io do
        assert check_checksums(checksum_io, {})
      end
      assert_includes output, "OK"
    end
  end

  def test_invalid_checksum_fails
    with_tempfile("hello world") do |path|
      wrong_hash = "0" * 32
      checksum_io = StringIO.new("#{wrong_hash}  #{path}\n")
      output, err = capture_io do
        refute check_checksums(checksum_io, {})
      end
      assert_includes output, "FAILED"
      assert_includes err, "did NOT match"
    end
  end

  def test_quiet_mode_suppresses_ok
    with_tempfile("hello") do |path|
      expected_hash = Digest::MD5.hexdigest("hello")
      checksum_io = StringIO.new("#{expected_hash}  #{path}\n")
      output, _err = capture_io do
        assert check_checksums(checksum_io, {"quiet" => true})
      end
      refute_includes output, "OK"
    end
  end

  def test_status_mode_no_output
    with_tempfile("hello") do |path|
      wrong_hash = "0" * 32
      checksum_io = StringIO.new("#{wrong_hash}  #{path}\n")
      output, _err = capture_io do
        refute check_checksums(checksum_io, {"status" => true})
      end
      assert_empty output
    end
  end

  def test_missing_file
    checksum_io = StringIO.new("#{"a" * 32}  /nonexistent/file/xyz\n")
    output, _err = capture_io do
      refute check_checksums(checksum_io, {})
    end
    assert_includes output, "FAILED"
  end

  def test_malformed_line_with_warn
    checksum_io = StringIO.new("not a valid line\n")
    _out, err = capture_io do
      check_checksums(checksum_io, {"warn" => true})
    end
    assert_includes err, "improperly formatted"
  end

  def test_strict_mode_rejects_malformed
    checksum_io = StringIO.new("not a valid line\n")
    capture_io do
      refute check_checksums(checksum_io, {"strict" => true})
    end
  end

  def test_binary_mode_separator
    with_tempfile("test") do |path|
      expected_hash = Digest::MD5.hexdigest("test")
      checksum_io = StringIO.new("#{expected_hash} *#{path}\n")
      capture_io do
        assert check_checksums(checksum_io, {})
      end
    end
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestMd5sumMainFunction < Minitest::Test
  include Md5sumTestHelper

  def test_main_compute_hash
    with_tempfile("hello world") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { md5sum_main }[0]
      expected_hash = Digest::MD5.hexdigest("hello world")
      assert_includes output, expected_hash
      assert_includes output, path
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_binary_mode
    with_tempfile("hello") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-b", path])
      output = capture_io { md5sum_main }[0]
      assert_includes output, " *"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_check_mode
    with_tempfile("test content") do |data_path|
      expected = Digest::MD5.hexdigest("test content")
      checksum_content = "#{expected}  #{data_path}\n"

      with_tempfile(checksum_content) do |checksum_path|
        old_argv = ARGV.dup
        ARGV.replace(["-c", checksum_path])
        output = nil
        err = assert_raises(SystemExit) do
          output = capture_io { md5sum_main }[0]
        end
        assert_equal 0, err.status
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz_md5"])
    _out, err = capture_io { md5sum_main }
    assert_includes err, "No such file"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { md5sum_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { md5sum_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
