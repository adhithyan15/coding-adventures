defmodule UnixTools.Yes do
  @moduledoc """
  yes -- output a string repeatedly until killed.

  ## What This Program Does

  This is a reimplementation of the GNU `yes` utility in Elixir. It outputs
  a string repeatedly, one line at a time, until it is killed or the output
  pipe is closed.

  ## How yes Works

  At its simplest, yes just prints "y" forever:

      yes         =>   y\\ny\\ny\\ny\\n...
      yes hello   =>   hello\\nhello\\nhello\\n...
      yes a b c   =>   a b c\\na b c\\na b c\\n...

  When given multiple arguments, they are joined with spaces, just like echo.
  The result is printed on each line.

  ## Why yes Exists

  `yes` is used in shell scripting to automatically answer "yes" to
  interactive prompts:

      yes | rm -i *.tmp    =>   answers "y" to every "remove?" prompt
      yes n | some-command  =>   answers "n" to every prompt

  It is also used for stress testing and benchmarking I/O throughput.

  ## Testability

  The `yes_output/2` function generates a finite number of lines for
  testing. The `main/1` entry point runs indefinitely (until SIGPIPE or
  SIGTERM), which is the correct behavior for the real utility but
  untestable in a unit test.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  ## How It Works

  1. Parse arguments with CLI Builder.
  2. Handle --help and --version.
  3. Join positional arguments with spaces (default: "y").
  4. Print the resulting line forever.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["yes" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{arguments: arguments}} ->
        # -----------------------------------------------------------------------
        # Build the output line from arguments.
        # -----------------------------------------------------------------------
        # The "string" argument is variadic, so it comes as a list.
        # If no arguments were given, default to "y".

        strings = normalize_strings(arguments["string"])
        line = if strings == [], do: "y", else: Enum.join(strings, " ")

        # Print forever. This loop will terminate when the output pipe
        # closes (SIGPIPE) or the process is killed (SIGTERM).
        print_forever(line)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "yes: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Infinite output
  # ---------------------------------------------------------------------------

  @doc """
  Print a line forever.

  This function loops indefinitely, writing `line` followed by a newline
  to stdout on each iteration. It will terminate when:

  - The output pipe is closed (causing an :ebadf or similar error)
  - The process receives SIGTERM or SIGKILL

  In a Unix pipeline like `yes | head -5`, the `head` command closes the
  pipe after reading 5 lines, which causes the write to fail and the
  process to exit.
  """
  def print_forever(line) do
    case IO.puts(line) do
      :ok -> print_forever(line)
      _ -> :ok
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Testable output generator
  # ---------------------------------------------------------------------------

  @doc """
  Generate a finite number of yes output lines for testing.

  Returns a list of strings, each being the `line` that would be printed.
  This function exists because `print_forever/1` runs indefinitely, making
  it impossible to test directly in a unit test.

  ## Parameters

  - `line` — the string to repeat (e.g., "y" or "hello world")
  - `max_lines` — the number of lines to generate

  ## Examples

      iex> UnixTools.Yes.yes_output("y", 3)
      ["y", "y", "y"]

      iex> UnixTools.Yes.yes_output("hello world", 2)
      ["hello world", "hello world"]
  """
  def yes_output(line, max_lines) when max_lines >= 0 do
    List.duplicate(line, max_lines)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_strings(nil), do: []
  defp normalize_strings(strings) when is_list(strings), do: strings
  defp normalize_strings(string) when is_binary(string), do: [string]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "yes.json"),
        else: nil
      ),
      "yes.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "yes.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find yes.json spec file"
  end
end
