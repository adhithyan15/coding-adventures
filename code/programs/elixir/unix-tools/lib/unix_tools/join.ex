defmodule UnixTools.Join do
  @moduledoc """
  join -- join lines of two files on a common field.

  ## What This Program Does

  This is a reimplementation of the GNU `join` utility in Elixir. It reads
  two sorted files and produces output lines by joining records that share
  a common field value, similar to a database JOIN operation.

  ## How join Works

  Given two files sorted by their join field:

      File 1 (people.txt):       File 2 (ages.txt):
      alice engineer             alice 30
      bob designer               bob 25
      carol manager              carol 35

      join people.txt ages.txt
      => alice engineer 30
         bob designer 25
         carol manager 35

  ## The Merge-Join Algorithm

  join uses a merge-join algorithm, which is efficient for sorted inputs.
  It works like merging two sorted lists:

  1. Read a line from each file.
  2. Compare the join fields.
  3. If equal: output the joined line, advance both.
  4. If file1 < file2: advance file1 (unmatched line from file1).
  5. If file1 > file2: advance file2 (unmatched line from file2).

  This is O(n + m) where n and m are the line counts — much faster than
  the O(n * m) nested-loop join that would be needed for unsorted data.

  ## Join Fields

  By default, join uses field 1 (the first whitespace-delimited column)
  from both files. You can change this:

  - `-1 FIELD` — use field FIELD from file 1
  - `-2 FIELD` — use field FIELD from file 2
  - `-j FIELD` — use field FIELD from both files

  Fields are 1-indexed (field 1 is the first column).

  ## Unpairable Lines

  Lines in one file that have no matching line in the other are called
  "unpairable." By default, they are suppressed. You can include them:

  - `-a 1` — also print unpairable lines from file 1
  - `-a 2` — also print unpairable lines from file 2
  - `-v 1` — print ONLY unpairable lines from file 1
  - `-v 2` — print ONLY unpairable lines from file 2

  ## Field Separator

  By default, fields are separated by whitespace (runs of spaces/tabs).
  Use `-t CHAR` to set a specific separator:

      join -t, file1.csv file2.csv

  ## Implementation Approach

  Pure functions implement each step:

  1. `parse_line/2` splits a line into fields using the separator.
  2. `get_join_key/2` extracts the join field from a parsed line.
  3. `merge_join/4` implements the merge-join algorithm.
  4. `format_output/4` constructs output lines.
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

    case Parser.parse(spec_path, ["join" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        # Determine join fields. -j overrides both -1 and -2.
        join_field = flags["join_field"]
        field1_idx = (join_field || flags["field1"] || 1) - 1
        field2_idx = (join_field || flags["field2"] || 1) - 1

        separator = flags["separator"]
        empty_val = flags["empty"] || ""
        ignore_case = !!flags["ignore_case"]

        unpaired_raw = List.wrap(flags["unpaired"] || [])
        unpaired_files = Enum.map(unpaired_raw, &String.to_integer/1)

        only_unpaired_val = flags["only_unpaired"]

        only_unpaired =
          if only_unpaired_val, do: String.to_integer(only_unpaired_val), else: nil

        opts = %{
          field1: field1_idx,
          field2: field2_idx,
          separator: separator,
          empty: empty_val,
          ignore_case: ignore_case,
          unpaired: unpaired_files,
          only_unpaired: only_unpaired
        }

        file1_path = arguments["file1"]
        file2_path = arguments["file2"]

        lines1 = read_lines(file1_path)
        lines2 = read_lines(file2_path)

        result = merge_join(lines1, lines2, opts)

        Enum.each(result, &IO.puts/1)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "join: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Line Parsing
  # ---------------------------------------------------------------------------

  @doc """
  Parse a line into fields using the given separator.

  If no separator is specified, splits on whitespace (runs of spaces/tabs).
  If a separator is given, splits on that exact character.

  ## Examples

      iex> UnixTools.Join.parse_line("alice 30 engineer", nil)
      ["alice", "30", "engineer"]

      iex> UnixTools.Join.parse_line("alice,30,engineer", ",")
      ["alice", "30", "engineer"]

      iex> UnixTools.Join.parse_line("  spaced  out  ", nil)
      ["spaced", "out"]
  """
  def parse_line(line, nil), do: String.split(line)
  def parse_line(line, sep), do: String.split(line, sep)

  @doc """
  Extract the join key from a list of fields at the given index.

  Returns the empty string if the index is out of bounds.

  ## Examples

      iex> UnixTools.Join.get_join_key(["alice", "30", "engineer"], 0)
      "alice"

      iex> UnixTools.Join.get_join_key(["alice", "30"], 5)
      ""
  """
  def get_join_key(fields, field_idx) do
    Enum.at(fields, field_idx, "")
  end

  @doc """
  Compare two join keys, optionally ignoring case.

  Returns `:lt`, `:eq`, or `:gt`.

  ## Examples

      iex> UnixTools.Join.compare_keys("alice", "bob", false)
      :lt

      iex> UnixTools.Join.compare_keys("Alice", "alice", true)
      :eq

      iex> UnixTools.Join.compare_keys("bob", "alice", false)
      :gt
  """
  def compare_keys(key1, key2, ignore_case) do
    k1 = if ignore_case, do: String.downcase(key1), else: key1
    k2 = if ignore_case, do: String.downcase(key2), else: key2

    cond do
      k1 < k2 -> :lt
      k1 > k2 -> :gt
      true -> :eq
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Output Formatting
  # ---------------------------------------------------------------------------

  @doc """
  Format a joined output line.

  The output format is: join_key followed by the non-key fields from both
  files, separated by the output separator (default: space).

  ## How Output Fields Are Assembled

  For a join on field 1:
  - The join key comes first.
  - Then all fields from file 1 EXCEPT the join field.
  - Then all fields from file 2 EXCEPT the join field.

  ## Examples

      iex> UnixTools.Join.format_output("alice", ["alice", "30"], ["alice", "engineer"], 0, 0, nil)
      "alice 30 engineer"

      iex> UnixTools.Join.format_output("alice", ["alice", "30"], ["alice", "engineer"], 0, 0, ",")
      "alice,30,engineer"
  """
  def format_output(key, fields1, fields2, field1_idx, field2_idx, separator) do
    sep = separator || " "

    other1 = List.delete_at(fields1, field1_idx)
    other2 = List.delete_at(fields2, field2_idx)

    ([key] ++ other1 ++ other2) |> Enum.join(sep)
  end

  @doc """
  Format an unpairable line (a line from one file with no match in the other).

  ## Examples

      iex> UnixTools.Join.format_unpaired("dave", ["dave", "40"], 0, nil)
      "dave 40"
  """
  def format_unpaired(key, fields, field_idx, separator) do
    sep = separator || " "
    other = List.delete_at(fields, field_idx)
    ([key] ++ other) |> Enum.join(sep)
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Merge-Join Algorithm
  # ---------------------------------------------------------------------------

  @doc """
  Perform a merge-join on two lists of lines.

  This is the core algorithm. Both inputs must be sorted by their join
  field. The algorithm walks through both lists simultaneously:

  ## Step-by-Step Example

      File 1 lines:        File 2 lines:
      ["alice eng",        ["alice 30",
       "bob des",           "carol 35"]
       "carol mgr"]

  Iteration 1: key1="alice", key2="alice" -> EQUAL, output joined line
  Iteration 2: key1="bob", key2="carol" -> bob < carol, unpairable from file1
  Iteration 3: key1="carol", key2="carol" -> EQUAL, output joined line

  ## Return Value

  Returns a list of formatted output strings.
  """
  def merge_join(lines1, lines2, opts) do
    parsed1 = Enum.map(lines1, fn line -> parse_line(line, opts.separator) end)
    parsed2 = Enum.map(lines2, fn line -> parse_line(line, opts.separator) end)

    do_merge_join(parsed1, parsed2, opts, [])
    |> Enum.reverse()
  end

  defp do_merge_join([], [], _opts, acc), do: acc

  defp do_merge_join([], remaining2, opts, acc) do
    # File 1 is exhausted — remaining lines from file 2 are unpairable.
    if 2 in opts.unpaired or opts.only_unpaired == 2 do
      unpaired_lines =
        Enum.map(remaining2, fn fields ->
          key = get_join_key(fields, opts.field2)
          format_unpaired(key, fields, opts.field2, opts.separator)
        end)

      Enum.reverse(unpaired_lines) ++ acc
    else
      acc
    end
  end

  defp do_merge_join(remaining1, [], opts, acc) do
    # File 2 is exhausted — remaining lines from file 1 are unpairable.
    if 1 in opts.unpaired or opts.only_unpaired == 1 do
      unpaired_lines =
        Enum.map(remaining1, fn fields ->
          key = get_join_key(fields, opts.field1)
          format_unpaired(key, fields, opts.field1, opts.separator)
        end)

      Enum.reverse(unpaired_lines) ++ acc
    else
      acc
    end
  end

  defp do_merge_join([fields1 | rest1] = all1, [fields2 | rest2] = all2, opts, acc) do
    key1 = get_join_key(fields1, opts.field1)
    key2 = get_join_key(fields2, opts.field2)

    case compare_keys(key1, key2, opts.ignore_case) do
      :eq ->
        # -----------------------------------------------------------------------
        # Keys match — handle possible duplicates in both files.
        # Collect all lines from both files with the same key.
        # -----------------------------------------------------------------------

        {same_key1, after1} = collect_same_key(all1, opts.field1, key1, opts.ignore_case)
        {same_key2, after2} = collect_same_key(all2, opts.field2, key2, opts.ignore_case)

        joined_lines =
          if opts.only_unpaired == nil do
            for f1 <- same_key1, f2 <- same_key2 do
              format_output(key1, f1, f2, opts.field1, opts.field2, opts.separator)
            end
          else
            []
          end

        new_acc = Enum.reverse(joined_lines) ++ acc
        do_merge_join(after1, after2, opts, new_acc)

      :lt ->
        # File 1 key is smaller — unpairable from file 1.
        new_acc =
          if 1 in opts.unpaired or opts.only_unpaired == 1 do
            line = format_unpaired(key1, fields1, opts.field1, opts.separator)
            [line | acc]
          else
            acc
          end

        do_merge_join(rest1, all2, opts, new_acc)

      :gt ->
        # File 2 key is smaller — unpairable from file 2.
        new_acc =
          if 2 in opts.unpaired or opts.only_unpaired == 2 do
            line = format_unpaired(key2, fields2, opts.field2, opts.separator)
            [line | acc]
          else
            acc
          end

        do_merge_join(all1, rest2, opts, new_acc)
    end
  end

  @doc """
  Collect consecutive lines that share the same join key.

  This handles the case where multiple lines in a file have the same
  join field value (like a SQL table with duplicate keys).

  ## Examples

      iex> lines = [["alice", "1"], ["alice", "2"], ["bob", "3"]]
      iex> UnixTools.Join.collect_same_key(lines, 0, "alice", false)
      {[["alice", "1"], ["alice", "2"]], [["bob", "3"]]}
  """
  def collect_same_key(lines, field_idx, key, ignore_case) do
    {same, rest_lines} =
      Enum.split_while(lines, fn fields ->
        compare_keys(get_join_key(fields, field_idx), key, ignore_case) == :eq
      end)

    {same, rest_lines}
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_lines(path) do
    File.read!(path)
    |> String.trim_trailing("\n")
    |> String.split("\n")
  end

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "join.json"),
        else: nil
      ),
      "join.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "join.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find join.json spec file"
  end
end
