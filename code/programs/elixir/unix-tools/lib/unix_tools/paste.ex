defmodule UnixTools.Paste do
  @moduledoc """
  paste -- merge lines of files.

  ## What This Program Does

  This is a reimplementation of the GNU `paste` utility in Elixir. It merges
  corresponding lines from multiple files side by side, separated by a
  delimiter (default: TAB).

  ## How paste Works

  Given two files:

      file1:    file2:
      A         1
      B         2
      C         3

  The output of `paste file1 file2` is:

      A\t1
      B\t2
      C\t3

  ## Modes

  - **Parallel** (default): Merge corresponding lines from all files.
    If one file is shorter, its columns become empty.

  - **Serial** (-s): Paste all lines of each file onto a single line,
    separated by delimiters. Each file produces one output line.

  ## Delimiter Cycling

  The `-d` flag specifies a list of delimiters that are cycled through.
  For example, `-d ','` uses commas, and `-d ',:'` alternates between
  comma and colon.

  ## Examples

      paste file1 file2           =>   lines merged with TABs
      paste -d, file1 file2       =>   lines merged with commas
      paste -s file1              =>   all lines of file1 on one line
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Merge multiple lists of lines in parallel mode.

  Takes a list of "columns" (each column is a list of lines from one file)
  and a list of delimiter characters. Produces output lines by zipping
  corresponding lines from each column.

  ## How Parallel Merge Works

  1. Find the maximum number of lines across all columns.
  2. For each line index, pick the corresponding line from each column
     (or "" if that column is shorter).
  3. Join the values with the cycling delimiter.

  ## Delimiter Cycling

  If delimiters = ["," , ":"], then between column 1-2 we use ",",
  between column 2-3 we use ":", between column 3-4 we use "," again, etc.

  ## Examples

      iex> UnixTools.Paste.paste_parallel([["a", "b"], ["1", "2"]], ["\\t"])
      ["a\\t1", "b\\t2"]
  """
  def paste_parallel(columns, delimiters \\ ["\t"]) do
    delimiters = if delimiters == [], do: ["\t"], else: delimiters
    max_lines = columns |> Enum.map(&length/1) |> Enum.max(fn -> 0 end)

    if max_lines == 0 do
      []
    else
      Enum.map(0..(max_lines - 1), fn line_idx ->
        values =
          Enum.map(columns, fn col ->
            Enum.at(col, line_idx, "")
          end)

        join_with_cycling_delimiters(values, delimiters)
      end)
    end
  end

  @doc """
  Merge lines in serial mode.

  Each column (file) produces one output line, with all its lines joined
  by the cycling delimiter.

  ## How Serial Merge Works

  For each file's lines, join them all into a single string using the
  delimiter list (cycling through it for each join point).

  ## Examples

      iex> UnixTools.Paste.paste_serial([["a", "b", "c"]], ["\\t"])
      ["a\\tb\\tc"]
  """
  def paste_serial(columns, delimiters \\ ["\t"]) do
    delimiters = if delimiters == [], do: ["\t"], else: delimiters

    Enum.map(columns, fn col ->
      join_with_cycling_delimiters(col, delimiters)
    end)
  end

  @doc """
  Parse a delimiter string into a list of individual delimiter characters.

  Handles escape sequences like `\\n` (newline), `\\t` (tab), `\\\\` (backslash),
  and `\\0` (empty string / no delimiter).

  ## Examples

      iex> UnixTools.Paste.parse_delimiters(",")
      [","]

      iex> UnixTools.Paste.parse_delimiters(",:")
      [",", ":"]
  """
  def parse_delimiters(nil), do: ["\t"]
  def parse_delimiters(""), do: ["\t"]

  def parse_delimiters(str) do
    parse_delim_chars(String.to_charlist(str), [])
  end

  defp parse_delim_chars([], acc), do: Enum.reverse(acc)

  defp parse_delim_chars([?\\ | rest_chars], acc) do
    case rest_chars do
      [?n | tail] -> parse_delim_chars(tail, ["\n" | acc])
      [?t | tail] -> parse_delim_chars(tail, ["\t" | acc])
      [?\\ | tail] -> parse_delim_chars(tail, ["\\" | acc])
      [?0 | tail] -> parse_delim_chars(tail, ["" | acc])
      [ch | tail] -> parse_delim_chars(tail, [<<ch>> | acc])
      [] -> Enum.reverse(["\\" | acc])
    end
  end

  defp parse_delim_chars([ch | tail], acc) do
    parse_delim_chars(tail, [<<ch>> | acc])
  end

  # ---------------------------------------------------------------------------
  # Helper: Join with cycling delimiters
  # ---------------------------------------------------------------------------

  @doc false
  defp join_with_cycling_delimiters([], _delimiters), do: ""
  defp join_with_cycling_delimiters([single], _delimiters), do: single

  defp join_with_cycling_delimiters(values, delimiters) do
    # Between N values there are N-1 joins. Cycle through delimiters.
    {result, _} =
      values
      |> Enum.with_index()
      |> Enum.reduce({"", 0}, fn {value, idx}, {acc, _delim_idx} ->
        if idx == 0 do
          {value, 0}
        else
          delim = Enum.at(delimiters, rem(idx - 1, length(delimiters)))
          {acc <> delim <> value, idx}
        end
      end)

    result
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["paste" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        serial = !!flags["serial"]
        delimiters = parse_delimiters(flags["delimiters"])

        file_list = normalize_files(arguments["files"])
        columns = Enum.map(file_list, &read_lines/1)

        output_lines =
          if serial do
            paste_serial(columns, delimiters)
          else
            paste_parallel(columns, delimiters)
          end

        Enum.each(output_lines, &IO.puts/1)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "paste: #{e.message}")
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
        IO.puts(:stderr, "paste: #{file_path}: #{:file.format_error(reason)}")
        []
    end
  end

  @doc false
  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "paste.json"),
        else: nil
      ),
      "paste.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "paste.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find paste.json spec file"
  end
end
