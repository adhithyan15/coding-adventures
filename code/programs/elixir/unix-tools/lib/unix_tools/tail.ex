defmodule UnixTools.Tail do
  @moduledoc """
  tail -- output the last part of files.

  ## What This Program Does

  This is a reimplementation of the GNU `tail` utility in Elixir. It prints
  the last N lines (or bytes) of each file to standard output. By default,
  N is 10.

  ## How tail Works

  tail is the complement of head:

      tail file.txt          =>   last 10 lines
      tail -n 5 file.txt     =>   last 5 lines
      tail -n +3 file.txt    =>   everything starting from line 3
      tail -c 100 file.txt   =>   last 100 bytes

  ## The +NUM Syntax

  GNU tail supports a special prefix syntax:

  - `-n 5` or `-n -5`: Output the last 5 lines.
  - `-n +5`: Output starting from line 5 (1-indexed).

  This means the -n and -c flags accept strings, not plain integers,
  because the `+` prefix changes the semantics entirely.
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

    case Parser.parse(spec_path, ["tail" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        lines_str = flags["lines"] || "10"
        bytes_str = flags["bytes"]
        byte_mode = bytes_str != nil
        quiet = !!flags["quiet"]
        verbose = !!flags["verbose"]
        zero_terminated = !!flags["zero_terminated"]
        delimiter = if zero_terminated, do: <<0>>, else: "\n"

        lines_parsed = parse_count(lines_str)
        bytes_parsed = if byte_mode, do: parse_count(bytes_str), else: nil

        file_list = normalize_files(arguments["files"])
        show_headers = determine_headers(quiet, verbose, length(file_list))

        file_list
        |> Enum.with_index()
        |> Enum.each(fn {file, index} ->
          if show_headers do
            if index > 0, do: IO.write("\n")
            label = if file == "-", do: "standard input", else: file
            IO.write("==> #{label} <==\n")
          end

          case read_file(file) do
            {:ok, content} ->
              if byte_mode do
                {count, from_start} = bytes_parsed
                IO.write(tail_bytes(content, count, from_start))
              else
                {count, from_start} = lines_parsed
                IO.write(tail_lines(content, count, from_start, delimiter))
              end

            {:error, reason} ->
              IO.puts(:stderr, "tail: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "tail: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Parse +/- count syntax.
  # ---------------------------------------------------------------------------

  @doc """
  Parse a count string that may have a + or - prefix.

  Returns `{value, from_start}` where `from_start` is true for + prefix.

  ## Truth Table

      Input    value   from_start
      -----    -----   ----------
      "10"     10      false       (last 10)
      "-10"    10      false       (last 10, explicit)
      "+10"    10      true        (from line/byte 10)
  """
  def parse_count("+" <> rest) do
    {String.to_integer(rest), true}
  end

  def parse_count("-" <> rest) do
    {String.to_integer(rest), false}
  end

  def parse_count(input) do
    {String.to_integer(input), false}
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Extract last N lines or bytes.
  # ---------------------------------------------------------------------------

  @doc """
  Extract lines from content according to the parsed count.

  If `from_start` is true, we output from line N onward (1-indexed).
  Otherwise, we output the last N lines.
  """
  def tail_lines("", _count, _from_start, _delimiter), do: ""

  def tail_lines(content, count, from_start, delimiter) do
    lines = String.split(content, delimiter)
    has_trailing = String.ends_with?(content, delimiter)

    effective_lines =
      if has_trailing do
        Enum.slice(lines, 0..(length(lines) - 2)//1)
      else
        lines
      end

    selected =
      if from_start do
        # +N means "start from line N" (1-indexed).
        Enum.drop(effective_lines, count - 1)
      else
        # Last N lines.
        len = length(effective_lines)

        if count >= len do
          effective_lines
        else
          Enum.drop(effective_lines, len - count)
        end
      end

    if length(selected) == 0 do
      ""
    else
      Enum.join(selected, delimiter) <> delimiter
    end
  end

  @doc """
  Extract bytes from content according to the parsed count.
  """
  def tail_bytes(content, count, from_start) do
    size = byte_size(content)

    if from_start do
      # +N means start from byte N (1-indexed).
      start = min(count - 1, size)
      binary_part(content, start, size - start)
    else
      # Last N bytes.
      start = max(0, size - count)
      binary_part(content, start, size - start)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp determine_headers(quiet, verbose, file_count) do
    cond do
      quiet -> false
      verbose -> true
      file_count > 1 -> true
      true -> false
    end
  end

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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "tail.json"),
        else: nil
      ),
      "tail.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "tail.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find tail.json spec file"
  end
end
