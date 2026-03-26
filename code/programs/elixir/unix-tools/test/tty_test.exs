defmodule TtyTest do
  @moduledoc """
  Tests for the tty tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-s for silent mode).
  3. Business logic (check_tty/0 detection).

  ## Note on TTY Detection

  In a test environment, stdin is typically NOT a terminal (tests run
  non-interactively). So check_tty/0 will usually return
  `{false, "not a tty"}`. We test both the interface and the expected
  non-interactive behavior.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "tty.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the tty spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["tty"])
    end

    test "silent flag defaults to false" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tty"])
      refute flags["silent"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: -s flag (silent mode)
  # ---------------------------------------------------------------------------

  describe "-s flag (silent mode)" do
    test "short flag -s sets silent to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tty", "-s"])
      assert flags["silent"] == true
    end

    test "long flag --silent sets silent to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tty", "--silent"])
      assert flags["silent"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["tty", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tty", "--help"])
      assert text =~ "tty"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tty", "--help"])
      assert String.downcase(text) =~ "terminal"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["tty", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["tty", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["tty", "--unknown"])
    end

    test "unknown short flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["tty", "-x"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic
  # ---------------------------------------------------------------------------

  describe "check_tty/0" do
    test "returns a two-element tuple" do
      {is_tty, name} = UnixTools.Tty.check_tty()
      assert is_boolean(is_tty)
      assert is_binary(name)
    end

    test "when not a tty, returns {false, \"not a tty\"}" do
      # In test environment, stdin is typically not a terminal.
      {is_tty, name} = UnixTools.Tty.check_tty()

      # We can't guarantee the test environment, but we can verify
      # the return structure is correct.
      if not is_tty do
        assert name == "not a tty"
      end
    end

    test "when is a tty, name starts with /" do
      {is_tty, name} = UnixTools.Tty.check_tty()

      if is_tty do
        assert String.starts_with?(name, "/")
      end
    end

    test "returns consistent results on repeated calls" do
      result1 = UnixTools.Tty.check_tty()
      result2 = UnixTools.Tty.check_tty()
      assert result1 == result2
    end
  end
end
