defmodule UnixTools.TrueTool do
  @moduledoc """
  true -- do nothing, successfully.

  ## What This Program Does

  This is a reimplementation of the POSIX `true` utility in Elixir. It does
  absolutely nothing and exits with status code 0 (success).

  That may sound useless, but `true` is a fundamental building block in
  shell scripting. It's used in:

  - Infinite loops:      `while true; do ...; done`
  - Conditional chains:  `command || true` (suppress failure)
  - Default commands:    placeholder in if/else branches

  ## Why Does true Accept --help and --version?

  POSIX `true` ignores all arguments and always exits 0. GNU coreutils
  extends this by supporting `--help` and `--version`. We follow the GNU
  convention, which means CLI Builder handles those two flags for us.
  Any other arguments are silently ignored.

  ## The Simplest Possible CLI Builder Program

  This is the minimal CLI Builder program: no flags, no arguments, no
  commands. The JSON spec defines only `--help` and `--version` via
  `builtin_flags`. The business logic is a single `System.halt(0)`.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Locate the JSON spec file.
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  ## How It Works

  1. Locate the `true.json` spec file relative to the project root.
  2. Prepend "true" to argv (CLI Builder expects argv[0] to be the
     program name).
  3. Call `Parser.parse/2` to validate and parse the arguments.
  4. Pattern-match on the result:
     - `HelpResult` => print help text and exit 0
     - `VersionResult` => print version and exit 0
     - `ParseResult` => exit 0 (the whole point)
     - `ParseErrors` => for `true`, still exit 0 (GNU behavior)
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    # ---------------------------------------------------------------------------
    # Step 1: Parse arguments via CLI Builder
    # ---------------------------------------------------------------------------

    case Parser.parse(spec_path, ["true" | argv]) do
      # -----------------------------------------------------------------------
      # Step 2a: Help was requested
      # -----------------------------------------------------------------------

      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      # -----------------------------------------------------------------------
      # Step 2b: Version was requested
      # -----------------------------------------------------------------------

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      # -----------------------------------------------------------------------
      # Step 2c: Normal invocation -- exit successfully
      # -----------------------------------------------------------------------
      # The entire business logic of `true`: do nothing. Success!

      {:ok, %ParseResult{}} ->
        :ok

      # -----------------------------------------------------------------------
      # Step 2d: Parse errors
      # -----------------------------------------------------------------------
      # For `true`, even parse errors result in success. GNU coreutils
      # `true` ignores everything. We still handle --help and --version
      # above because they are useful, but errors don't change the exit code.

      {:error, %ParseErrors{}} ->
        :ok
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "true.json"),
        else: nil
      ),
      "true.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "true.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find true.json spec file"
  end
end
