defmodule Sha256sumTest do
  @moduledoc """
  Tests for the sha256sum tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. SHA-256 hash computation with known test vectors.
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

  @spec_path Path.join([__DIR__, "..", "sha256sum.json"]) |> Path.expand()

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
      assert {:ok, %ParseResult{}} = parse_argv(["sha256sum"])
    end

    test "-c sets check to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sha256sum", "-c", "sums.sha256"])
      assert flags["check"] == true
    end

    test "-b sets binary to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sha256sum", "-b", "file.txt"])
      assert flags["binary"] == true
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["sha256sum", "a.txt", "b.txt"])
      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["sha256sum", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["sha256sum", "--help"])
      assert text =~ "sha256sum"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["sha256sum", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["sha256sum", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["sha256sum", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_sha256/1
  # ---------------------------------------------------------------------------

  describe "compute_sha256/1" do
    test "empty string hash" do
      # The SHA-256 hash of the empty string is a well-known constant.
      assert UnixTools.Sha256sum.compute_sha256("") ==
               "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    end

    test "hash of 'hello\\n'" do
      assert UnixTools.Sha256sum.compute_sha256("hello\n") ==
               "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
    end

    test "hash of 'abc'" do
      assert UnixTools.Sha256sum.compute_sha256("abc") ==
               "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    end

    test "hash is always 64 hex characters" do
      hash = UnixTools.Sha256sum.compute_sha256("test data")
      assert String.length(hash) == 64
      assert hash =~ ~r/^[0-9a-f]+$/
    end

    test "different inputs produce different hashes" do
      hash1 = UnixTools.Sha256sum.compute_sha256("hello")
      hash2 = UnixTools.Sha256sum.compute_sha256("world")
      assert hash1 != hash2
    end

    test "identical inputs produce identical hashes" do
      hash1 = UnixTools.Sha256sum.compute_sha256("test")
      hash2 = UnixTools.Sha256sum.compute_sha256("test")
      assert hash1 == hash2
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_sha256_file/1
  # ---------------------------------------------------------------------------

  describe "compute_sha256_file/1" do
    setup do
      tmp_file = Path.join(System.tmp_dir!(), "sha256_test_#{:rand.uniform(100_000)}.txt")
      File.write!(tmp_file, "hello\n")

      on_exit(fn -> File.rm(tmp_file) end)

      %{tmp_file: tmp_file}
    end

    test "computes hash of a file", %{tmp_file: tmp_file} do
      assert {:ok, hash} = UnixTools.Sha256sum.compute_sha256_file(tmp_file)
      assert hash == "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
    end

    test "returns error for nonexistent file" do
      assert {:error, :enoent} = UnixTools.Sha256sum.compute_sha256_file("/nonexistent/file.txt")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_hash/3
  # ---------------------------------------------------------------------------

  describe "format_hash/3" do
    test "text mode format (two spaces)" do
      assert UnixTools.Sha256sum.format_hash("abc123", "file.txt") == "abc123  file.txt"
    end

    test "text mode format explicit" do
      assert UnixTools.Sha256sum.format_hash("abc123", "file.txt", false) == "abc123  file.txt"
    end

    test "binary mode format (space + asterisk)" do
      assert UnixTools.Sha256sum.format_hash("abc123", "file.txt", true) == "abc123 *file.txt"
    end

    test "stdin filename" do
      assert UnixTools.Sha256sum.format_hash("abc123", "-") == "abc123  -"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_checksum_line/1
  # ---------------------------------------------------------------------------

  describe "parse_checksum_line/1" do
    @empty_hash "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    test "parses text mode line" do
      result = UnixTools.Sha256sum.parse_checksum_line("#{@empty_hash}  empty.txt")
      assert {:ok, {@empty_hash, "empty.txt"}} = result
    end

    test "parses binary mode line" do
      result = UnixTools.Sha256sum.parse_checksum_line("#{@empty_hash} *empty.txt")
      assert {:ok, {@empty_hash, "empty.txt"}} = result
    end

    test "normalizes hash to lowercase" do
      upper_hash = String.upcase(@empty_hash)
      result = UnixTools.Sha256sum.parse_checksum_line("#{upper_hash}  file.txt")
      {:ok, {hash, _}} = result
      assert hash == @empty_hash
    end

    test "rejects invalid format" do
      result = UnixTools.Sha256sum.parse_checksum_line("not a valid checksum line")
      assert {:error, :invalid_format} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Test: verify_file/2
  # ---------------------------------------------------------------------------

  describe "verify_file/2" do
    setup do
      tmp_file = Path.join(System.tmp_dir!(), "sha256_verify_#{:rand.uniform(100_000)}.txt")
      File.write!(tmp_file, "")

      on_exit(fn -> File.rm(tmp_file) end)

      %{tmp_file: tmp_file}
    end

    test "returns :ok when hash matches", %{tmp_file: tmp_file} do
      expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
      assert :ok = UnixTools.Sha256sum.verify_file(tmp_file, expected)
    end

    test "returns {:error, :mismatch} when hash doesn't match", %{tmp_file: tmp_file} do
      assert {:error, :mismatch} = UnixTools.Sha256sum.verify_file(tmp_file, "0000")
    end

    test "returns {:error, reason} for nonexistent file" do
      assert {:error, :enoent} = UnixTools.Sha256sum.verify_file("/nonexistent", "abc")
    end
  end
end
