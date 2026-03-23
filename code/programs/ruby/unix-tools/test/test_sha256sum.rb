# frozen_string_literal: true

# test_sha256sum.rb -- Tests for the Ruby sha256sum tool
# ========================================================
#
# === What These Tests Verify ===
#
# These tests exercise the sha256sum tool's digest computation, output
# formatting, and check mode. Structurally similar to md5sum tests but
# with SHA-256 (64-character hex output).

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

require_relative "../sha256sum_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module Sha256sumTestHelper
  SHA256SUM_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "sha256sum.json")

  def parse_sha256sum_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(SHA256SUM_TEST_SPEC, ["sha256sum"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("sha256sum_test")
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

class TestSha256sumCliIntegration < Minitest::Test
  include Sha256sumTestHelper

  def test_no_flags_returns_parse_result
    result = parse_sha256sum_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_sha256sum_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "sha256sum"
  end

  def test_version_returns_version_result
    result = parse_sha256sum_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_check_flag
    result = parse_sha256sum_argv(["-c", "/dev/null"])
    assert result.flags["check"]
  end

  def test_binary_flag
    result = parse_sha256sum_argv(["-b"])
    assert result.flags["binary"]
  end
end

# ===========================================================================
# Test: compute_sha256
# ===========================================================================

class TestComputeSha256 < Minitest::Test
  def test_empty_string
    io = StringIO.new("")
    hash = compute_sha256(io)
    # SHA-256 of empty string is well-known
    expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    assert_equal expected, hash
  end

  def test_hello_world
    io = StringIO.new("hello world")
    hash = compute_sha256(io)
    expected = Digest::SHA256.hexdigest("hello world")
    assert_equal expected, hash
  end

  def test_returns_64_char_hex_string
    io = StringIO.new("test data")
    hash = compute_sha256(io)
    assert_equal 64, hash.length
    assert_match(/\A[0-9a-f]{64}\z/, hash)
  end

  def test_different_inputs_different_hashes
    hash1 = compute_sha256(StringIO.new("abc"))
    hash2 = compute_sha256(StringIO.new("def"))
    refute_equal hash1, hash2
  end

  def test_same_input_same_hash
    hash1 = compute_sha256(StringIO.new("same"))
    hash2 = compute_sha256(StringIO.new("same"))
    assert_equal hash1, hash2
  end

  def test_different_from_md5
    content = "test"
    sha256 = compute_sha256(StringIO.new(content))
    md5 = Digest::MD5.hexdigest(content)
    refute_equal sha256, md5
  end

  def test_file_hash
    content = "file content for sha256 test"
    expected = Digest::SHA256.hexdigest(content)

    f = Tempfile.new("sha256_file_test")
    f.write(content)
    f.rewind
    hash = compute_sha256(f)
    f.close
    f.unlink

    assert_equal expected, hash
  end
end

# ===========================================================================
# Test: format_sha256_line
# ===========================================================================

class TestFormatSha256Line < Minitest::Test
  def test_text_mode
    result = format_sha256_line("abc123", "file.txt", false)
    assert_equal "abc123  file.txt", result
  end

  def test_binary_mode
    result = format_sha256_line("abc123", "file.txt", true)
    assert_equal "abc123 *file.txt", result
  end

  def test_preserves_hash
    hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    result = format_sha256_line(hash, "empty", false)
    assert_includes result, hash
  end
end

# ===========================================================================
# Test: check_sha256_checksums
# ===========================================================================

class TestCheckSha256Checksums < Minitest::Test
  include Sha256sumTestHelper

  def test_valid_checksum_passes
    with_tempfile("hello world") do |path|
      expected_hash = Digest::SHA256.hexdigest("hello world")
      checksum_io = StringIO.new("#{expected_hash}  #{path}\n")
      output, _err = capture_io do
        assert check_sha256_checksums(checksum_io, {})
      end
      assert_includes output, "OK"
    end
  end

  def test_invalid_checksum_fails
    with_tempfile("hello world") do |path|
      wrong_hash = "0" * 64
      checksum_io = StringIO.new("#{wrong_hash}  #{path}\n")
      output, err = capture_io do
        refute check_sha256_checksums(checksum_io, {})
      end
      assert_includes output, "FAILED"
      assert_includes err, "did NOT match"
    end
  end

  def test_quiet_mode_suppresses_ok
    with_tempfile("hello") do |path|
      expected_hash = Digest::SHA256.hexdigest("hello")
      checksum_io = StringIO.new("#{expected_hash}  #{path}\n")
      output, _err = capture_io do
        assert check_sha256_checksums(checksum_io, {"quiet" => true})
      end
      refute_includes output, "OK"
    end
  end

  def test_status_mode_no_output
    with_tempfile("hello") do |path|
      wrong_hash = "0" * 64
      checksum_io = StringIO.new("#{wrong_hash}  #{path}\n")
      output, _err = capture_io do
        refute check_sha256_checksums(checksum_io, {"status" => true})
      end
      assert_empty output
    end
  end

  def test_missing_file
    checksum_io = StringIO.new("#{"a" * 64}  /nonexistent/file/xyz\n")
    output, _err = capture_io do
      refute check_sha256_checksums(checksum_io, {})
    end
    assert_includes output, "FAILED"
  end

  def test_malformed_line_with_warn
    checksum_io = StringIO.new("not a valid line\n")
    _out, err = capture_io do
      check_sha256_checksums(checksum_io, {"warn" => true})
    end
    assert_includes err, "improperly formatted"
  end

  def test_strict_mode_rejects_malformed
    checksum_io = StringIO.new("not a valid line\n")
    capture_io do
      refute check_sha256_checksums(checksum_io, {"strict" => true})
    end
  end

  def test_binary_mode_separator
    with_tempfile("test") do |path|
      expected_hash = Digest::SHA256.hexdigest("test")
      checksum_io = StringIO.new("#{expected_hash} *#{path}\n")
      capture_io do
        assert check_sha256_checksums(checksum_io, {})
      end
    end
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestSha256sumMainFunction < Minitest::Test
  include Sha256sumTestHelper

  def test_main_compute_hash
    with_tempfile("hello world") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { sha256sum_main }[0]
      expected_hash = Digest::SHA256.hexdigest("hello world")
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
      output = capture_io { sha256sum_main }[0]
      assert_includes output, " *"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_check_mode
    with_tempfile("test content") do |data_path|
      expected = Digest::SHA256.hexdigest("test content")
      checksum_content = "#{expected}  #{data_path}\n"

      with_tempfile(checksum_content) do |checksum_path|
        old_argv = ARGV.dup
        ARGV.replace(["-c", checksum_path])
        output = nil
        err = assert_raises(SystemExit) do
          output = capture_io { sha256sum_main }[0]
        end
        assert_equal 0, err.status
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz_sha256"])
    _out, err = capture_io { sha256sum_main }
    assert_includes err, "No such file"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { sha256sum_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { sha256sum_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
