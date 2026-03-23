defmodule UnixTools.Comm do
  @moduledoc """
  comm -- compare two sorted files line by line.

  ## What This Program Does

  This is a reimplementation of the GNU `comm` utility in Elixir. It reads
  two sorted files and produces three-column output:

  - **Column 1**: Lines unique to file 1.
  - **Column 2**: Lines unique to file 2.
  - **Column 3**: Lines common to both files.

  ## How comm Works

  Given two sorted files:

      file1:    file2:
      apple     banana
      banana    cherry
      cherry    date

  The output of `comm file1 file2` is:

      apple
              banana
              cherry
                      (no common-only lines shown with tabs)

  Wait, let's be precise. Both files share "banana" and "cherry":

      apple                    (only in file1)
      \t\tbanana               (in both)
      \t\tcherry               (in both)
      \tdate                   (only in file2)

  ## Column Suppression

  - `-1`: Suppress column 1 (lines unique to file 1).
  - `-2`: Suppress column 2 (lines unique to file 2).
  - `-3`: Suppress column 3 (lines common to both).

  These can be combined: `comm -12 file1 file2` shows only lines in both files.

  ## The Merge Algorithm

  Since both inputs are sorted, we use a merge-style comparison:

  1. Compare the current line from each file.
  2. If file1's line < file2's line: it's unique to file1 (column 1).
  3. If file1's line > file2's line: it's unique to file2 (column 2).
  4. If equal: it's common (column 3).
  5. Advance the pointer(s) accordingly.

  This is O(n + m) where n and m are the line counts of each file.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Compare two sorted lists of lines and produce three-column output.

  Returns a list of `{column, line}` tuples where column is 1, 2, or 3.

  ## Options

  - `:suppress` - A MapSet of column numbers to suppress (e.g., MapSet.new([1, 2])).

  ## How the Algorithm Works

  We walk through both lists simultaneously, comparing the current head
  of each. This is the classic "merge" step from merge sort, but instead
  of merging into one list, we categorize each element.

  ## Examples

      iex> UnixTools.Comm.compare_sorted(["a", "b", "c"], ["b", "c", "d"])
      [{1, "a"}, {3, "b"}, {3, "c"}, {2, "d"}]
  """
  def compare_sorted(lines1, lines2, opts \\ %{}) do
    suppress = opts[:suppress] || MapSet.new()
    result = do_compare(lines1, lines2, [])

    result
    |> Enum.filter(fn {col, _line} -> not MapSet.member?(suppress, col) end)
  end

  @doc false
  defp do_compare([], [], acc), do: Enum.reverse(acc)

  defp do_compare([h1 | t1], [], acc) do
    do_compare(t1, [], [{1, h1} | acc])
  end

  defp do_compare([], [h2 | t2], acc) do
    do_compare([], t2, [{2, h2} | acc])
  end

  defp do_compare([h1 | t1] = _list1, [h2 | t2] = _list2, acc) do
    cond do
      h1 < h2 -> do_compare(t1, [h2 | t2], [{1, h1} | acc])
      h1 > h2 -> do_compare([h1 | t1], t2, [{2, h2} | acc])
      true -> do_compare(t1, t2, [{3, h1} | acc])
    end
  end

  @doc """
  Format a comparison result into the traditional comm output format.

  Each line is prefixed with the appropriate number of TAB characters:
  - Column 1: no prefix.
  - Column 2: one TAB prefix.
  - Column 3: two TAB prefixes.

  When columns are suppressed, the TAB prefixes adjust accordingly.

  ## How Tab Prefixing Works

  The number of tabs before a column depends on how many lower-numbered
  columns are NOT suppressed. For example:
  - If columns 1 and 2 are both shown: column 3 gets 2 tabs.
  - If column 1 is suppressed: column 2 gets 0 tabs, column 3 gets 1 tab.

  ## Examples

      iex> UnixTools.Comm.format_output([{1, "a"}, {3, "b"}], MapSet.new())
      ["a", "\\t\\tb"]
  """
  def format_output(results, suppress) do
    Enum.map(results, fn {col, line} ->
      # Count how many unsuppressed columns come before this one.
      # For column 1, there are no columns before it, so 0 tabs.
      # For column 2, count unsuppressed among [1].
      # For column 3, count unsuppressed among [1, 2].
      tabs_before =
        if col <= 1 do
          0
        else
          Enum.count(1..(col - 1), fn c ->
            not MapSet.member?(suppress, c)
          end)
        end

      String.duplicate("\t", tabs_before) <> line
    end)
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["comm" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        suppress = MapSet.new()
        suppress = if flags["suppress_col1"], do: MapSet.put(suppress, 1), else: suppress
        suppress = if flags["suppress_col2"], do: MapSet.put(suppress, 2), else: suppress
        suppress = if flags["suppress_col3"], do: MapSet.put(suppress, 3), else: suppress

        file1 = arguments["file1"]
        file2 = arguments["file2"]

        lines1 = read_lines(file1)
        lines2 = read_lines(file2)

        results = compare_sorted(lines1, lines2, %{suppress: suppress})
        formatted = format_output(results, suppress)

        Enum.each(formatted, &IO.puts/1)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "comm: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp read_lines("-") do
    case IO.read(:stdio, :eof) do
      {:error, _} -> []
      :eof -> []
      data -> String.split(data, "\n", trim: true)
    end
  end

  defp read_lines(file_path) do
    case File.read(file_path) do
      {:ok, content} -> String.split(content, "\n", trim: true)
      {:error, reason} ->
        IO.puts(:stderr, "comm: #{file_path}: #{:file.format_error(reason)}")
        []
    end
  end

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "comm.json"),
        else: nil
      ),
      "comm.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "comm.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find comm.json spec file"
  end
end
