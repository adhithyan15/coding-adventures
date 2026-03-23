defmodule EnvTest do
  @moduledoc """
  Tests for the env tool (UnixTools.EnvTool).

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Assignment parsing (NAME=VALUE pairs vs command tokens).
  3. Environment building (base env, unset, ignore, assignments).
  4. Environment formatting (newline vs NUL separator).
  5. Edge cases (empty values, multiple assignments, no command).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "env.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "-i sets ignore-environment" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["env", "-i", "bash"])
      assert flags["ignore_environment"] == true
    end

    test "-u sets unset variable" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["env", "-u", "HOME", "bash"])
      # Repeatable flags return a list
      assert flags["unset"] == ["HOME"]
    end

    test "-0 sets null terminator" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["env", "-0"])
      assert flags["null"] == true
    end

    test "-C sets chdir" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["env", "-C", "/tmp", "bash"])
      assert flags["chdir"] == "/tmp"
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["env", "--help"])
      assert text =~ "env"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["env", "--version"])
      assert version =~ "1.0.0"
    end

    test "arguments are captured as variadic" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["env", "FOO=bar", "echo"])
      assert arguments["assignments_and_command"] == ["FOO=bar", "echo"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_assignments
  # ---------------------------------------------------------------------------

  describe "parse_assignments" do
    test "simple KEY=VALUE pair" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["FOO=bar", "echo"])
      assert assigns == [{"FOO", "bar"}]
      assert cmd == ["echo"]
    end

    test "multiple assignments before command" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["A=1", "B=2", "echo", "hello"])
      assert assigns == [{"A", "1"}, {"B", "2"}]
      assert cmd == ["echo", "hello"]
    end

    test "no assignments — just command" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["echo", "hello"])
      assert assigns == []
      assert cmd == ["echo", "hello"]
    end

    test "no command — just assignments" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["FOO=bar", "BAZ=qux"])
      assert assigns == [{"FOO", "bar"}, {"BAZ", "qux"}]
      assert cmd == []
    end

    test "empty value" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["FOO=", "echo"])
      assert assigns == [{"FOO", ""}]
      assert cmd == ["echo"]
    end

    test "value with equals sign" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["FOO=a=b=c", "echo"])
      assert assigns == [{"FOO", "a=b=c"}]
      assert cmd == ["echo"]
    end

    test "command argument with equals is not an assignment" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments(["echo", "A=1"])
      assert assigns == []
      assert cmd == ["echo", "A=1"]
    end

    test "empty list" do
      {assigns, cmd} = UnixTools.EnvTool.parse_assignments([])
      assert assigns == []
      assert cmd == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_single_assignment
  # ---------------------------------------------------------------------------

  describe "parse_single_assignment" do
    test "valid assignment" do
      assert {:ok, "HOME", "/Users/alice"} = UnixTools.EnvTool.parse_single_assignment("HOME=/Users/alice")
    end

    test "underscore in variable name" do
      assert {:ok, "MY_VAR", "value"} = UnixTools.EnvTool.parse_single_assignment("MY_VAR=value")
    end

    test "digit in variable name (not first)" do
      assert {:ok, "VAR1", "value"} = UnixTools.EnvTool.parse_single_assignment("VAR1=value")
    end

    test "non-assignment (no equals)" do
      assert :not_assignment = UnixTools.EnvTool.parse_single_assignment("echo")
    end

    test "invalid variable name (starts with digit)" do
      assert :not_assignment = UnixTools.EnvTool.parse_single_assignment("1FOO=bar")
    end

    test "empty key is not assignment" do
      assert :not_assignment = UnixTools.EnvTool.parse_single_assignment("=value")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_environment
  # ---------------------------------------------------------------------------

  describe "build_environment" do
    test "adds assignments to current env" do
      current = %{"HOME" => "/home/user"}
      result = UnixTools.EnvTool.build_environment(current, [{"FOO", "bar"}], %{})
      assert result["HOME"] == "/home/user"
      assert result["FOO"] == "bar"
    end

    test "assignments override existing variables" do
      current = %{"FOO" => "old"}
      result = UnixTools.EnvTool.build_environment(current, [{"FOO", "new"}], %{})
      assert result["FOO"] == "new"
    end

    test "ignore_env starts with empty environment" do
      current = %{"HOME" => "/home/user", "PATH" => "/usr/bin"}
      result = UnixTools.EnvTool.build_environment(current, [{"FOO", "bar"}], %{ignore_env: true})
      assert result == %{"FOO" => "bar"}
      refute Map.has_key?(result, "HOME")
    end

    test "unset removes specific variables" do
      current = %{"HOME" => "/home/user", "PATH" => "/usr/bin", "SHELL" => "/bin/zsh"}
      result = UnixTools.EnvTool.build_environment(current, [], %{unset_vars: ["PATH", "SHELL"]})
      assert result == %{"HOME" => "/home/user"}
    end

    test "unset then assign same variable" do
      current = %{"FOO" => "old"}
      result = UnixTools.EnvTool.build_environment(current, [{"FOO", "new"}], %{unset_vars: ["FOO"]})
      # Unset happens first, then assignment overrides
      assert result["FOO"] == "new"
    end

    test "empty current env with assignments" do
      result = UnixTools.EnvTool.build_environment(%{}, [{"A", "1"}, {"B", "2"}], %{})
      assert result == %{"A" => "1", "B" => "2"}
    end

    test "no modifications returns current env" do
      current = %{"HOME" => "/home/user"}
      result = UnixTools.EnvTool.build_environment(current, [], %{})
      assert result == current
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_environment integration
  # ---------------------------------------------------------------------------

  describe "build_environment integration" do
    test "ignore env + assignments creates minimal env" do
      current = %{"HOME" => "/home", "PATH" => "/usr/bin", "SHELL" => "/bin/zsh"}
      result = UnixTools.EnvTool.build_environment(
        current,
        [{"ONLY", "this"}],
        %{ignore_env: true, unset_vars: []}
      )
      assert result == %{"ONLY" => "this"}
    end

    test "multiple unset + multiple assignments" do
      current = %{"A" => "1", "B" => "2", "C" => "3", "D" => "4"}
      result = UnixTools.EnvTool.build_environment(
        current,
        [{"E", "5"}, {"F", "6"}],
        %{unset_vars: ["A", "C"]}
      )
      assert result == %{"B" => "2", "D" => "4", "E" => "5", "F" => "6"}
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_environment
  # ---------------------------------------------------------------------------

  describe "format_environment" do
    test "formats as NAME=VALUE with newlines" do
      env = %{"A" => "1", "B" => "2"}
      result = UnixTools.EnvTool.format_environment(env, %{})
      assert result == "A=1\nB=2"
    end

    test "formats as NAME=VALUE with NUL separators" do
      env = %{"A" => "1", "B" => "2"}
      result = UnixTools.EnvTool.format_environment(env, %{null_terminator: true})
      assert result == "A=1\0B=2"
    end

    test "empty environment produces empty string" do
      result = UnixTools.EnvTool.format_environment(%{}, %{})
      assert result == ""
    end

    test "single variable" do
      result = UnixTools.EnvTool.format_environment(%{"FOO" => "bar"}, %{})
      assert result == "FOO=bar"
    end

    test "sorted by key" do
      env = %{"Z" => "last", "A" => "first", "M" => "middle"}
      result = UnixTools.EnvTool.format_environment(env, %{})
      lines = String.split(result, "\n")
      assert Enum.at(lines, 0) == "A=first"
      assert Enum.at(lines, 1) == "M=middle"
      assert Enum.at(lines, 2) == "Z=last"
    end

    test "value with special characters" do
      env = %{"PATH" => "/usr/bin:/usr/local/bin"}
      result = UnixTools.EnvTool.format_environment(env, %{})
      assert result == "PATH=/usr/bin:/usr/local/bin"
    end

    test "empty value" do
      env = %{"EMPTY" => ""}
      result = UnixTools.EnvTool.format_environment(env, %{})
      assert result == "EMPTY="
    end
  end
end
