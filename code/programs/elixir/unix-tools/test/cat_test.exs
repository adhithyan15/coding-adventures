defmodule CatTest do
  @moduledoc """
  Tests for the cat tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Content processing (line numbering, squeeze blank, show tabs/ends).
  3. Non-printing character display.
  4. File argument handling.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "cat.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the cat spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["cat"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["cat", "file1.txt", "file2.txt"])
      assert arguments["files"] == ["file1.txt", "file2.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-n sets number to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-n"])
      assert flags["number"] == true
    end

    test "-b sets number_nonblank to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-b"])
      assert flags["number_nonblank"] == true
    end

    test "-s sets squeeze_blank to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-s"])
      assert flags["squeeze_blank"] == true
    end

    test "-T sets show_tabs to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-T"])
      assert flags["show_tabs"] == true
    end

    test "-E sets show_ends to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-E"])
      assert flags["show_ends"] == true
    end

    test "-A sets show_all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "-A"])
      assert flags["show_all"] == true
    end

    test "--number long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cat", "--number"])
      assert flags["number"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["cat", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["cat", "--help"])
      assert text =~ "cat"
    end

    test "help text mentions concatenate" do
      {:ok, %HelpResult{text: text}} = parse_argv(["cat", "--help"])
      assert String.downcase(text) =~ "concatenat"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["cat", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["cat", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["cat", "--unknown"])
    end

    test "error contains meaningful message" do
      {:error, %ParseErrors{errors: errors}} = parse_argv(["cat", "--unknown"])
      assert length(errors) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Content processing business logic
  # ---------------------------------------------------------------------------

  describe "process_content/3" do
    test "basic content is processed (returns updated line number)" do
      # Capture stdout to verify output.
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\nworld\n", %{
            number: false,
            number_nonblank: false,
            squeeze_blank: false,
            show_tabs: false,
            show_ends: false,
            show_nonprinting: false
          }, 1)
        end)

      assert output == "hello\nworld\n"
    end

    test "line numbering with -n" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\nworld\n", %{
            number: true,
            number_nonblank: false,
            squeeze_blank: false,
            show_tabs: false,
            show_ends: false,
            show_nonprinting: false
          }, 1)
        end)

      assert output =~ "1\thello"
      assert output =~ "2\tworld"
    end

    test "non-blank numbering with -b skips blank lines" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\n\nworld\n", %{
            number: false,
            number_nonblank: true,
            squeeze_blank: false,
            show_tabs: false,
            show_ends: false,
            show_nonprinting: false
          }, 1)
        end)

      # Line 1 should be numbered, blank line not numbered, line 2 numbered as 2.
      assert output =~ "1\thello"
      assert output =~ "2\tworld"
      # The blank line should appear but without a number.
      lines = String.split(output, "\n", trim: true)
      blank_line = Enum.at(lines, 1)
      refute blank_line =~ ~r/^\s+\d/
    end

    test "squeeze blank lines with -s" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\n\n\n\nworld\n", %{
            number: false,
            number_nonblank: false,
            squeeze_blank: true,
            show_tabs: false,
            show_ends: false,
            show_nonprinting: false
          }, 1)
        end)

      # Multiple blank lines should be squeezed to one.
      assert output == "hello\n\nworld\n"
    end

    test "show tabs with -T" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\tworld\n", %{
            number: false,
            number_nonblank: false,
            squeeze_blank: false,
            show_tabs: true,
            show_ends: false,
            show_nonprinting: false
          }, 1)
        end)

      assert output =~ "^I"
      refute output =~ "\t"
    end

    test "show ends with -E" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Cat.process_content("hello\nworld\n", %{
            number: false,
            number_nonblank: false,
            squeeze_blank: false,
            show_tabs: false,
            show_ends: true,
            show_nonprinting: false
          }, 1)
        end)

      assert output =~ "hello$"
      assert output =~ "world$"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Non-printing character display
  # ---------------------------------------------------------------------------

  describe "show_nonprinting/1" do
    test "control character becomes caret notation" do
      # ASCII 1 (SOH) should become ^A
      assert UnixTools.Cat.show_nonprinting(<<1>>) == "^A"
    end

    test "DEL becomes ^?" do
      assert UnixTools.Cat.show_nonprinting(<<127>>) == "^?"
    end

    test "tab is preserved" do
      assert UnixTools.Cat.show_nonprinting("\t") == "\t"
    end

    test "regular text is unchanged" do
      assert UnixTools.Cat.show_nonprinting("hello") == "hello"
    end
  end
end
