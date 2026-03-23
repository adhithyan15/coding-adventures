defmodule XargsTest do
  @moduledoc """
  Tests for the xargs tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Input splitting (whitespace, null, custom delimiter).
  3. Quote handling in whitespace mode.
  4. Command building (batching, replacement mode).
  5. EOF string handling.
  6. Command execution.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "xargs.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "command argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["xargs", "echo", "hello"])
      assert arguments["command"] == ["echo", "hello"]
    end

    test "-0 sets null delimiter" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-0", "echo"])
      assert flags["null"] == true
    end

    test "-d sets custom delimiter" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-d", ",", "echo"])
      assert flags["delimiter"] == ","
    end

    test "-n sets max args" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-n", "3", "echo"])
      assert flags["max_args"] == 3
    end

    test "-I sets replace string" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-I", "{}", "echo", "{}"])
      assert flags["replace"] == "{}"
    end

    test "-t sets verbose" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-t", "echo"])
      assert flags["verbose"] == true
    end

    test "-r sets no-run-if-empty" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["xargs", "-r", "echo"])
      assert flags["no_run_if_empty"] == true
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["xargs", "--help"])
      assert text =~ "xargs"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["xargs", "--version"])
      assert version =~ "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_input — whitespace mode
  # ---------------------------------------------------------------------------

  describe "split_input whitespace mode" do
    test "splits on spaces" do
      assert UnixTools.Xargs.split_input("a b c", %{}) == ["a", "b", "c"]
    end

    test "splits on tabs" do
      assert UnixTools.Xargs.split_input("a\tb\tc", %{}) == ["a", "b", "c"]
    end

    test "splits on newlines" do
      assert UnixTools.Xargs.split_input("a\nb\nc", %{}) == ["a", "b", "c"]
    end

    test "handles multiple spaces" do
      assert UnixTools.Xargs.split_input("a   b   c", %{}) == ["a", "b", "c"]
    end

    test "handles mixed whitespace" do
      assert UnixTools.Xargs.split_input("a \t b \n c", %{}) == ["a", "b", "c"]
    end

    test "empty input returns empty list" do
      assert UnixTools.Xargs.split_input("", %{}) == []
    end

    test "whitespace-only input returns empty list" do
      assert UnixTools.Xargs.split_input("   \t\n  ", %{}) == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_input — null delimiter mode
  # ---------------------------------------------------------------------------

  describe "split_input null delimiter mode" do
    test "splits on null bytes" do
      assert UnixTools.Xargs.split_input("a\0b\0c", %{null_delimiter: true}) == ["a", "b", "c"]
    end

    test "preserves spaces in null mode" do
      result = UnixTools.Xargs.split_input("hello world\0foo bar", %{null_delimiter: true})
      assert result == ["hello world", "foo bar"]
    end

    test "empty input returns empty list" do
      assert UnixTools.Xargs.split_input("", %{null_delimiter: true}) == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_input — custom delimiter
  # ---------------------------------------------------------------------------

  describe "split_input custom delimiter" do
    test "splits on comma" do
      assert UnixTools.Xargs.split_input("a,b,c", %{delimiter: ","}) == ["a", "b", "c"]
    end

    test "splits on pipe" do
      assert UnixTools.Xargs.split_input("a|b|c", %{delimiter: "|"}) == ["a", "b", "c"]
    end

    test "preserves whitespace with custom delimiter" do
      result = UnixTools.Xargs.split_input("hello world,foo bar", %{delimiter: ","})
      assert result == ["hello world", "foo bar"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_input — EOF string
  # ---------------------------------------------------------------------------

  describe "split_input with EOF string" do
    test "stops at EOF string" do
      result = UnixTools.Xargs.split_input("a b END c d", %{eof_str: "END"})
      assert result == ["a", "b"]
    end

    test "no EOF string found — all items returned" do
      result = UnixTools.Xargs.split_input("a b c", %{eof_str: "END"})
      assert result == ["a", "b", "c"]
    end

    test "EOF string at beginning — empty result" do
      result = UnixTools.Xargs.split_input("END a b c", %{eof_str: "END"})
      assert result == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_whitespace_with_quotes
  # ---------------------------------------------------------------------------

  describe "split_whitespace_with_quotes" do
    test "simple words" do
      assert UnixTools.Xargs.split_whitespace_with_quotes("a b c") == ["a", "b", "c"]
    end

    test "double-quoted string preserved" do
      result = UnixTools.Xargs.split_whitespace_with_quotes(~s|hello "world peace" test|)
      assert result == ["hello", "world peace", "test"]
    end

    test "single-quoted string preserved" do
      result = UnixTools.Xargs.split_whitespace_with_quotes("hello 'world peace' test")
      assert result == ["hello", "world peace", "test"]
    end

    test "empty input" do
      assert UnixTools.Xargs.split_whitespace_with_quotes("") == []
    end

    test "only whitespace" do
      assert UnixTools.Xargs.split_whitespace_with_quotes("   ") == []
    end

    test "adjacent quotes" do
      result = UnixTools.Xargs.split_whitespace_with_quotes(~s|"a" "b"|)
      assert result == ["a", "b"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_commands — default mode
  # ---------------------------------------------------------------------------

  describe "build_commands default mode" do
    test "all items in one command" do
      result = UnixTools.Xargs.build_commands(["echo"], ["a", "b", "c"], %{})
      assert result == [{"echo", ["a", "b", "c"]}]
    end

    test "command with base args" do
      result = UnixTools.Xargs.build_commands(["echo", "-n"], ["a", "b"], %{})
      assert result == [{"echo", ["-n", "a", "b"]}]
    end

    test "empty items produce command with no extra args" do
      result = UnixTools.Xargs.build_commands(["echo"], [], %{})
      assert result == [{"echo", []}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_commands — batch mode (-n)
  # ---------------------------------------------------------------------------

  describe "build_commands batch mode" do
    test "splits into chunks of max_args" do
      result = UnixTools.Xargs.build_commands(["echo"], ["a", "b", "c", "d", "e"], %{max_args: 2})
      assert result == [
        {"echo", ["a", "b"]},
        {"echo", ["c", "d"]},
        {"echo", ["e"]}
      ]
    end

    test "max_args larger than items" do
      result = UnixTools.Xargs.build_commands(["echo"], ["a", "b"], %{max_args: 5})
      assert result == [{"echo", ["a", "b"]}]
    end

    test "max_args of 1" do
      result = UnixTools.Xargs.build_commands(["echo"], ["a", "b", "c"], %{max_args: 1})
      assert result == [
        {"echo", ["a"]},
        {"echo", ["b"]},
        {"echo", ["c"]}
      ]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_commands — replacement mode (-I)
  # ---------------------------------------------------------------------------

  describe "build_commands replacement mode" do
    test "replaces placeholder in args" do
      result = UnixTools.Xargs.build_commands(
        ["echo", "item: {}"],
        ["a", "b"],
        %{replace_str: "{}"}
      )
      assert result == [
        {"echo", ["item: a"]},
        {"echo", ["item: b"]}
      ]
    end

    test "multiple placeholders in args" do
      result = UnixTools.Xargs.build_commands(
        ["mv", "{}", "{}.bak"],
        ["file1", "file2"],
        %{replace_str: "{}"}
      )
      assert result == [
        {"mv", ["file1", "file1.bak"]},
        {"mv", ["file2", "file2.bak"]}
      ]
    end

    test "no placeholder in args — unchanged" do
      result = UnixTools.Xargs.build_commands(
        ["echo", "hello"],
        ["a", "b"],
        %{replace_str: "{}"}
      )
      assert result == [
        {"echo", ["hello"]},
        {"echo", ["hello"]}
      ]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: execute_command
  # ---------------------------------------------------------------------------

  describe "execute_command" do
    test "runs echo command" do
      {output, exit_code} = UnixTools.Xargs.execute_command({"echo", ["hello", "world"]}, %{})
      assert String.trim(output) == "hello world"
      assert exit_code == 0
    end

    test "captures exit code" do
      {_output, exit_code} = UnixTools.Xargs.execute_command({"false", []}, %{})
      assert exit_code != 0
    end

    test "nonexistent command returns error" do
      {_output, exit_code} = UnixTools.Xargs.execute_command(
        {"nonexistent_command_xyz_12345", []}, %{}
      )
      assert exit_code != 0
    end

    test "verbose mode prints command to stderr" do
      output = ExUnit.CaptureIO.capture_io(:stderr, fn ->
        UnixTools.Xargs.execute_command({"echo", ["hi"]}, %{verbose: true})
      end)
      assert output =~ "echo hi"
    end
  end
end
