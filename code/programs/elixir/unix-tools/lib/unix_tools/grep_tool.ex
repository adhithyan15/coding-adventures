defmodule UnixTools.GrepTool do
  @moduledoc """
  grep -- print lines that match patterns.

  ## What This Program Does

  This is a reimplementation of the GNU `grep` utility in Elixir. It searches
  input files for lines containing a match to a given pattern and prints those
  lines.

  ## How grep Works

  At its simplest:

      grep "hello" file.txt     =>   prints lines containing "hello"
      grep -i "hello" file.txt  =>   case-insensitive search
      grep -n "hello" file.txt  =>   prints line numbers too

  ## Pattern Matching Modes

  grep supports several pattern matching modes:

  | Flag | Mode               | Description                        |
  |------|--------------------|------------------------------------|
  | -G   | Basic regex        | Default. `.` `*` `^` `$` work      |
  | -E   | Extended regex     | `+` `?` `{n}` `|` `()` also work   |
  | -F   | Fixed string       | No regex — literal string match     |
  | -P   | Perl regex         | Full PCRE (Elixir default)          |

  Since Elixir's `Regex` module uses PCRE under the hood, we treat both
  basic and extended modes as PCRE. Fixed-string mode escapes the pattern.

  ## Output Control Flags

  | Flag | Effect                                               |
  |------|------------------------------------------------------|
  | -c   | Print only a count of matching lines                 |
  | -l   | Print only filenames with matches                    |
  | -L   | Print only filenames without matches                 |
  | -o   | Print only the matched parts                         |
  | -n   | Prefix each line with its line number                |
  | -H   | Print filename with each match (default for 2+ files)|
  | -h   | Suppress filename prefix                             |
  | -q   | Quiet — no output, just exit status                  |

  ## Context Lines

  grep can show lines surrounding each match:

  - `-A NUM` — show NUM lines after each match
  - `-B NUM` — show NUM lines before each match
  - `-C NUM` — show NUM lines before and after each match

  Context lines are separated from matches in different groups by `--`.

  ## Implementation Approach

  The core logic is split into pure functions:

  1. `compile_pattern/2` builds a Regex from the pattern string and flags.
  2. `search_lines/3` finds matching lines (with line numbers) in content.
  3. `format_match/3` formats a single match for output.
  4. `apply_context/3` expands matches to include surrounding context lines.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["grep" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          ignore_case: !!flags["ignore_case"],
          invert_match: !!flags["invert_match"],
          fixed_strings: !!flags["fixed_strings"],
          word_regexp: !!flags["word_regexp"],
          line_regexp: !!flags["line_regexp"],
          count_only: !!flags["count"],
          files_with_matches: !!flags["files_with_matches"],
          files_without_match: !!flags["files_without_match"],
          only_matching: !!flags["only_matching"],
          line_number: !!flags["line_number"],
          with_filename: !!flags["with_filename"],
          no_filename: !!flags["no_filename"],
          quiet: !!flags["quiet"],
          max_count: flags["max_count"],
          after_lines: flags["after_context"] || 0,
          before_lines: flags["before_context"] || 0,
          context_lines: flags["context"] || 0
        }

        pattern_str = arguments["pattern"] || hd(List.wrap(flags["regexp"] || []))
        file_list = normalize_files(arguments["files"])

        # Show filenames by default when searching multiple files.
        show_filename =
          cond do
            opts.no_filename -> false
            opts.with_filename -> true
            length(file_list) > 1 -> true
            true -> false
          end

        opts = Map.put(opts, :show_filename, show_filename)

        case compile_pattern(pattern_str, opts) do
          {:ok, regex} ->
            any_match =
              Enum.reduce(file_list, false, fn file_path, found_any ->
                content = File.read!(file_path)
                result = search_content(content, regex, file_path, opts)
                found_any or result
              end)

            unless any_match do
              System.halt(1)
            end

          {:error, msg} ->
            IO.puts(:stderr, "grep: #{msg}")
            System.halt(2)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "grep: #{e.message}")
        end)

        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Pattern Compilation
  # ---------------------------------------------------------------------------

  @doc """
  Compile a pattern string into a Regex.

  ## How Fixed-String Mode Works

  With `-F`, the pattern is treated as a literal string — all regex
  metacharacters are escaped. This is useful for searching for strings
  that contain `.`, `*`, `[`, etc.

      compile_pattern("file.txt", %{fixed_strings: true, ...})
      => Regex matching literal "file.txt" (dot is not a wildcard)

  ## How Word-Regexp Mode Works

  With `-w`, the pattern is wrapped in `\\b...\\b` word boundaries so
  it only matches whole words:

      "cat" matches "cat" but not "category" or "concatenate"

  ## How Line-Regexp Mode Works

  With `-x`, the pattern is anchored to match the entire line:

      "hello" matches "hello" but not "hello world"

  ## Case Sensitivity

  With `-i`, the `i` (caseless) flag is added to the regex.

  ## Examples

      iex> {:ok, regex} = UnixTools.GrepTool.compile_pattern("hello", %{fixed_strings: false, word_regexp: false, line_regexp: false, ignore_case: false})
      iex> Regex.match?(regex, "hello world")
      true

      iex> {:ok, regex} = UnixTools.GrepTool.compile_pattern("hello", %{fixed_strings: false, word_regexp: true, line_regexp: false, ignore_case: false})
      iex> Regex.match?(regex, "helloworld")
      false
  """
  def compile_pattern(pattern_str, opts) do
    # -------------------------------------------------------------------------
    # Step 1: Escape if fixed-string mode.
    # -------------------------------------------------------------------------

    escaped =
      if opts.fixed_strings do
        Regex.escape(pattern_str)
      else
        pattern_str
      end

    # -------------------------------------------------------------------------
    # Step 2: Wrap with word boundaries or line anchors.
    # -------------------------------------------------------------------------

    wrapped =
      cond do
        opts.line_regexp -> "^#{escaped}$"
        opts.word_regexp -> "\\b#{escaped}\\b"
        true -> escaped
      end

    # -------------------------------------------------------------------------
    # Step 3: Build regex options string.
    # -------------------------------------------------------------------------

    regex_opts = if opts.ignore_case, do: "i", else: ""

    case Regex.compile(wrapped, regex_opts) do
      {:ok, regex} -> {:ok, regex}
      {:error, {msg, _pos}} -> {:error, "invalid pattern: #{msg}"}
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Searching
  # ---------------------------------------------------------------------------

  @doc """
  Search lines of text for matches against a regex.

  Returns a list of `{line_number, line_text}` tuples for matching lines.
  If `invert_match` is true, returns non-matching lines instead.
  If `max_count` is set, stops after that many matches.

  ## Examples

      iex> {:ok, regex} = Regex.compile("hello")
      iex> lines = ["hello world", "goodbye", "hello again"]
      iex> UnixTools.GrepTool.search_lines(lines, regex, %{invert_match: false, max_count: nil})
      [{1, "hello world"}, {3, "hello again"}]
  """
  def search_lines(lines, regex, opts) do
    matches =
      lines
      |> Enum.with_index(1)
      |> Enum.filter(fn {line_text, _idx} ->
        matched = Regex.match?(regex, line_text)

        if opts.invert_match, do: not matched, else: matched
      end)
      |> Enum.map(fn {line_text, idx} -> {idx, line_text} end)

    case opts.max_count do
      nil -> matches
      n when is_integer(n) -> Enum.take(matches, n)
      _ -> matches
    end
  end

  @doc """
  Apply context (before/after) to a list of match positions.

  Given matched line numbers and the total list of lines, expands the
  result to include surrounding context lines. Returns a list of
  `{line_number, line_text, :match | :context}` tuples.

  ## How Context Works

  For `-B 2 -A 1` with a match at line 5 in a 10-line file:

      Line 3:  context (before)
      Line 4:  context (before)
      Line 5:  MATCH
      Line 6:  context (after)

  When multiple match groups overlap, they are merged into one block.
  Groups that don't overlap are separated by a `--` separator.

  ## Examples

      iex> all_lines = ["a", "b", "c", "d", "e"]
      iex> matches = [{3, "c"}]
      iex> UnixTools.GrepTool.apply_context(matches, all_lines, 1, 1)
      [{2, "b", :context}, {3, "c", :match}, {4, "d", :context}]
  """
  def apply_context(matches, _all_lines, before_count, after_count)
      when before_count == 0 and after_count == 0 do
    Enum.map(matches, fn {line_num, line_text} -> {line_num, line_text, :match} end)
  end

  def apply_context(matches, all_lines, before_count, after_count) do
    total_lines = length(all_lines)
    match_set = MapSet.new(matches, fn {line_num, _} -> line_num end)

    # -------------------------------------------------------------------------
    # For each match, compute the range of lines to include.
    # -------------------------------------------------------------------------

    ranges =
      Enum.map(matches, fn {line_num, _} ->
        range_start = max(1, line_num - before_count)
        range_end = min(total_lines, line_num + after_count)
        range_start..range_end
      end)

    # -------------------------------------------------------------------------
    # Merge overlapping ranges.
    # -------------------------------------------------------------------------

    merged_ranges = merge_ranges(ranges)

    # -------------------------------------------------------------------------
    # Build the output list from merged ranges.
    # -------------------------------------------------------------------------

    Enum.flat_map(merged_ranges, fn range_val ->
      Enum.map(range_val, fn line_num ->
        line_text = Enum.at(all_lines, line_num - 1)
        tag = if MapSet.member?(match_set, line_num), do: :match, else: :context
        {line_num, line_text, tag}
      end)
    end)
  end

  @doc """
  Merge overlapping or adjacent ranges into consolidated ranges.

  ## Examples

      iex> UnixTools.GrepTool.merge_ranges([1..3, 2..5, 7..9])
      [1..5, 7..9]

      iex> UnixTools.GrepTool.merge_ranges([1..3, 4..6])
      [1..6]
  """
  def merge_ranges([]), do: []

  def merge_ranges(ranges) do
    sorted = Enum.sort_by(ranges, fn first.._//_step -> first end)

    Enum.reduce(sorted, [], fn curr_start..curr_end//_step, acc ->
      case acc do
        [] ->
          [curr_start..curr_end]

        [prev_start..prev_end//_s | rest_ranges] ->
          if curr_start <= prev_end + 1 do
            [prev_start..max(prev_end, curr_end) | rest_ranges]
          else
            [curr_start..curr_end | acc]
          end
      end
    end)
    |> Enum.reverse()
  end

  @doc """
  Format a single match line for output.

  Handles:
  - Filename prefix (`-H` / multiple files)
  - Line number prefix (`-n`)
  - Only-matching mode (`-o`)
  - Context separator (`:` for matches, `-` for context)
  """
  def format_match(line_text, line_num, regex, file_path, tag, opts) do
    separator = if tag == :match, do: ":", else: "-"

    prefix_parts = []

    prefix_parts =
      if opts.show_filename do
        prefix_parts ++ [file_path]
      else
        prefix_parts
      end

    prefix_parts =
      if opts.line_number do
        prefix_parts ++ ["#{line_num}"]
      else
        prefix_parts
      end

    prefix =
      if length(prefix_parts) > 0 do
        Enum.join(prefix_parts, separator) <> separator
      else
        ""
      end

    if opts.only_matching and tag == :match do
      # In only-matching mode, print just the matched substrings.
      Regex.scan(regex, line_text)
      |> Enum.map(fn [matched_text | _] -> "#{prefix}#{matched_text}" end)
      |> Enum.join("\n")
    else
      "#{prefix}#{line_text}"
    end
  end

  # ---------------------------------------------------------------------------
  # Orchestration
  # ---------------------------------------------------------------------------

  defp search_content(content, regex, file_path, opts) do
    lines = String.split(content, "\n", trim: false)
    # Remove trailing empty line from split
    lines = if List.last(lines) == "", do: Enum.slice(lines, 0, length(lines) - 1), else: lines

    matches = search_lines(lines, regex, opts)

    cond do
      opts.quiet ->
        length(matches) > 0

      opts.count_only ->
        if opts.show_filename do
          IO.puts("#{file_path}:#{length(matches)}")
        else
          IO.puts("#{length(matches)}")
        end

        length(matches) > 0

      opts.files_with_matches ->
        if length(matches) > 0 do
          IO.puts(file_path)
          true
        else
          false
        end

      opts.files_without_match ->
        if length(matches) == 0 do
          IO.puts(file_path)
          true
        else
          false
        end

      true ->
        # Determine context window sizes.
        before_ctx =
          if opts.context_lines > 0, do: opts.context_lines, else: opts.before_lines

        after_ctx =
          if opts.context_lines > 0, do: opts.context_lines, else: opts.after_lines

        display_lines = apply_context(matches, lines, before_ctx, after_ctx)

        Enum.each(display_lines, fn {line_num, line_text, tag} ->
          formatted = format_match(line_text, line_num, regex, file_path, tag, opts)
          IO.puts(formatted)
        end)

        length(matches) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_files(nil), do: []
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "grep.json"),
        else: nil
      ),
      "grep.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "grep.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find grep.json spec file"
  end
end
