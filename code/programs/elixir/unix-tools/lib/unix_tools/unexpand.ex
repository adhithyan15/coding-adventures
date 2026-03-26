defmodule UnixTools.Unexpand do
  @moduledoc """
  unexpand -- convert spaces to tabs.

  ## What This Program Does

  This is a reimplementation of the GNU `unexpand` utility in Elixir. It
  converts leading sequences of spaces to tabs, the inverse of `expand`.

  ## How unexpand Works

  By default, unexpand only converts leading blanks to tabs. With `-a`, it
  converts all sequences of spaces that align with tab stops.

      unexpand file.txt          =>   convert leading spaces to tabs
      unexpand -a file.txt       =>   convert ALL aligned spaces to tabs
      unexpand -t 4 file.txt     =>   use tab stops every 4 columns

  ## The Algorithm

  For each line, we track the column position. When spaces span a tab stop
  boundary, we replace them with a tab character. Only complete spans to
  a tab stop are replaced -- partial spans remain as spaces.
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

    case Parser.parse(spec_path, ["unexpand" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        convert_all = !!flags["all"] and not !!flags["first_only"]
        tabs_str = flags["tabs"]
        tab_size = if tabs_str, do: String.to_integer(tabs_str), else: 8

        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          case read_file(file) do
            {:ok, content} ->
              process_content(content, tab_size, convert_all)

            {:error, reason} ->
              IO.puts(:stderr, "unexpand: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "unexpand: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Space-to-tab conversion.
  # ---------------------------------------------------------------------------

  @doc """
  Convert spaces to tabs in a single line.

  We track column position and accumulate spaces. When we reach a tab stop
  with accumulated spaces, we replace them with a tab.
  """
  def unexpand_line(line, tab_size, convert_all) do
    line
    |> String.graphemes()
    |> Enum.reduce({"", 0, 0, false}, fn ch, {result, column, pending_spaces, seen_non_blank} ->
      cond do
        ch == " " and (convert_all or not seen_non_blank) ->
          # Accumulate spaces.
          new_column = column + 1
          new_pending = pending_spaces + 1

          if rem(new_column, tab_size) == 0 do
            # Reached a tab stop -- replace with tab.
            {result <> "\t", new_column, 0, seen_non_blank}
          else
            {result, new_column, new_pending, seen_non_blank}
          end

        ch == "\t" and (convert_all or not seen_non_blank) ->
          # Tab already -- flush and keep.
          advance = tab_size - rem(column, tab_size)
          {result <> "\t", column + advance, 0, seen_non_blank}

        true ->
          # Non-blank or blank past initial (when not in -a mode).
          new_seen = seen_non_blank or (ch != " " and ch != "\t")

          # Flush pending spaces as literal spaces.
          flushed =
            if pending_spaces > 0 do
              result <> String.duplicate(" ", pending_spaces)
            else
              result
            end

          {flushed <> ch, column + 1, 0, new_seen}
      end
    end)
    |> then(fn {result, _column, pending_spaces, _seen} ->
      # Flush any remaining pending spaces.
      if pending_spaces > 0 do
        result <> String.duplicate(" ", pending_spaces)
      else
        result
      end
    end)
  end

  defp process_content(content, tab_size, convert_all) do
    lines = String.split(content, "\n")

    lines
    |> Enum.with_index()
    |> Enum.each(fn {line, index} ->
      converted = unexpand_line(line, tab_size, convert_all)

      if index < length(lines) - 1 do
        IO.write(converted <> "\n")
      else
        if line != "" do
          IO.write(converted)
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "unexpand.json"),
        else: nil
      ),
      "unexpand.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "unexpand.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find unexpand.json spec file"
  end
end
