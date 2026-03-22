defmodule UnixTools.Uniq do
  @moduledoc """
  uniq -- report or omit repeated lines.

  ## What This Program Does

  This is a reimplementation of the GNU `uniq` utility in Elixir. It filters
  adjacent matching lines from input, writing the results to output.

  ## How uniq Works

  uniq compares **adjacent** lines. It does NOT sort the input -- if you want
  to find all duplicates regardless of position, pipe through `sort` first.

  ## Operation Modes

  - Default: output one copy of each group of adjacent identical lines.
  - `-c` (count): Prefix each line with the number of occurrences.
  - `-d` (repeated): Only show lines that appear more than once.
  - `-u` (unique): Only show lines that appear exactly once.

  ## Comparison Options

  - `-i`: Ignore case when comparing.
  - `-f N`: Skip the first N fields before comparing.
  - `-s N`: Skip the first N characters before comparing.
  - `-w N`: Compare no more than N characters.
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

    case Parser.parse(spec_path, ["uniq" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        show_count = !!flags["count"]
        repeated_only = !!flags["repeated"]
        unique_only = !!flags["unique"]
        skip_fields = flags["skip_fields"] || 0
        skip_chars_count = flags["skip_chars"] || 0
        check_chars_count = flags["check_chars"]
        ignore_case = !!flags["ignore_case"]
        zero_terminated = !!flags["zero_terminated"]
        file_delimiter = if zero_terminated, do: <<0>>, else: "\n"

        input_file = arguments["input"]
        output_file = arguments["output"]

        # Read input.
        content = read_input(input_file)

        # Split into lines.
        lines = String.split(content, file_delimiter)

        # Remove trailing empty element.
        lines =
          if String.ends_with?(content, file_delimiter) and lines != [] do
            List.delete_at(lines, -1)
          else
            lines
          end

        # Group adjacent identical lines.
        groups =
          group_lines(lines, skip_fields, skip_chars_count, check_chars_count, ignore_case)

        # Filter and format.
        output_lines =
          groups
          |> Enum.filter(fn {_line, line_count} ->
            cond do
              repeated_only -> line_count >= 2
              unique_only -> line_count == 1
              true -> true
            end
          end)
          |> Enum.map(fn {line, line_count} ->
            if show_count do
              "      #{line_count} #{line}"
            else
              line
            end
          end)

        output_content =
          if output_lines != [] do
            Enum.join(output_lines, file_delimiter) <> file_delimiter
          else
            ""
          end

        # Write output.
        if output_file do
          File.write!(output_file, output_content)
        else
          IO.write(output_content)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "uniq: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Line comparison and grouping.
  # ---------------------------------------------------------------------------

  @doc """
  Extract the comparison key from a line.

  Applies field skipping, character skipping, and character limiting.
  """
  def get_comparison_key(line, skip_fields, skip_chars_count, check_chars_count, ignore_case) do
    key = line

    # Step 1: Skip fields.
    key =
      if skip_fields > 0 do
        do_skip_fields(key, skip_fields)
      else
        key
      end

    # Step 2: Skip characters.
    key =
      if skip_chars_count > 0 do
        String.slice(key, skip_chars_count..-1//1) || ""
      else
        key
      end

    # Step 3: Limit to check_chars characters.
    key =
      if check_chars_count do
        String.slice(key, 0, check_chars_count)
      else
        key
      end

    # Step 4: Case-insensitive.
    if ignore_case do
      String.downcase(key)
    else
      key
    end
  end

  defp do_skip_fields(str, 0), do: str

  defp do_skip_fields(str, n) do
    # Skip whitespace, then non-whitespace (one field).
    str = String.replace(str, ~r/^[\s]*[\S]*/, "", global: false)
    do_skip_fields(str, n - 1)
  end

  @doc """
  Group adjacent lines that have matching comparison keys.

  Returns a list of {line, count} tuples where `line` is the first line
  in each group and `count` is the number of adjacent matches.
  """
  def group_lines(lines, skip_fields, skip_chars_count, check_chars_count, ignore_case) do
    Enum.reduce(lines, [], fn line, acc ->
      key = get_comparison_key(line, skip_fields, skip_chars_count, check_chars_count, ignore_case)

      case acc do
        [{prev_line, prev_count} | rest] ->
          prev_key =
            get_comparison_key(
              prev_line,
              skip_fields,
              skip_chars_count,
              check_chars_count,
              ignore_case
            )

          if key == prev_key do
            [{prev_line, prev_count + 1} | rest]
          else
            [{line, 1} | acc]
          end

        [] ->
          [{line, 1}]
      end
    end)
    |> Enum.reverse()
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_input(nil), do: read_stdin()
  defp read_input("-"), do: read_stdin()

  defp read_input(file) do
    case File.read(file) do
      {:ok, content} ->
        content

      {:error, reason} ->
        IO.puts(:stderr, "uniq: #{file}: #{:file.format_error(reason)}")
        System.halt(1)
    end
  end

  defp read_stdin do
    case IO.read(:stdio, :eof) do
      {:error, _} -> ""
      :eof -> ""
      data -> data
    end
  end

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "uniq.json"),
        else: nil
      ),
      "uniq.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "uniq.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find uniq.json spec file"
  end
end
