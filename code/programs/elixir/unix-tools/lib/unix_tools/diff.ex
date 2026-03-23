defmodule UnixTools.Diff do
  @moduledoc """
  diff -- compare files line by line.

  ## What This Program Does

  This is a reimplementation of the GNU `diff` utility in Elixir. It compares
  two files (or directories) and reports the differences between them.

  ## How diff Works

  At its simplest:

      diff file1.txt file2.txt   =>   shows differences between the two files

  ## Output Formats

  diff supports several output formats:

  | Format    | Flag | Description                                        |
  |-----------|------|----------------------------------------------------|
  | Normal    | -    | Default: shows change commands (a, d, c)           |
  | Unified   | -u   | Shows context with +/- prefixes (like git diff)    |
  | Context   | -c   | Shows context with ! markers                       |

  ## The LCS Algorithm

  The heart of diff is the **Longest Common Subsequence (LCS)** algorithm.
  Given two sequences, LCS finds the longest sequence of elements that appear
  in both, in the same order (but not necessarily contiguous).

  For example:
      File 1: A B C D E
      File 2: A C D F E

      LCS:    A   C D   E   (length 4)

  The lines NOT in the LCS are the differences:
      - B was deleted (in file1 but not LCS)
      - F was inserted (in file2 but not LCS)

  ## How LCS Works (Dynamic Programming)

  We build a 2D table where `dp[i][j]` = length of LCS of first i lines of
  file1 and first j lines of file2.

  The recurrence:
  - If `lines1[i] == lines2[j]`: `dp[i][j] = dp[i-1][j-1] + 1`
  - Otherwise: `dp[i][j] = max(dp[i-1][j], dp[i][j-1])`

  Then we backtrack through the table to find the actual LCS.

  ## Normalization Flags

  - `-i` (ignore-case): Compare lines case-insensitively.
  - `-b` (ignore-space-change): Collapse runs of whitespace to a single space.
  - `-w` (ignore-all-space): Remove all whitespace before comparing.
  - `-B` (ignore-blank-lines): Skip blank lines entirely.

  ## Implementation Approach

  1. `compute_lcs/3` finds the longest common subsequence using DP.
  2. `compute_edit_script/3` converts LCS into an edit script (add/delete/change).
  3. `format_normal/3`, `format_unified/4`, `format_context/4` render the output.
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

    case Parser.parse(spec_path, ["diff" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          unified: flags["unified"],
          context_format: flags["context_format"],
          ignore_case: !!flags["ignore_case"],
          ignore_space_change: !!flags["ignore_space_change"],
          ignore_all_space: !!flags["ignore_all_space"],
          ignore_blank_lines: !!flags["ignore_blank_lines"],
          brief: !!flags["brief"],
          recursive: !!flags["recursive"]
        }

        file1 = arguments["file1"]
        file2 = arguments["file2"]

        run(file1, file2, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "diff: #{err.message}")
        end)

        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Line Normalization
  # ---------------------------------------------------------------------------

  @doc """
  Normalize a line for comparison according to the given options.

  This applies the ignore flags (-i, -b, -w) to produce a "comparison key"
  for each line. The original line is preserved for output.

  ## Examples

      iex> UnixTools.Diff.normalize_line("Hello World", %{ignore_case: true})
      "hello world"

      iex> UnixTools.Diff.normalize_line("a  b  c", %{ignore_space_change: true})
      "a b c"

      iex> UnixTools.Diff.normalize_line("a b c", %{ignore_all_space: true})
      "abc"
  """
  def normalize_line(line, opts \\ %{}) do
    result = line

    result =
      if opts[:ignore_all_space] do
        String.replace(result, ~r/\s/, "")
      else
        if opts[:ignore_space_change] do
          result |> String.replace(~r/\s+/, " ") |> String.trim()
        else
          result
        end
      end

    result = if opts[:ignore_case], do: String.downcase(result), else: result

    result
  end

  @doc """
  Filter out blank lines if the ignore_blank_lines option is set.

  Returns a list of `{original_index, line}` tuples so we can track
  the original line numbers for output.
  """
  def filter_lines(lines, opts \\ %{}) do
    indexed = Enum.with_index(lines, 1)

    if opts[:ignore_blank_lines] do
      Enum.reject(indexed, fn {line, _idx} -> String.trim(line) == "" end)
    else
      indexed
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: LCS (Longest Common Subsequence)
  # ---------------------------------------------------------------------------

  @doc """
  Compute the Longest Common Subsequence of two lists of lines.

  Uses the classic dynamic programming approach. Returns a list of
  `{index_in_lines1, index_in_lines2}` pairs (0-based) representing
  matching lines.

  ## How Dynamic Programming Builds the Table

  Consider lines1 = ["A", "B", "C"] and lines2 = ["A", "C"]:

          ""  "A"  "C"
      ""  [0,  0,   0]
      "A" [0,  1,   1]
      "B" [0,  1,   1]
      "C" [0,  1,   2]

  Reading: LCS("ABC", "AC") = 2 (the subsequence "A", "C").

  ## Examples

      iex> UnixTools.Diff.compute_lcs(["a", "b", "c"], ["a", "c"], %{})
      [{0, 0}, {2, 1}]
  """
  def compute_lcs(lines1, lines2, opts) do
    n = length(lines1)
    m = length(lines2)

    arr1 = :array.from_list(lines1)
    arr2 = :array.from_list(lines2)

    # -------------------------------------------------------------------------
    # Build the DP table.
    #
    # dp is a map from {i, j} to the LCS length of lines1[0..i-1] and
    # lines2[0..j-1]. We initialize row 0 and column 0 to 0.
    # -------------------------------------------------------------------------

    dp =
      Enum.reduce(0..n, %{}, fn i, acc ->
        Enum.reduce(0..m, acc, fn j, inner_acc ->
          cond do
            i == 0 or j == 0 ->
              Map.put(inner_acc, {i, j}, 0)

            normalize_line(:array.get(i - 1, arr1), opts) ==
                normalize_line(:array.get(j - 1, arr2), opts) ->
              Map.put(inner_acc, {i, j}, Map.get(inner_acc, {i - 1, j - 1}, 0) + 1)

            true ->
              val = max(Map.get(inner_acc, {i - 1, j}, 0), Map.get(inner_acc, {i, j - 1}, 0))
              Map.put(inner_acc, {i, j}, val)
          end
        end)
      end)

    # -------------------------------------------------------------------------
    # Backtrack through the table to find the actual LCS indices.
    # -------------------------------------------------------------------------

    backtrack(dp, arr1, arr2, n, m, opts, [])
  end

  defp backtrack(_dp, _arr1, _arr2, 0, _j, _opts, acc), do: acc
  defp backtrack(_dp, _arr1, _arr2, _i, 0, _opts, acc), do: acc

  defp backtrack(dp, arr1, arr2, i, j, opts, acc) do
    if normalize_line(:array.get(i - 1, arr1), opts) ==
         normalize_line(:array.get(j - 1, arr2), opts) do
      backtrack(dp, arr1, arr2, i - 1, j - 1, opts, [{i - 1, j - 1} | acc])
    else
      if Map.get(dp, {i - 1, j}, 0) >= Map.get(dp, {i, j - 1}, 0) do
        backtrack(dp, arr1, arr2, i - 1, j, opts, acc)
      else
        backtrack(dp, arr1, arr2, i, j - 1, opts, acc)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Edit Script
  # ---------------------------------------------------------------------------

  @doc """
  Convert an LCS into an edit script.

  An edit script is a list of operations:
  - `{:equal, idx1, idx2}` — lines match (in the LCS)
  - `{:delete, idx1}` — line exists only in file1 (deleted)
  - `{:insert, idx2}` — line exists only in file2 (inserted)

  ## How It Works

  Walk through both files simultaneously. Whenever both indices match
  an LCS pair, emit `:equal`. Otherwise, emit `:delete` for lines in
  file1 that aren't matched, and `:insert` for lines in file2.

  ## Examples

      iex> lcs = [{0, 0}, {2, 1}]
      iex> UnixTools.Diff.compute_edit_script(3, 2, lcs)
      [{:equal, 0, 0}, {:delete, 1}, {:equal, 2, 1}]
  """
  def compute_edit_script(len1, len2, lcs) do
    build_edit_script(0, 0, lcs, len1, len2, [])
    |> Enum.reverse()
  end

  defp build_edit_script(i, j, [], len1, len2, acc) do
    # No more LCS pairs. Remaining lines are deletes/inserts.
    acc = Enum.reduce(i..(len1 - 1)//1, acc, fn idx, a -> [{:delete, idx} | a] end)
    Enum.reduce(j..(len2 - 1)//1, acc, fn idx, a -> [{:insert, idx} | a] end)
  end

  defp build_edit_script(i, j, [{lcs_i, lcs_j} | rest_lcs], len1, len2, acc) do
    # Emit deletes for file1 lines before the LCS match
    acc = Enum.reduce(i..(lcs_i - 1)//1, acc, fn idx, a -> [{:delete, idx} | a] end)
    # Emit inserts for file2 lines before the LCS match
    acc = Enum.reduce(j..(lcs_j - 1)//1, acc, fn idx, a -> [{:insert, idx} | a] end)
    # Emit the equal line
    acc = [{:equal, lcs_i, lcs_j} | acc]

    build_edit_script(lcs_i + 1, lcs_j + 1, rest_lcs, len1, len2, acc)
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Grouping into Hunks
  # ---------------------------------------------------------------------------

  @doc """
  Group an edit script into hunks (contiguous regions of change).

  A hunk is a group of consecutive non-equal operations, optionally surrounded
  by context lines. Two hunks that are close together (separated by fewer than
  `2 * context` equal lines) are merged into one.

  Returns a list of hunks, where each hunk is a list of edit operations
  with surrounding context.
  """
  def group_into_hunks(edit_script, context_lines \\ 3) do
    # Split into runs of changes and equals
    {hunks, current} =
      Enum.reduce(edit_script, {[], []}, fn op, {completed_hunks, current_hunk} ->
        case op do
          {:equal, _, _} ->
            {completed_hunks, current_hunk ++ [op]}

          _ ->
            {completed_hunks, current_hunk ++ [op]}
        end
      end)

    all_ops = Enum.concat(Enum.reverse(hunks), current)

    # Find change regions (indices of non-equal ops in the flat list)
    indexed_ops = Enum.with_index(all_ops)
    change_indices = for {{op_type, _, _}, idx} <- indexed_ops, op_type != :equal, do: idx
    change_indices = change_indices ++ (for {{:delete, _}, idx} <- indexed_ops, do: idx)
    change_indices = change_indices ++ (for {{:insert, _}, idx} <- indexed_ops, do: idx)
    change_indices = Enum.uniq(change_indices) |> Enum.sort()

    if change_indices == [] do
      []
    else
      # Group changes that are within 2*context of each other
      groups = group_nearby_changes(change_indices, context_lines * 2)

      Enum.map(groups, fn {first_change, last_change} ->
        # Add context before and after
        start_idx = max(0, first_change - context_lines)
        end_idx = min(length(all_ops) - 1, last_change + context_lines)

        Enum.slice(all_ops, start_idx..end_idx)
      end)
    end
  end

  defp group_nearby_changes([], _gap), do: []

  defp group_nearby_changes([first | rest], gap) do
    {groups, current_start, current_end} =
      Enum.reduce(rest, {[], first, first}, fn idx, {grps, grp_start, grp_end} ->
        if idx - grp_end <= gap do
          {grps, grp_start, idx}
        else
          {[{grp_start, grp_end} | grps], idx, idx}
        end
      end)

    Enum.reverse([{current_start, current_end} | groups])
  end

  # ---------------------------------------------------------------------------
  # Output Formatting: Normal Format
  # ---------------------------------------------------------------------------

  @doc """
  Format the edit script in normal diff format.

  Normal format uses three kinds of commands:
  - `NaM` — add lines after line N in file1 (from lines M in file2)
  - `NdM` — delete line N from file1 (would appear at line M in file2)
  - `N,McP,Q` — change lines N-M in file1 to lines P-Q in file2

  Lines from file1 are prefixed with "< ", lines from file2 with "> ".
  A "---" separator appears between deleted and added lines in a change.

  ## Examples

      Normal output for changing line 2:
          2c2
          < old line
          ---
          > new line
  """
  def format_normal(edit_script, lines1, lines2) do
    # Group consecutive changes
    chunks = chunk_changes(edit_script)

    chunks
    |> Enum.map(fn chunk ->
      deletes = for {:delete, idx} <- chunk, do: idx
      inserts = for {:insert, idx} <- chunk, do: idx

      cond do
        deletes != [] and inserts != [] ->
          # Change
          del_range = format_range(deletes)
          ins_range = format_range(inserts)
          del_lines = Enum.map(deletes, fn i -> "< #{Enum.at(lines1, i)}" end)
          ins_lines = Enum.map(inserts, fn i -> "> #{Enum.at(lines2, i)}" end)

          "#{del_range}c#{ins_range}\n" <>
            Enum.join(del_lines, "\n") <> "\n---\n" <> Enum.join(ins_lines, "\n")

        deletes != [] ->
          # Delete
          del_range = format_range(deletes)
          # The "after" position in file2 is the last equal line before this delete
          after_pos = find_corresponding_position(edit_script, List.first(deletes), :delete)
          del_lines = Enum.map(deletes, fn i -> "< #{Enum.at(lines1, i)}" end)

          "#{del_range}d#{after_pos}\n" <> Enum.join(del_lines, "\n")

        inserts != [] ->
          # Add
          ins_range = format_range(inserts)
          after_pos = find_corresponding_position(edit_script, List.first(inserts), :insert)
          ins_lines = Enum.map(inserts, fn i -> "> #{Enum.at(lines2, i)}" end)

          "#{after_pos}a#{ins_range}\n" <> Enum.join(ins_lines, "\n")
      end
    end)
    |> Enum.join("\n")
  end

  # ---------------------------------------------------------------------------
  # Output Formatting: Unified Format
  # ---------------------------------------------------------------------------

  @doc """
  Format the diff in unified format (like `diff -u` or `git diff`).

  Unified format shows context lines around changes with +/- prefixes:

      --- file1
      +++ file2
      @@ -start,count +start,count @@
       context line
      -deleted line
      +added line
       context line
  """
  def format_unified(edit_script, lines1, lines2, context \\ 3) do
    hunks = group_into_hunks(edit_script, context)

    if hunks == [] do
      ""
    else
      hunk_strs =
        Enum.map(hunks, fn hunk ->
          # Compute range for each side
          {f1_start, f1_count, f2_start, f2_count} = compute_hunk_ranges(hunk)

          header = "@@ -#{f1_start + 1},#{f1_count} +#{f2_start + 1},#{f2_count} @@"

          lines =
            Enum.map(hunk, fn
              {:equal, idx1, _idx2} -> " #{Enum.at(lines1, idx1)}"
              {:delete, idx1} -> "-#{Enum.at(lines1, idx1)}"
              {:insert, idx2} -> "+#{Enum.at(lines2, idx2)}"
            end)

          header <> "\n" <> Enum.join(lines, "\n")
        end)

      Enum.join(hunk_strs, "\n")
    end
  end

  # ---------------------------------------------------------------------------
  # Output Formatting: Context Format
  # ---------------------------------------------------------------------------

  @doc """
  Format the diff in context format (like `diff -c`).

  Context format shows the full context around changes with markers:
  - `!` for changed lines
  - `-` for deleted lines
  - `+` for added lines
  - ` ` for context lines
  """
  def format_context(edit_script, lines1, lines2, context \\ 3) do
    hunks = group_into_hunks(edit_script, context)

    if hunks == [] do
      ""
    else
      hunk_strs =
        Enum.map(hunks, fn hunk ->
          {f1_start, f1_count, f2_start, f2_count} = compute_hunk_ranges(hunk)

          separator = "***************"

          file1_header = "*** #{f1_start + 1},#{f1_start + f1_count} ****"
          file1_lines =
            hunk
            |> Enum.filter(fn
              {:insert, _} -> false
              _ -> true
            end)
            |> Enum.map(fn
              {:equal, idx1, _} -> "  #{Enum.at(lines1, idx1)}"
              {:delete, idx1} -> "- #{Enum.at(lines1, idx1)}"
            end)

          file2_header = "--- #{f2_start + 1},#{f2_start + f2_count} ----"
          file2_lines =
            hunk
            |> Enum.filter(fn
              {:delete, _} -> false
              _ -> true
            end)
            |> Enum.map(fn
              {:equal, _, idx2} -> "  #{Enum.at(lines2, idx2)}"
              {:insert, idx2} -> "+ #{Enum.at(lines2, idx2)}"
            end)

          separator <> "\n" <>
            file1_header <> "\n" <> Enum.join(file1_lines, "\n") <> "\n" <>
            file2_header <> "\n" <> Enum.join(file2_lines, "\n")
        end)

      Enum.join(hunk_strs, "\n")
    end
  end

  # ---------------------------------------------------------------------------
  # High-level diff function
  # ---------------------------------------------------------------------------

  @doc """
  Compute and format the diff between two lists of lines.

  Returns `{:identical, ""}` if files are the same, or
  `{:different, output}` with the formatted diff.

  ## Options

  - `:unified` — number of context lines for unified format (enables -u)
  - `:context_format` — number of context lines for context format (enables -c)
  - `:brief` — just report whether files differ
  - `:ignore_case`, `:ignore_space_change`, `:ignore_all_space`, `:ignore_blank_lines`
  """
  def diff_lines(lines1, lines2, opts \\ %{}) do
    lcs = compute_lcs(lines1, lines2, opts)
    edit_script = compute_edit_script(length(lines1), length(lines2), lcs)

    has_changes = Enum.any?(edit_script, fn
      {:equal, _, _} -> false
      _ -> true
    end)

    if has_changes do
      output =
        cond do
          opts[:brief] ->
            :brief

          opts[:unified] ->
            format_unified(edit_script, lines1, lines2, opts[:unified])

          opts[:context_format] ->
            format_context(edit_script, lines1, lines2, opts[:context_format])

          true ->
            format_normal(edit_script, lines1, lines2)
        end

      {:different, output}
    else
      {:identical, ""}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp chunk_changes(edit_script) do
    edit_script
    |> Enum.chunk_by(fn
      {:equal, _, _} -> :equal
      _ -> :change
    end)
    |> Enum.reject(fn chunk ->
      Enum.all?(chunk, fn
        {:equal, _, _} -> true
        _ -> false
      end)
    end)
  end

  defp format_range([single]), do: "#{single + 1}"
  defp format_range(indices), do: "#{List.first(indices) + 1},#{List.last(indices) + 1}"

  defp find_corresponding_position(edit_script, target_idx, type) do
    # Find the last equal line before the target operation
    before =
      Enum.take_while(edit_script, fn
        {:delete, idx} when type == :delete -> idx < target_idx
        {:insert, idx} when type == :insert -> idx < target_idx
        {:equal, _, _} -> true
        _ -> true
      end)

    last_equal =
      Enum.filter(before, fn
        {:equal, _, _} -> true
        _ -> false
      end)
      |> List.last()

    case {type, last_equal} do
      {:delete, nil} -> 0
      {:delete, {:equal, _, j}} -> j + 1
      {:insert, nil} -> 0
      {:insert, {:equal, i, _}} -> i + 1
    end
  end

  defp compute_hunk_ranges(hunk) do
    f1_indices =
      Enum.flat_map(hunk, fn
        {:equal, idx1, _} -> [idx1]
        {:delete, idx1} -> [idx1]
        {:insert, _} -> []
      end)

    f2_indices =
      Enum.flat_map(hunk, fn
        {:equal, _, idx2} -> [idx2]
        {:insert, idx2} -> [idx2]
        {:delete, _} -> []
      end)

    f1_start = if f1_indices == [], do: 0, else: Enum.min(f1_indices)
    f1_count = length(f1_indices)
    f2_start = if f2_indices == [], do: 0, else: Enum.min(f2_indices)
    f2_count = length(f2_indices)

    {f1_start, f1_count, f2_start, f2_count}
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(file1, file2, opts) do
    if File.dir?(file1) and File.dir?(file2) and opts[:recursive] do
      diff_directories(file1, file2, opts)
    else
      diff_files(file1, file2, opts)
    end
  end

  defp diff_files(file1, file2, opts) do
    with {:ok, content1} <- File.read(file1),
         {:ok, content2} <- File.read(file2) do
      lines1 = String.split(content1, "\n", trim: true)
      lines2 = String.split(content2, "\n", trim: true)

      case diff_lines(lines1, lines2, opts) do
        {:identical, _} ->
          :ok

        {:different, :brief} ->
          IO.puts("Files #{file1} and #{file2} differ")
          System.halt(1)

        {:different, output} ->
          if opts[:unified] do
            IO.puts("--- #{file1}")
            IO.puts("+++ #{file2}")
          end

          IO.puts(output)
          System.halt(1)
      end
    else
      {:error, reason} ->
        IO.puts(:stderr, "diff: #{:file.format_error(reason)}")
        System.halt(2)
    end
  end

  defp diff_directories(dir1, dir2, opts) do
    files1 = File.ls!(dir1) |> Enum.sort()
    files2 = File.ls!(dir2) |> Enum.sort()

    all_files = Enum.uniq(files1 ++ files2) |> Enum.sort()

    Enum.each(all_files, fn file_name ->
      path1 = Path.join(dir1, file_name)
      path2 = Path.join(dir2, file_name)

      cond do
        not File.exists?(path1) ->
          IO.puts("Only in #{dir2}: #{file_name}")

        not File.exists?(path2) ->
          IO.puts("Only in #{dir1}: #{file_name}")

        File.dir?(path1) and File.dir?(path2) ->
          diff_directories(path1, path2, opts)

        true ->
          diff_files(path1, path2, opts)
      end
    end)
  end

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "diff.json"),
        else: nil
      ),
      "diff.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "diff.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find diff.json spec file"
  end
end
