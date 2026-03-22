defmodule YesTest do
  @moduledoc """
  Tests for the yes tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Business logic (yes_output/2 for testable finite output).
  3. Argument handling (default "y", custom string, variadic args).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "yes.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the yes spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["yes"])
    end

    test "string argument defaults when not provided" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["yes"])
      # No string given, so the argument should be nil or default.
      strings = arguments["string"]
      assert strings == nil or strings == [] or strings == "y"
    end

    test "single string argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["yes", "hello"])
      assert arguments["string"] == ["hello"]
    end

    test "multiple string arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["yes", "hello", "world"])
      assert arguments["string"] == ["hello", "world"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["yes", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["yes", "--help"])
      assert text =~ "yes"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["yes", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["yes", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["yes", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: yes_output/2 business logic
  # ---------------------------------------------------------------------------

  describe "yes_output/2" do
    test "generates correct number of lines with default string" do
      result = UnixTools.Yes.yes_output("y", 5)
      assert result == ["y", "y", "y", "y", "y"]
    end

    test "generates correct number of lines with custom string" do
      result = UnixTools.Yes.yes_output("hello", 3)
      assert result == ["hello", "hello", "hello"]
    end

    test "generates zero lines when max_lines is 0" do
      result = UnixTools.Yes.yes_output("y", 0)
      assert result == []
    end

    test "generates one line when max_lines is 1" do
      result = UnixTools.Yes.yes_output("y", 1)
      assert result == ["y"]
    end

    test "preserves multi-word strings" do
      result = UnixTools.Yes.yes_output("hello world", 2)
      assert result == ["hello world", "hello world"]
    end

    test "works with empty string" do
      result = UnixTools.Yes.yes_output("", 3)
      assert result == ["", "", ""]
    end

    test "generates large number of lines" do
      result = UnixTools.Yes.yes_output("y", 1000)
      assert length(result) == 1000
      assert Enum.all?(result, &(&1 == "y"))
    end
  end
end
