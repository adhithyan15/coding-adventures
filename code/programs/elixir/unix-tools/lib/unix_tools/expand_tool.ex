defmodule UnixTools.ExpandTool do
  @moduledoc """
  expand -- convert tabs to spaces.

  ## What This Program Does

  This is a reimplementation of the GNU `expand` utility in Elixir. It
  converts tab characters in files to the appropriate number of spaces,
  maintaining proper column alignment.

  ## How expand Works

  Tabs in text files are not a fixed number of spaces. Instead, they advance
  the cursor to the next **tab stop**. By default, tab stops are every 8
  columns.

  A tab at column 3 advances to column 8 (5 spaces).
  A tab at column 7 advances to column 8 (1 space).

  ## The -t Flag (Tab Stops)

  Changes the tab stop interval:

      expand -t 4 file.txt      =>   tabs every 4 columns
      expand -t 2,6,10 file.txt =>   variable tab stops

  ## The -i Flag (Initial Only)

  With `-i`, only leading tabs (before any non-blank character) are expanded.
  Tabs after non-blank characters are left as-is.
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

    case Parser.parse(spec_path, ["expand" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        initial_only = !!flags["initial"]
        tabs_str = flags["tabs"]
        tab_stops = if tabs_str, do: parse_tab_stops(tabs_str), else: 8

        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          case read_file(file) do
            {:ok, content} ->
              process_content(content, tab_stops, initial_only)

            {:error, reason} ->
              IO.puts(:stderr, "expand: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "expand: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Tab expansion.
  # ---------------------------------------------------------------------------

  @doc """
  Parse a tab stops specification.

  Returns either a single integer (uniform stops) or a list of integers
  (variable stops).
  """
  def parse_tab_stops(tabs_str) do
    if String.contains?(tabs_str, ",") do
      tabs_str
      |> String.split(",")
      |> Enum.map(&String.trim/1)
      |> Enum.map(&String.to_integer/1)
    else
      String.to_integer(tabs_str)
    end
  end

  @doc """
  Calculate the number of spaces needed to reach the next tab stop.

  For uniform tab stops, this is: tab_size - rem(column, tab_size).
  For variable stops, find the smallest stop greater than the current column.
  """
  def spaces_to_next_tab(column, tab_stops) when is_integer(tab_stops) do
    tab_stops - rem(column, tab_stops)
  end

  def spaces_to_next_tab(column, tab_stops) when is_list(tab_stops) do
    case Enum.find(tab_stops, fn stop -> stop > column end) do
      nil -> 1
      stop -> stop - column
    end
  end

  @doc """
  Expand tabs in a single line.

  Processes character by character, replacing tabs with the appropriate
  number of spaces to reach the next tab stop.
  """
  def expand_line(line, tab_stops, initial_only) do
    line
    |> String.graphemes()
    |> Enum.reduce({"", 0, false}, fn ch, {result, column, seen_non_blank} ->
      case ch do
        "\t" when initial_only and seen_non_blank ->
          # Past initial blanks -- keep the tab.
          spaces = spaces_to_next_tab(column, tab_stops)
          {result <> "\t", column + spaces, seen_non_blank}

        "\t" ->
          # Replace tab with spaces.
          spaces = spaces_to_next_tab(column, tab_stops)
          {result <> String.duplicate(" ", spaces), column + spaces, seen_non_blank}

        " " ->
          {result <> " ", column + 1, seen_non_blank}

        _ ->
          {result <> ch, column + 1, true}
      end
    end)
    |> elem(0)
  end

  defp process_content(content, tab_stops, initial_only) do
    lines = String.split(content, "\n")

    lines
    |> Enum.with_index()
    |> Enum.each(fn {line, index} ->
      expanded = expand_line(line, tab_stops, initial_only)

      if index < length(lines) - 1 do
        IO.write(expanded <> "\n")
      else
        if line != "" do
          IO.write(expanded)
        end
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_file("-") do
    case IO.read(:stdio, :eof) do
      {:error, reason} -> {:error, reason}
      :eof -> {:ok, ""}
      data -> {:ok, data}
    end
  end

  defp read_file(file), do: File.read(file)

  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "expand.json"),
        else: nil
      ),
      "expand.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "expand.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find expand.json spec file"
  end
end
