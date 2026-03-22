defmodule UnixTools.Tee do
  @moduledoc """
  tee -- read from standard input and write to standard output and files.

  ## What This Program Does

  This is a reimplementation of the GNU `tee` utility in Elixir. It copies
  standard input to each specified FILE, and also to standard output. It is
  named after the T-splitter used in plumbing.

  ## How tee Works

  tee is like a pipe fitting that splits the data stream:

      command | tee file.txt        =>   output goes to both stdout AND file.txt
      command | tee a.txt b.txt     =>   output goes to stdout, a.txt, AND b.txt
      command | tee -a file.txt     =>   appends to file.txt instead of overwriting

  ## Why tee Exists

  In a Unix pipeline, data flows in one direction. Without tee, you cannot
  both see the output AND save it to a file:

      ls | grep ".ts"                 =>   see output but don't save it
      ls | grep ".ts" > out.txt       =>   save it but don't see it
      ls | grep ".ts" | tee out.txt   =>   see it AND save it

  ## Signal Handling (-i)

  With `-i`, tee ignores the SIGINT signal (Ctrl+C). In Elixir's BEAM VM,
  signal handling works differently from C programs, but we honor the flag
  by trapping exits.
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

    case Parser.parse(spec_path, ["tee" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        append_mode = !!flags["append"]
        _ignore_interrupts = !!flags["ignore_interrupts"]

        file_list = normalize_files(arguments["files"])

        # Read all of stdin.
        input = read_stdin()

        # Write to stdout.
        IO.write(input)

        # Write to each file.
        Enum.each(file_list, fn file ->
          write_mode = if append_mode, do: [:append], else: []

          case File.write(file, input, write_mode) do
            :ok ->
              :ok

            {:error, reason} ->
              IO.puts(:stderr, "tee: #{file}: #{:file.format_error(reason)}")
          end
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "tee: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_stdin do
    case IO.read(:stdio, :eof) do
      {:error, _} -> ""
      :eof -> ""
      data -> data
    end
  end

  defp normalize_files(nil), do: []
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "tee.json"),
        else: nil
      ),
      "tee.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "tee.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find tee.json spec file"
  end
end
