defmodule SeqTest do
  @moduledoc """
  Tests for the seq tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Business logic (generate_sequence, decimal_places, format_number).
  3. Edge cases (floating point, descending sequences, equal width).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "seq.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the seq spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "single number argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["seq", "5"])
      assert arguments["numbers"] == ["5"]
    end

    test "two number arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["seq", "3", "7"])
      assert arguments["numbers"] == ["3", "7"]
    end

    test "three number arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["seq", "1", "2", "10"])
      assert arguments["numbers"] == ["1", "2", "10"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-s sets separator" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["seq", "-s", ", ", "5"])
      assert flags["separator"] == ", "
    end

    test "-w sets equal_width" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["seq", "-w", "5"])
      assert flags["equal_width"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["seq", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["seq", "--help"])
      assert text =~ "seq"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["seq", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["seq", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["seq", "--unknown", "5"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - decimal_places
  # ---------------------------------------------------------------------------

  describe "decimal_places/1" do
    test "integer has 0 decimal places" do
      assert UnixTools.Seq.decimal_places("3") == 0
    end

    test "one decimal place" do
      assert UnixTools.Seq.decimal_places("1.5") == 1
    end

    test "two decimal places" do
      assert UnixTools.Seq.decimal_places("0.25") == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_number
  # ---------------------------------------------------------------------------

  describe "format_number/2" do
    test "integer formatting" do
      assert UnixTools.Seq.format_number(3.0, 0) == "3"
    end

    test "one decimal place formatting" do
      assert UnixTools.Seq.format_number(1.5, 1) == "1.5"
    end

    test "two decimal place formatting" do
      assert UnixTools.Seq.format_number(0.25, 2) == "0.25"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - generate_sequence
  # ---------------------------------------------------------------------------

  describe "generate_sequence/4" do
    test "simple ascending sequence" do
      assert UnixTools.Seq.generate_sequence(1.0, 1.0, 5.0, 0) == ["1", "2", "3", "4", "5"]
    end

    test "ascending with increment of 2" do
      assert UnixTools.Seq.generate_sequence(1.0, 2.0, 7.0, 0) == ["1", "3", "5", "7"]
    end

    test "descending sequence" do
      assert UnixTools.Seq.generate_sequence(5.0, -1.0, 1.0, 0) == ["5", "4", "3", "2", "1"]
    end

    test "floating point sequence" do
      result = UnixTools.Seq.generate_sequence(0.5, 0.5, 2.0, 1)
      assert result == ["0.5", "1.0", "1.5", "2.0"]
    end

    test "single element sequence" do
      assert UnixTools.Seq.generate_sequence(5.0, 1.0, 5.0, 0) == ["5"]
    end

    test "empty sequence when first > last with positive increment" do
      assert UnixTools.Seq.generate_sequence(5.0, 1.0, 3.0, 0) == []
    end

    test "zero increment returns empty" do
      assert UnixTools.Seq.generate_sequence(1.0, 0.0, 5.0, 0) == []
    end
  end
end
