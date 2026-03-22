defmodule UnixTools.Rev do
  @moduledoc """
  rev -- reverse lines characterwise.

  ## What This Program Does

  This is a reimplementation of the `rev` utility in Elixir. It copies the
  specified files to standard output, reversing the order of characters in
  every line.

  ## How rev Works

  rev reverses each line independently:

      echo "hello" | rev    =>   "olleh"
      echo "abc\\nxyz" | rev =>   "cba\\nzyx"

  ## Why rev Exists

  rev is useful in shell pipelines for manipulating text. A common idiom
  is using `rev | cut | rev` to extract fields from the end of a line:

      echo "/usr/local/bin/node" | rev | cut -d/ -f1 | rev
      => "node"

  ## Unicode Handling

  Elixir's `String.reverse/1` is Unicode-aware. It correctly handles
  multi-byte characters, grapheme clusters, and combining characters.
  This is one area where Elixir shines compared to languages that operate
  on bytes or code units.
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

    case Parser.parse(spec_path, ["rev" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{arguments: arguments}} ->
        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          case read_file(file) do
            {:ok, content} ->
              process_content(content)

            {:error, reason} ->
              IO.puts(:stderr, "rev: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "rev: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Reverse each line.
  # ---------------------------------------------------------------------------

  @doc """
  Reverse a single line's characters.

  Elixir's `String.reverse/1` handles Unicode grapheme clusters correctly,
  so we don't need the manual surrogate pair handling that JavaScript requires.

  ## Examples

      reverse_line("hello")  => "olleh"
      reverse_line("ab cd")  => "dc ba"
      reverse_line("")       => ""
  """
  def reverse_line(line) do
    String.reverse(line)
  end

  @doc """
  Process file content: reverse each line and write to stdout.

  We split on newlines, reverse each line independently, and output
  each with a trailing newline.
  """
  def process_content(content) do
    lines = String.split(content, "\n")

    has_trailing = String.ends_with?(content, "\n")

    effective_lines =
      if has_trailing do
        Enum.slice(lines, 0..(length(lines) - 2)//1)
      else
        lines
      end

    Enum.each(effective_lines, fn line ->
      IO.puts(reverse_line(line))
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "rev.json"),
        else: nil
      ),
      "rev.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "rev.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find rev.json spec file"
  end
end
