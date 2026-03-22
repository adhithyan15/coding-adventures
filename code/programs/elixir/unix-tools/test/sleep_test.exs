defmodule SleepTest do
  @moduledoc """
  Tests for the sleep tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Duration parsing (parse_duration/1 with all suffixes).
  3. Suffix extraction (extract_suffix/1).
  4. Edge cases (invalid input, floating point, multiple durations).

  ## Note on Sleep Testing

  We do NOT test the actual sleeping behavior (Process.sleep) because
  that would make tests slow. Instead, we test the pure functions that
  parse and compute durations. The sleeping itself is a trivial call
  to Process.sleep/1.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "sleep.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the sleep spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "single duration argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["sleep", "5"])
      assert arguments["duration"] == ["5"]
    end

    test "multiple duration arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["sleep", "1h", "30m"])
      assert arguments["duration"] == ["1h", "30m"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["sleep", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["sleep", "--help"])
      assert text =~ "sleep"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["sleep", "--help"])
      assert String.downcase(text) =~ "delay"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["sleep", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["sleep", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["sleep", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_duration/1 — seconds (no suffix or 's')
  # ---------------------------------------------------------------------------

  describe "parse_duration/1 — seconds" do
    test "integer seconds without suffix" do
      assert {:ok, 5.0} = UnixTools.Sleep.parse_duration("5")
    end

    test "integer seconds with 's' suffix" do
      assert {:ok, 5.0} = UnixTools.Sleep.parse_duration("5s")
    end

    test "floating point seconds without suffix" do
      assert {:ok, 1.5} = UnixTools.Sleep.parse_duration("1.5")
    end

    test "floating point seconds with 's' suffix" do
      assert {:ok, 1.5} = UnixTools.Sleep.parse_duration("1.5s")
    end

    test "zero seconds" do
      assert {:ok, +0.0} = UnixTools.Sleep.parse_duration("0")
    end

    test "zero seconds with suffix" do
      assert {:ok, +0.0} = UnixTools.Sleep.parse_duration("0s")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_duration/1 — minutes
  # ---------------------------------------------------------------------------

  describe "parse_duration/1 — minutes" do
    test "integer minutes" do
      assert {:ok, 120.0} = UnixTools.Sleep.parse_duration("2m")
    end

    test "floating point minutes" do
      assert {:ok, 90.0} = UnixTools.Sleep.parse_duration("1.5m")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_duration/1 — hours
  # ---------------------------------------------------------------------------

  describe "parse_duration/1 — hours" do
    test "integer hours" do
      assert {:ok, 3600.0} = UnixTools.Sleep.parse_duration("1h")
    end

    test "floating point hours" do
      assert {:ok, 5400.0} = UnixTools.Sleep.parse_duration("1.5h")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_duration/1 — days
  # ---------------------------------------------------------------------------

  describe "parse_duration/1 — days" do
    test "integer days" do
      assert {:ok, 86400.0} = UnixTools.Sleep.parse_duration("1d")
    end

    test "floating point days" do
      assert {:ok, 43200.0} = UnixTools.Sleep.parse_duration("0.5d")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_duration/1 — error cases
  # ---------------------------------------------------------------------------

  describe "parse_duration/1 — errors" do
    test "non-numeric input returns error" do
      assert {:error, _} = UnixTools.Sleep.parse_duration("abc")
    end

    test "empty string returns error" do
      assert {:error, _} = UnixTools.Sleep.parse_duration("")
    end

    test "just a suffix returns error" do
      assert {:error, _} = UnixTools.Sleep.parse_duration("m")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: extract_suffix/1
  # ---------------------------------------------------------------------------

  describe "extract_suffix/1" do
    test "no suffix returns the string and multiplier 1.0" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("5")
      assert numeric == "5"
      assert multiplier == 1.0
    end

    test "'s' suffix returns multiplier 1.0" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("5s")
      assert numeric == "5"
      assert multiplier == 1.0
    end

    test "'m' suffix returns multiplier 60.0" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("2m")
      assert numeric == "2"
      assert multiplier == 60.0
    end

    test "'h' suffix returns multiplier 3600.0" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("1h")
      assert numeric == "1"
      assert multiplier == 3600.0
    end

    test "'d' suffix returns multiplier 86400.0" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("1d")
      assert numeric == "1"
      assert multiplier == 86400.0
    end

    test "floating point with suffix" do
      {numeric, multiplier} = UnixTools.Sleep.extract_suffix("1.5h")
      assert numeric == "1.5"
      assert multiplier == 3600.0
    end
  end
end
