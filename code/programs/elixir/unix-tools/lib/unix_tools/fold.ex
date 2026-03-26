defmodule UnixTools.Fold do
  @moduledoc """
  fold -- wrap each input line to fit in specified width.

  ## What This Program Does

  This is a reimplementation of the GNU `fold` utility in Elixir. It wraps
  input lines that are longer than a specified width (default: 80 columns).

  ## How fold Works

  fold reads input and inserts newlines to ensure no output line exceeds
  the specified width:

      fold file.txt              =>   wrap at 80 columns
      fold -w 40 file.txt        =>   wrap at 40 columns
      fold -s file.txt           =>   break at word boundaries (spaces)
      fold -b file.txt           =>   count bytes, not columns

  ## Breaking at Spaces (-s)

  Without `-s`, fold breaks at exactly the width, even mid-word. With `-s`,
  fold tries to break at the last space before the width limit.
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

    case Parser.parse(spec_path, ["fold" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        count_bytes = !!flags["bytes"]
        break_at_spaces = !!flags["spaces"]
        width = flags["width"] || 80

        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          case read_file(file) do
            {:ok, content} ->
              process_content(content, width, break_at_spaces, count_bytes)

            {:error, reason} ->
              IO.puts(:stderr, "fold: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "fold: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Line folding.
  # ---------------------------------------------------------------------------

  @doc """
  Fold a single line to fit within the specified width.

  Walks through the line character by character, tracking column width.
  When the width is exceeded, inserts a newline break.

  With `-s`, breaks at the last space before the width limit.
  With `-b`, counts bytes instead of display columns.
  """
  def fold_line(line, width, break_at_spaces, count_bytes) do
    if width <= 0, do: line

    line
    |> String.graphemes()
    |> do_fold(width, break_at_spaces, count_bytes, [], "", 0, -1, 0)
  end

  defp do_fold([], _width, _break_spaces, _count_bytes, segments, current, _last_space_idx, _last_space_width) do
    if current != "" do
      segments ++ [current]
    else
      segments
    end
    |> Enum.join("\n")
  end

  defp do_fold(
         [ch | rest],
         width,
         break_at_spaces,
         count_bytes,
         segments,
         current,
         last_space_idx,
         last_space_width
       ) do
    # Calculate character width.
    char_width =
      cond do
        count_bytes -> byte_size(ch)
        ch == "\t" -> 8 - rem(String.length(current) - count_leading(current), 8)
        ch == "\b" -> if String.length(current) > 0, do: -1, else: 0
        true -> 1
      end

    # Track last space for -s mode.
    {new_space_idx, new_space_width} =
      if break_at_spaces and ch == " " do
        {String.length(current), String.length(current)}
      else
        {last_space_idx, last_space_width}
      end

    current_width = String.length(current)

    if current_width + char_width > width do
      if break_at_spaces and new_space_idx >= 0 do
        # Break at the last space.
        before = String.slice(current, 0, new_space_idx + 1)
        after_space = String.slice(current, (new_space_idx + 1)..-1//1) <> ch

        do_fold(
          rest,
          width,
          break_at_spaces,
          count_bytes,
          segments ++ [before],
          after_space,
          String.length(after_space) - 1,
          -1,
          0
        )
      else
        # Break at current position.
        do_fold(
          rest,
          width,
          break_at_spaces,
          count_bytes,
          segments ++ [current],
          ch,
          -1,
          0
        )
      end
    else
      do_fold(
        rest,
        width,
        break_at_spaces,
        count_bytes,
        segments,
        current <> ch,
        new_space_idx,
        new_space_width
      )
    end
  end

  # Handle the 9-arg version (after space break).
  defp do_fold(chars, width, break_spaces, count_bytes, segments, current, _space_idx, _space_width, _) do
    do_fold(chars, width, break_spaces, count_bytes, segments, current, -1, 0)
  end

  defp count_leading(str) do
    str
    |> String.graphemes()
    |> Enum.take_while(fn ch -> ch == " " or ch == "\t" end)
    |> length()
  end

  defp process_content(content, width, break_at_spaces, count_bytes) do
    lines = String.split(content, "\n")

    lines
    |> Enum.with_index()
    |> Enum.each(fn {line, index} ->
      folded = fold_line(line, width, break_at_spaces, count_bytes)

      if index < length(lines) - 1 do
        IO.write(folded <> "\n")
      else
        if line != "" do
          IO.write(folded)
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "fold.json"),
        else: nil
      ),
      "fold.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "fold.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find fold.json spec file"
  end
end
