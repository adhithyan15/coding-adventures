defmodule WcTest do
  @moduledoc """
  Tests for the wc tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Counting business logic (lines, words, bytes, chars, max line length).
  3. Output formatting (right-aligned columns, totals).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "wc.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the wc spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["wc"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["wc", "file1.txt"])
      assert arguments["files"] == ["file1.txt"]
    end

    test "multiple file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["wc", "a.txt", "b.txt"])
      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-l sets lines to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "-l"])
      assert flags["lines"] == true
    end

    test "-w sets words to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "-w"])
      assert flags["words"] == true
    end

    test "-c sets bytes to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "-c"])
      assert flags["bytes"] == true
    end

    test "-m sets chars to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "-m"])
      assert flags["chars"] == true
    end

    test "-L sets max_line_length to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "-L"])
      assert flags["max_line_length"] == true
    end

    test "--lines long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "--lines"])
      assert flags["lines"] == true
    end

    test "--words long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["wc", "--words"])
      assert flags["words"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["wc", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["wc", "--help"])
      assert text =~ "wc"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["wc", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["wc", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["wc", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Counting business logic
  # ---------------------------------------------------------------------------

  describe "count_content/2" do
    test "counts lines (newline characters)" do
      counts = UnixTools.Wc.count_content("hello\nworld\n", "test.txt")
      assert counts.lines == 2
    end

    test "counts words" do
      counts = UnixTools.Wc.count_content("hello world\nfoo bar baz\n", "test.txt")
      assert counts.words == 5
    end

    test "counts bytes" do
      counts = UnixTools.Wc.count_content("hello\n", "test.txt")
      assert counts.bytes == 6
    end

    test "counts characters" do
      counts = UnixTools.Wc.count_content("hello\n", "test.txt")
      assert counts.chars == 6
    end

    test "max line length" do
      counts = UnixTools.Wc.count_content("hi\nhello world\na\n", "test.txt")
      assert counts.max_line_length == 11
    end

    test "empty content" do
      counts = UnixTools.Wc.count_content("", "test.txt")
      assert counts.lines == 0
      assert counts.words == 0
      assert counts.bytes == 0
    end

    test "single line no trailing newline" do
      counts = UnixTools.Wc.count_content("hello", "test.txt")
      assert counts.lines == 0
      assert counts.words == 1
      assert counts.bytes == 5
    end

    test "preserves filename" do
      counts = UnixTools.Wc.count_content("hello\n", "myfile.txt")
      assert counts.filename == "myfile.txt"
    end

    test "multiple spaces between words" do
      counts = UnixTools.Wc.count_content("  hello   world  \n", "test.txt")
      assert counts.words == 2
    end

    test "tabs and mixed whitespace" do
      counts = UnixTools.Wc.count_content("hello\tworld\n", "test.txt")
      assert counts.words == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Output formatting
  # ---------------------------------------------------------------------------

  describe "format_line/3" do
    test "formats with all columns" do
      counts = %{lines: 10, words: 42, bytes: 256, chars: 256, max_line_length: 80, filename: "test.txt"}
      display = %{lines: true, words: true, bytes: true, chars: false, max_line_length: false}

      result = UnixTools.Wc.format_line(counts, display, 3)
      assert result =~ " 10"
      assert result =~ " 42"
      assert result =~ "256"
      assert result =~ "test.txt"
    end

    test "formats with only lines" do
      counts = %{lines: 5, words: 10, bytes: 50, chars: 50, max_line_length: 20, filename: "test.txt"}
      display = %{lines: true, words: false, bytes: false, chars: false, max_line_length: false}

      result = UnixTools.Wc.format_line(counts, display, 2)
      assert result =~ " 5"
      assert result =~ "test.txt"
      refute result =~ "10"
      refute result =~ "50"
    end

    test "empty filename not included" do
      counts = %{lines: 5, words: 10, bytes: 50, chars: 50, max_line_length: 20, filename: ""}
      display = %{lines: true, words: true, bytes: true, chars: false, max_line_length: false}

      result = UnixTools.Wc.format_line(counts, display, 2)
      # Should not end with extra space or filename.
      refute result =~ "  \n"
    end
  end
end
