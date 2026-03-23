defmodule Md5sumTest do
  @moduledoc """
  Tests for the md5sum tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. MD5 hash computation with known test vectors.
  3. File hashing.
  4. Output formatting (text mode, binary mode).
  5. Checksum line parsing.
  6. File verification.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "md5sum.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["md5sum"])
    end

    test "-c sets check to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["md5sum", "-c", "sums.md5"])
      assert flags["check"] == true
    end

    test "-b sets binary to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["md5sum", "-b", "file.txt"])
      assert flags["binary"] == true
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["md5sum", "a.txt", "b.txt"])
      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["md5sum", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["md5sum", "--help"])
      assert text =~ "md5sum"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["md5sum", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["md5sum", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["md5sum", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_md5/1
  # ---------------------------------------------------------------------------

  describe "compute_md5/1" do
    test "empty string hash" do
      # The MD5 hash of the empty string is a well-known constant.
      assert UnixTools.Md5sum.compute_md5("") == "d41d8cd98f00b204e9800998ecf8427e"
    end

    test "hash of 'hello\\n'" do
      assert UnixTools.Md5sum.compute_md5("hello\n") == "b1946ac92492d2347c6235b4d2611184"
    end

    test "hash of 'abc'" do
      assert UnixTools.Md5sum.compute_md5("abc") == "900150983cd24fb0d6963f7d28e17f72"
    end

    test "hash is always 32 hex characters" do
      hash = UnixTools.Md5sum.compute_md5("test data")
      assert String.length(hash) == 32
      assert hash =~ ~r/^[0-9a-f]+$/
    end

    test "different inputs produce different hashes" do
      hash1 = UnixTools.Md5sum.compute_md5("hello")
      hash2 = UnixTools.Md5sum.compute_md5("world")
      assert hash1 != hash2
    end

    test "identical inputs produce identical hashes" do
      hash1 = UnixTools.Md5sum.compute_md5("test")
      hash2 = UnixTools.Md5sum.compute_md5("test")
      assert hash1 == hash2
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_md5_file/1
  # ---------------------------------------------------------------------------

  describe "compute_md5_file/1" do
    setup do
      tmp_file = Path.join(System.tmp_dir!(), "md5_test_#{:rand.uniform(100_000)}.txt")
      File.write!(tmp_file, "hello\n")

      on_exit(fn -> File.rm(tmp_file) end)

      %{tmp_file: tmp_file}
    end

    test "computes hash of a file", %{tmp_file: tmp_file} do
      assert {:ok, hash} = UnixTools.Md5sum.compute_md5_file(tmp_file)
      assert hash == "b1946ac92492d2347c6235b4d2611184"
    end

    test "returns error for nonexistent file" do
      assert {:error, :enoent} = UnixTools.Md5sum.compute_md5_file("/nonexistent/file.txt")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_hash/3
  # ---------------------------------------------------------------------------

  describe "format_hash/3" do
    test "text mode format (two spaces)" do
      assert UnixTools.Md5sum.format_hash("abc123", "file.txt") == "abc123  file.txt"
    end

    test "text mode format explicit" do
      assert UnixTools.Md5sum.format_hash("abc123", "file.txt", false) == "abc123  file.txt"
    end

    test "binary mode format (space + asterisk)" do
      assert UnixTools.Md5sum.format_hash("abc123", "file.txt", true) == "abc123 *file.txt"
    end

    test "stdin filename" do
      assert UnixTools.Md5sum.format_hash("abc123", "-") == "abc123  -"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_checksum_line/1
  # ---------------------------------------------------------------------------

  describe "parse_checksum_line/1" do
    test "parses text mode line" do
      result = UnixTools.Md5sum.parse_checksum_line("d41d8cd98f00b204e9800998ecf8427e  empty.txt")
      assert {:ok, {"d41d8cd98f00b204e9800998ecf8427e", "empty.txt"}} = result
    end

    test "parses binary mode line" do
      result = UnixTools.Md5sum.parse_checksum_line("d41d8cd98f00b204e9800998ecf8427e *empty.txt")
      assert {:ok, {"d41d8cd98f00b204e9800998ecf8427e", "empty.txt"}} = result
    end

    test "normalizes hash to lowercase" do
      result = UnixTools.Md5sum.parse_checksum_line("D41D8CD98F00B204E9800998ECF8427E  file.txt")
      {:ok, {hash, _}} = result
      assert hash == "d41d8cd98f00b204e9800998ecf8427e"
    end

    test "rejects invalid format" do
      result = UnixTools.Md5sum.parse_checksum_line("not a valid checksum line")
      assert {:error, :invalid_format} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Test: verify_file/2
  # ---------------------------------------------------------------------------

  describe "verify_file/2" do
    setup do
      tmp_file = Path.join(System.tmp_dir!(), "md5_verify_#{:rand.uniform(100_000)}.txt")
      File.write!(tmp_file, "")

      on_exit(fn -> File.rm(tmp_file) end)

      %{tmp_file: tmp_file}
    end

    test "returns :ok when hash matches", %{tmp_file: tmp_file} do
      # Hash of empty file
      assert :ok = UnixTools.Md5sum.verify_file(tmp_file, "d41d8cd98f00b204e9800998ecf8427e")
    end

    test "returns {:error, :mismatch} when hash doesn't match", %{tmp_file: tmp_file} do
      assert {:error, :mismatch} = UnixTools.Md5sum.verify_file(tmp_file, "0000000000000000000000000000000")
    end

    test "returns {:error, reason} for nonexistent file" do
      assert {:error, :enoent} = UnixTools.Md5sum.verify_file("/nonexistent", "abc")
    end
  end
end
