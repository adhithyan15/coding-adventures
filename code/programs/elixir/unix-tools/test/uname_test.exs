defmodule UnameTest do
  @moduledoc """
  Tests for the uname tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. System info gathering (kernel name, hostname, machine).
  3. Output formatting with field selection.
  4. The -a (all) flag.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "uname.json"]) |> Path.expand()

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
      assert {:ok, %ParseResult{}} = parse_argv(["uname"])
    end

    test "-a sets all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uname", "-a"])
      assert flags["all"] == true
    end

    test "-s sets kernel_name to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uname", "-s"])
      assert flags["kernel_name"] == true
    end

    test "-n sets nodename to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uname", "-n"])
      assert flags["nodename"] == true
    end

    test "-m sets machine to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uname", "-m"])
      assert flags["machine"] == true
    end

    test "-r sets kernel_release to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uname", "-r"])
      assert flags["kernel_release"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["uname", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["uname", "--help"])
      assert text =~ "uname"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["uname", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["uname", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["uname", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_system_info/0
  # ---------------------------------------------------------------------------

  describe "get_system_info/0" do
    test "returns a map with expected keys" do
      info = UnixTools.Uname.get_system_info()
      assert is_map(info)
      assert Map.has_key?(info, :kernel_name)
      assert Map.has_key?(info, :nodename)
      assert Map.has_key?(info, :kernel_release)
      assert Map.has_key?(info, :kernel_version)
      assert Map.has_key?(info, :machine)
      assert Map.has_key?(info, :processor)
      assert Map.has_key?(info, :hardware_platform)
      assert Map.has_key?(info, :operating_system)
    end

    test "kernel_name is a non-empty string" do
      info = UnixTools.Uname.get_system_info()
      assert is_binary(info.kernel_name)
      assert String.length(info.kernel_name) > 0
    end

    test "nodename is a non-empty string" do
      info = UnixTools.Uname.get_system_info()
      assert is_binary(info.nodename)
      assert String.length(info.nodename) > 0
    end

    test "machine is a non-empty string" do
      info = UnixTools.Uname.get_system_info()
      assert is_binary(info.machine)
      assert String.length(info.machine) > 0
    end

    test "kernel_name is Darwin or Linux on Unix" do
      info = UnixTools.Uname.get_system_info()
      assert info.kernel_name in ["Darwin", "Linux"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_output/2
  # ---------------------------------------------------------------------------

  describe "format_output/2" do
    @sample_info %{
      kernel_name: "Linux",
      nodename: "myhost",
      kernel_release: "5.4.0",
      kernel_version: "#1 SMP",
      machine: "x86_64",
      processor: "x86_64",
      hardware_platform: "x86_64",
      operating_system: "GNU/Linux"
    }

    test "default (no flags) shows kernel name only" do
      result = UnixTools.Uname.format_output(@sample_info, %{})
      assert result == "Linux"
    end

    test "kernel_name flag" do
      result = UnixTools.Uname.format_output(@sample_info, %{kernel_name: true})
      assert result == "Linux"
    end

    test "nodename flag" do
      result = UnixTools.Uname.format_output(@sample_info, %{nodename: true})
      assert result == "myhost"
    end

    test "machine flag" do
      result = UnixTools.Uname.format_output(@sample_info, %{machine: true})
      assert result == "x86_64"
    end

    test "multiple flags show multiple fields" do
      result = UnixTools.Uname.format_output(@sample_info,
        %{kernel_name: true, nodename: true})
      assert result == "Linux myhost"
    end

    test "all flag shows everything" do
      result = UnixTools.Uname.format_output(@sample_info, %{all: true})
      assert result == "Linux myhost 5.4.0 #1 SMP x86_64 x86_64 x86_64 GNU/Linux"
    end

    test "operating_system flag" do
      result = UnixTools.Uname.format_output(@sample_info, %{operating_system: true})
      assert result == "GNU/Linux"
    end
  end
end
