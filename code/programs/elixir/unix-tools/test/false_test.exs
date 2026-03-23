defmodule FalseToolTest do
  @moduledoc """
  Tests for the false tool.

  ## What These Tests Verify

  These tests exercise the CLI Builder integration for the `false` utility.
  Like `true`, `false` has no flags and no arguments beyond --help and
  --version. The tests verify:

  1. Normal invocation returns a ParseResult (the program would exit 1).
  2. --help returns a HelpResult with meaningful text.
  3. --version returns a VersionResult with the correct version string.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "false.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the false spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior (no arguments)
  # ---------------------------------------------------------------------------

  describe "default behavior (no arguments)" do
    test "returns {:ok, %ParseResult{}}" do
      assert {:ok, %ParseResult{}} = parse_argv(["false"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["false", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["false", "--help"])
      assert text =~ "false"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["false", "--help"])
      assert String.downcase(text) =~ "nothing"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["false", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["false", "--version"])
      assert version == "1.0.0"
    end
  end
end
