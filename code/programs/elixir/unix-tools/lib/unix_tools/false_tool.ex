defmodule UnixTools.FalseTool do
  @moduledoc """
  false -- do nothing, unsuccessfully.

  ## What This Program Does

  This is a reimplementation of the POSIX `false` utility in Elixir. It does
  absolutely nothing and exits with status code 1 (failure).

  `false` is the counterpart to `true`. Where `true` always succeeds,
  `false` always fails. It's used in shell scripting for:

  - Breaking loops:    `while false; do ...; done` (never executes)
  - Testing:           `if false; then ...; fi` (never true)
  - Conditional chains: `command && false` (force failure)

  ## The Mirror Image of true

  This module is structurally identical to `TrueTool`. The only difference
  is the exit code: 1 instead of 0. Both support `--help` and `--version`
  via CLI Builder's builtin flags.

  This symmetry reflects the boolean nature of these utilities: they are
  the constants `true` and `false` of shell scripting.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Locate the JSON spec file.
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  Identical structure to `TrueTool.main/1`, but exits with code 1 instead
  of 0. The --help and --version flags still exit 0 (they succeeded at
  their task), matching GNU coreutils behavior.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    # ---------------------------------------------------------------------------
    # Step 1: Parse arguments via CLI Builder
    # ---------------------------------------------------------------------------

    case Parser.parse(spec_path, ["false" | argv]) do
      # -----------------------------------------------------------------------
      # Step 2a: Help was requested
      # -----------------------------------------------------------------------
      # Note: --help exits 0 even for `false`. The help request succeeded.

      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      # -----------------------------------------------------------------------
      # Step 2b: Version was requested
      # -----------------------------------------------------------------------

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      # -----------------------------------------------------------------------
      # Step 2c: Normal invocation -- exit unsuccessfully
      # -----------------------------------------------------------------------
      # The entire business logic of `false`: fail. Always.

      {:ok, %ParseResult{}} ->
        System.halt(1)

      # -----------------------------------------------------------------------
      # Step 2d: Parse errors
      # -----------------------------------------------------------------------
      # For `false`, errors also result in exit 1. It always fails.

      {:error, %ParseErrors{}} ->
        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "false.json"),
        else: nil
      ),
      "false.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "false.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find false.json spec file"
  end
end
