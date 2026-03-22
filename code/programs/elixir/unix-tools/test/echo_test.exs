defmodule EchoTest do
  @moduledoc """
  Tests for the echo tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Business logic (escape interpretation, newline suppression).
  3. Edge cases (empty input, multiple arguments, octal escapes).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "echo.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the echo spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult with empty strings" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["echo"])
      # No strings given, so the argument should be empty or nil.
      strings = arguments["strings"]
      assert strings == nil or strings == []
    end

    test "single argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["echo", "hello"])
      assert arguments["strings"] == ["hello"]
    end

    test "multiple arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["echo", "hello", "world"])
      assert arguments["strings"] == ["hello", "world"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: -n flag (no trailing newline)
  # ---------------------------------------------------------------------------

  describe "-n flag" do
    test "sets no_newline to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["echo", "-n", "hello"])
      assert flags["no_newline"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: -e flag (enable escapes)
  # ---------------------------------------------------------------------------

  describe "-e flag" do
    test "sets enable_escapes to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["echo", "-e", "hello"])
      assert flags["enable_escapes"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["echo", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["echo", "--help"])
      assert text =~ "echo"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["echo", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["echo", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Escape interpretation business logic
  # ---------------------------------------------------------------------------

  describe "interpret_escapes/1" do
    test "backslash-n becomes newline" do
      assert UnixTools.Echo.interpret_escapes("hello\\nworld") == "hello\nworld"
    end

    test "backslash-t becomes tab" do
      assert UnixTools.Echo.interpret_escapes("hello\\tworld") == "hello\tworld"
    end

    test "backslash-backslash becomes single backslash" do
      assert UnixTools.Echo.interpret_escapes("hello\\\\world") == "hello\\world"
    end

    test "backslash-a becomes bell" do
      assert UnixTools.Echo.interpret_escapes("\\a") == <<7>>
    end

    test "backslash-b becomes backspace" do
      assert UnixTools.Echo.interpret_escapes("\\b") == <<8>>
    end

    test "backslash-f becomes form feed" do
      assert UnixTools.Echo.interpret_escapes("\\f") == <<12>>
    end

    test "backslash-r becomes carriage return" do
      assert UnixTools.Echo.interpret_escapes("\\r") == "\r"
    end

    test "octal escape \\0101 becomes A" do
      assert UnixTools.Echo.interpret_escapes("\\0101") == "A"
    end

    test "\\0 with no digits becomes null" do
      assert UnixTools.Echo.interpret_escapes("\\0") == <<0>>
    end

    test "unrecognized escape is passed through" do
      assert UnixTools.Echo.interpret_escapes("\\x") == "\\x"
    end

    test "no escapes in plain text" do
      assert UnixTools.Echo.interpret_escapes("hello world") == "hello world"
    end

    test "empty string returns empty" do
      assert UnixTools.Echo.interpret_escapes("") == ""
    end

    test "multiple escapes in sequence" do
      assert UnixTools.Echo.interpret_escapes("a\\nb\\tc") == "a\nb\tc"
    end
  end
end
