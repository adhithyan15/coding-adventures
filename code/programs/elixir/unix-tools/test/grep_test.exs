defmodule GrepTest do
  @moduledoc """
  Tests for the grep tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-i, -v, -n, -c, -l, -L, -o, -w, -x, -F, -A, -B, -C).
  3. Business logic (compile_pattern, search_lines, apply_context,
     merge_ranges, format_match).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "grep.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the grep spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "pattern argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["grep", "hello", "file.txt"])

      assert arguments["pattern"] == "hello"
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["grep", "hello", "a.txt", "b.txt"])

      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-i sets ignore_case to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-i", "hello", "file.txt"])

      assert flags["ignore_case"] == true
    end

    test "-v sets invert_match to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-v", "hello", "file.txt"])

      assert flags["invert_match"] == true
    end

    test "-n sets line_number to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-n", "hello", "file.txt"])

      assert flags["line_number"] == true
    end

    test "-c sets count to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-c", "hello", "file.txt"])

      assert flags["count"] == true
    end

    test "-l sets files_with_matches to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-l", "hello", "file.txt"])

      assert flags["files_with_matches"] == true
    end

    test "-o sets only_matching to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-o", "hello", "file.txt"])

      assert flags["only_matching"] == true
    end

    test "-F sets fixed_strings to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-F", "hello", "file.txt"])

      assert flags["fixed_strings"] == true
    end

    test "-w sets word_regexp to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-w", "hello", "file.txt"])

      assert flags["word_regexp"] == true
    end

    test "-A sets after_context" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-A", "3", "hello", "file.txt"])

      assert flags["after_context"] == 3
    end

    test "-B sets before_context" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-B", "2", "hello", "file.txt"])

      assert flags["before_context"] == 2
    end

    test "-C sets context" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-C", "1", "hello", "file.txt"])

      assert flags["context"] == 1
    end

    test "-m sets max_count" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["grep", "-m", "5", "hello", "file.txt"])

      assert flags["max_count"] == 5
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["grep", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["grep", "--help"])
      assert text =~ "grep"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["grep", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["grep", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["grep", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - compile_pattern
  # ---------------------------------------------------------------------------

  describe "compile_pattern/2" do
    test "compiles a basic regex pattern" do
      opts = %{fixed_strings: false, word_regexp: false, line_regexp: false, ignore_case: false}
      {:ok, regex} = UnixTools.GrepTool.compile_pattern("hello", opts)
      assert Regex.match?(regex, "hello world")
      refute Regex.match?(regex, "goodbye world")
    end

    test "case-insensitive matching" do
      opts = %{fixed_strings: false, word_regexp: false, line_regexp: false, ignore_case: true}
      {:ok, regex} = UnixTools.GrepTool.compile_pattern("hello", opts)
      assert Regex.match?(regex, "Hello World")
      assert Regex.match?(regex, "HELLO")
    end

    test "fixed-string mode escapes regex metacharacters" do
      opts = %{fixed_strings: true, word_regexp: false, line_regexp: false, ignore_case: false}
      {:ok, regex} = UnixTools.GrepTool.compile_pattern("file.txt", opts)
      assert Regex.match?(regex, "file.txt")
      refute Regex.match?(regex, "fileatxt")
    end

    test "word-regexp mode matches whole words only" do
      opts = %{fixed_strings: false, word_regexp: true, line_regexp: false, ignore_case: false}
      {:ok, regex} = UnixTools.GrepTool.compile_pattern("cat", opts)
      assert Regex.match?(regex, "the cat sat")
      refute Regex.match?(regex, "concatenate")
    end

    test "line-regexp mode matches entire lines only" do
      opts = %{fixed_strings: false, word_regexp: false, line_regexp: true, ignore_case: false}
      {:ok, regex} = UnixTools.GrepTool.compile_pattern("hello", opts)
      assert Regex.match?(regex, "hello")
      refute Regex.match?(regex, "hello world")
    end

    test "returns error for invalid regex" do
      opts = %{fixed_strings: false, word_regexp: false, line_regexp: false, ignore_case: false}
      assert {:error, _msg} = UnixTools.GrepTool.compile_pattern("[invalid", opts)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - search_lines
  # ---------------------------------------------------------------------------

  describe "search_lines/3" do
    test "finds matching lines" do
      {:ok, regex} = Regex.compile("hello")
      lines = ["hello world", "goodbye", "hello again"]
      opts = %{invert_match: false, max_count: nil}

      result = UnixTools.GrepTool.search_lines(lines, regex, opts)
      assert result == [{1, "hello world"}, {3, "hello again"}]
    end

    test "invert match returns non-matching lines" do
      {:ok, regex} = Regex.compile("hello")
      lines = ["hello world", "goodbye", "hello again"]
      opts = %{invert_match: true, max_count: nil}

      result = UnixTools.GrepTool.search_lines(lines, regex, opts)
      assert result == [{2, "goodbye"}]
    end

    test "max_count limits results" do
      {:ok, regex} = Regex.compile("line")
      lines = ["line 1", "line 2", "line 3", "line 4"]
      opts = %{invert_match: false, max_count: 2}

      result = UnixTools.GrepTool.search_lines(lines, regex, opts)
      assert length(result) == 2
      assert result == [{1, "line 1"}, {2, "line 2"}]
    end

    test "no matches returns empty list" do
      {:ok, regex} = Regex.compile("missing")
      lines = ["hello", "world"]
      opts = %{invert_match: false, max_count: nil}

      result = UnixTools.GrepTool.search_lines(lines, regex, opts)
      assert result == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - apply_context
  # ---------------------------------------------------------------------------

  describe "apply_context/4" do
    test "no context returns matches only" do
      matches = [{3, "match line"}]
      all_lines = ["a", "b", "match line", "d", "e"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 0, 0)
      assert result == [{3, "match line", :match}]
    end

    test "before context includes preceding lines" do
      matches = [{3, "match line"}]
      all_lines = ["a", "b", "match line", "d", "e"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 1, 0)

      assert result == [
               {2, "b", :context},
               {3, "match line", :match}
             ]
    end

    test "after context includes following lines" do
      matches = [{3, "match line"}]
      all_lines = ["a", "b", "match line", "d", "e"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 0, 1)

      assert result == [
               {3, "match line", :match},
               {4, "d", :context}
             ]
    end

    test "both before and after context" do
      matches = [{3, "match line"}]
      all_lines = ["a", "b", "match line", "d", "e"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 1, 1)

      assert result == [
               {2, "b", :context},
               {3, "match line", :match},
               {4, "d", :context}
             ]
    end

    test "context does not go out of bounds" do
      matches = [{1, "first line"}]
      all_lines = ["first line", "second"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 5, 5)

      assert result == [
               {1, "first line", :match},
               {2, "second", :context}
             ]
    end

    test "overlapping context ranges are merged" do
      matches = [{2, "b"}, {4, "d"}]
      all_lines = ["a", "b", "c", "d", "e"]

      result = UnixTools.GrepTool.apply_context(matches, all_lines, 1, 1)

      assert result == [
               {1, "a", :context},
               {2, "b", :match},
               {3, "c", :context},
               {4, "d", :match},
               {5, "e", :context}
             ]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - merge_ranges
  # ---------------------------------------------------------------------------

  describe "merge_ranges/1" do
    test "empty list returns empty" do
      assert UnixTools.GrepTool.merge_ranges([]) == []
    end

    test "non-overlapping ranges stay separate" do
      assert UnixTools.GrepTool.merge_ranges([1..3, 7..9]) == [1..3, 7..9]
    end

    test "overlapping ranges are merged" do
      assert UnixTools.GrepTool.merge_ranges([1..3, 2..5, 7..9]) == [1..5, 7..9]
    end

    test "adjacent ranges are merged" do
      assert UnixTools.GrepTool.merge_ranges([1..3, 4..6]) == [1..6]
    end

    test "single range returns as-is" do
      assert UnixTools.GrepTool.merge_ranges([5..10]) == [5..10]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_match
  # ---------------------------------------------------------------------------

  describe "format_match/6" do
    test "simple match without decorations" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: false, line_number: false, only_matching: false}

      result = UnixTools.GrepTool.format_match("hello world", 1, regex, "file.txt", :match, opts)
      assert result == "hello world"
    end

    test "match with line number" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: false, line_number: true, only_matching: false}

      result = UnixTools.GrepTool.format_match("hello world", 42, regex, "file.txt", :match, opts)
      assert result == "42:hello world"
    end

    test "match with filename" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: true, line_number: false, only_matching: false}

      result = UnixTools.GrepTool.format_match("hello world", 1, regex, "file.txt", :match, opts)
      assert result == "file.txt:hello world"
    end

    test "match with filename and line number" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: true, line_number: true, only_matching: false}

      result = UnixTools.GrepTool.format_match("hello world", 5, regex, "file.txt", :match, opts)
      assert result == "file.txt:5:hello world"
    end

    test "context line uses dash separator" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: true, line_number: true, only_matching: false}

      result = UnixTools.GrepTool.format_match("context line", 3, regex, "file.txt", :context, opts)
      assert result == "file.txt-3-context line"
    end

    test "only matching mode shows matched parts" do
      {:ok, regex} = Regex.compile("hello")
      opts = %{show_filename: false, line_number: false, only_matching: true}

      result =
        UnixTools.GrepTool.format_match("say hello to hello", 1, regex, "file.txt", :match, opts)

      assert result == "hello\nhello"
    end
  end
end
