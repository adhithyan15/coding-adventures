defmodule UnixTools.Tty do
  @moduledoc """
  tty -- print the file name of the terminal connected to standard input.

  ## What This Program Does

  This is a reimplementation of the POSIX `tty` utility in Elixir. It prints
  the file name of the terminal connected to standard input. If stdin is not
  a terminal (e.g., when piped or redirected), it prints "not a tty".

  ## How tty Works

      $ tty              =>   /dev/ttys001
      $ echo | tty       =>   not a tty
      $ tty -s           =>   (no output, just exit code)

  ## Exit Status

  The exit status communicates whether stdin is a terminal:

  - Exit 0: stdin IS a terminal
  - Exit 1: stdin is NOT a terminal

  This makes `tty` useful in shell scripts for detecting interactive use:

      if tty -s; then
        echo "Running interactively"
      else
        echo "Running from a script or pipe"
      fi

  ## The -s (Silent) Flag

  With `-s`, tty prints nothing at all. It only sets the exit code.
  This is useful when you only care about the yes/no answer, not the
  terminal name.

  ## How We Detect a TTY

  In Erlang/Elixir, we can check if a file descriptor is connected to a
  terminal using the system's `tty` command or by checking if the Erlang
  I/O device is a terminal. We use `:os.cmd/1` to call the system's
  `tty` command, which gives us both the terminal name and the is-a-tty
  check in one call.
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
  3. Check if stdin is a tty.
  4. If -s (silent), just exit with appropriate code.
  5. Otherwise, print the tty name or "not a tty".
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["tty" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags}} ->
        # -----------------------------------------------------------------------
        # Business logic: check if stdin is a tty and report.
        # -----------------------------------------------------------------------

        silent = !!flags["silent"]
        {is_tty, tty_name} = check_tty()

        if silent do
          # Silent mode: no output, just exit code.
          unless is_tty, do: System.halt(1)
        else
          # Normal mode: print tty name or "not a tty".
          IO.puts(tty_name)
          unless is_tty, do: System.halt(1)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "tty: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Check whether stdin is connected to a terminal.

  Returns a tuple `{is_tty, name}` where:

  - `is_tty` is `true` if stdin is a terminal, `false` otherwise
  - `name` is the terminal device path (e.g., "/dev/ttys001") if stdin
    is a terminal, or "not a tty" if it is not

  ## How It Works

  We shell out to the system `tty` command, which reads file descriptor 0
  (stdin) and reports whether it's a terminal. This is the most reliable
  approach because Erlang's I/O system abstracts away file descriptors,
  making it difficult to check directly from Elixir.

  The system `tty` command:
  - Prints the terminal path and exits 0 if stdin is a tty
  - Prints "not a tty" and exits 1 if stdin is not a tty
  """
  def check_tty do
    result =
      try do
        System.cmd("tty", [], stderr_to_stdout: true)
      rescue
        _ -> {"not a tty\n", 1}
      end

    case result do
      {output, 0} ->
        # stdin is a tty. The output is the device path with a trailing newline.
        {true, String.trim(output)}

      {_output, _exit_code} ->
        # stdin is not a tty.
        {false, "not a tty"}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "tty.json"),
        else: nil
      ),
      "tty.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "tty.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find tty.json spec file"
  end
end
