defmodule BasenameTest do
  @moduledoc """
  Tests for the basename tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (compute_basename with various paths and suffixes).
  3. Edge cases (trailing slashes, all-slashes path, suffix equals name).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "basename.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the basename spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "single name argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["basename", "/usr/bin/sort"])
      assert arguments["name"] == ["/usr/bin/sort"]
    end

    test "multiple name arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["basename", "-a", "/usr/bin/sort", "/usr/bin/ls"])

      assert arguments["name"] == ["/usr/bin/sort", "/usr/bin/ls"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-a sets multiple to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["basename", "-a", "file"])
      assert flags["multiple"] == true
    end

    test "-s sets suffix" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["basename", "-s", ".h", "file.h"])
      assert flags["suffix"] == ".h"
    end

    test "-z sets zero to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["basename", "-z", "file"])
      assert flags["zero"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["basename", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["basename", "--help"])
      assert text =~ "basename"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["basename", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["basename", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["basename", "--unknown", "file"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - compute_basename
  # ---------------------------------------------------------------------------

  describe "compute_basename/2" do
    test "strips directory from absolute path" do
      assert UnixTools.BasenameTool.compute_basename("/usr/bin/sort") == "sort"
    end

    test "strips directory from relative path" do
      assert UnixTools.BasenameTool.compute_basename("include/stdio.h") == "stdio.h"
    end

    test "removes suffix when specified" do
      assert UnixTools.BasenameTool.compute_basename("stdio.h", ".h") == "stdio"
    end

    test "does not remove suffix when name equals suffix" do
      assert UnixTools.BasenameTool.compute_basename(".h", ".h") == ".h"
    end

    test "handles trailing slashes" do
      assert UnixTools.BasenameTool.compute_basename("/usr/bin/") == "bin"
    end

    test "handles all-slashes path" do
      assert UnixTools.BasenameTool.compute_basename("///") == "/"
    end

    test "handles root path" do
      assert UnixTools.BasenameTool.compute_basename("/") == "/"
    end

    test "handles plain filename" do
      assert UnixTools.BasenameTool.compute_basename("hello.txt") == "hello.txt"
    end

    test "strips suffix from path with directory" do
      assert UnixTools.BasenameTool.compute_basename("/usr/include/stdio.h", ".h") == "stdio"
    end

    test "nil suffix is ignored" do
      assert UnixTools.BasenameTool.compute_basename("/usr/bin/sort", nil) == "sort"
    end
  end
end
