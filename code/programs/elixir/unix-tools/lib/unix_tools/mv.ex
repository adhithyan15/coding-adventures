defmodule UnixTools.Mv do
  @moduledoc """
  mv -- move (rename) files.

  ## What This Program Does

  This is a reimplementation of the GNU `mv` utility in Elixir. It moves
  or renames files and directories.

  ## How mv Works

  At its simplest:

      mv old.txt new.txt            =>   renames old.txt to new.txt
      mv file.txt dir/              =>   moves file.txt into dir/
      mv dir1/ dir2/                =>   renames dir1 to dir2

  ## mv vs cp: What's the Difference?

  - `cp` creates a new copy, leaving the original intact.
  - `mv` relocates the file — the original is gone after the move.

  Under the hood, `mv` first tries `rename(2)` which is an atomic operation
  that just updates directory entries. If the source and destination are on
  different filesystems, `rename` fails, so `mv` falls back to copy + delete.

  ## Overwrite Modes

  Three flags control what happens when the destination already exists:

  - `-f` (force): Do not prompt before overwriting (default behavior)
  - `-i` (interactive): Would prompt before overwriting (not implemented)
  - `-n` (no-clobber): Never overwrite an existing file

  ## Implementation Approach

  Business logic is in pure functions:

  1. `resolve_destination/3` determines the actual path for each source.
  2. `move_single/3` handles moving one source, with rename-then-fallback.
  3. `should_skip?/3` checks no-clobber and update conditions.
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

    case Parser.parse(spec_path, ["mv" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          force: !!flags["force"],
          no_clobber: !!flags["no_clobber"],
          verbose: !!flags["verbose"],
          update: !!flags["update"]
        }

        sources_and_dest = normalize_args(arguments["sources"])
        {sources, dest} = split_sources_and_dest(sources_and_dest)

        run(sources, dest, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "mv: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Run the move operation for a list of sources and a destination.

  Multiple sources require the destination to be an existing directory.
  """
  def run(sources, dest, opts) do
    dest_is_dir = File.dir?(dest)

    if length(sources) > 1 and not dest_is_dir do
      IO.puts(:stderr, "mv: target '#{dest}' is not a directory")
      System.halt(1)
    end

    Enum.each(sources, fn source ->
      actual_dest = resolve_destination(source, dest, dest_is_dir)
      move_single(source, actual_dest, opts)
    end)
  end

  @doc """
  Resolve the destination path for a single source.

  If the destination is a directory, the source's basename is appended:

      resolve_destination("docs/file.txt", "/backup", true)
      => "/backup/file.txt"

  Otherwise the destination is used as-is:

      resolve_destination("old.txt", "new.txt", false)
      => "new.txt"
  """
  def resolve_destination(source, dest, dest_is_dir) do
    if dest_is_dir do
      Path.join(dest, Path.basename(source))
    else
      dest
    end
  end

  @doc """
  Check whether a move should be skipped.

  Returns `true` if:
  - `no_clobber` is set and the destination already exists
  - `update` is set and the destination is newer than the source

  ## Truth Table

  | no_clobber | dest_exists | update | dest_newer | Result |
  |------------|-------------|--------|------------|--------|
  | true       | true        | *      | *          | SKIP   |
  | false      | *           | true   | true       | SKIP   |
  | otherwise  | *           | *      | *          | move   |
  """
  def should_skip?(source, dest, opts) do
    dest_exists = File.exists?(dest)

    cond do
      opts.no_clobber and dest_exists ->
        true

      opts.update and dest_exists ->
        source_mtime = File.stat!(source).mtime
        dest_mtime = File.stat!(dest).mtime
        dest_mtime >= source_mtime

      true ->
        false
    end
  end

  @doc """
  Move a single source to a destination.

  The strategy is:
  1. Try `File.rename/2` (atomic, same-filesystem only).
  2. If rename fails (e.g., cross-device), fall back to copy + delete.

  This mirrors how GNU mv works: rename is fast (just updates a directory
  entry), but only works within the same filesystem. For cross-device moves,
  the data must actually be copied byte by byte.
  """
  def move_single(source, dest, opts) do
    unless File.exists?(source) do
      IO.puts(:stderr, "mv: cannot stat '#{source}': No such file or directory")
      return_with_error()
    end

    unless should_skip?(source, dest, opts) do
      result = File.rename(source, dest)

      case result do
        :ok ->
          :ok

        {:error, :exdev} ->
          # ---------------------------------------------------------------
          # Cross-device move: copy then delete the source.
          # ---------------------------------------------------------------

          if File.dir?(source) do
            File.cp_r!(source, dest)
            File.rm_rf!(source)
          else
            File.cp!(source, dest)
            File.rm!(source)
          end

        {:error, reason} ->
          IO.puts(:stderr, "mv: cannot move '#{source}' to '#{dest}': #{:file.format_error(reason)}")
      end

      if opts.verbose do
        IO.puts("renamed '#{source}' -> '#{dest}'")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp return_with_error, do: :error

  @doc """
  Split arguments into sources (all but last) and destination (last).
  """
  def split_sources_and_dest(args) do
    {Enum.slice(args, 0, length(args) - 1), List.last(args)}
  end

  defp normalize_args(args) when is_list(args), do: args
  defp normalize_args(arg) when is_binary(arg), do: [arg]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "mv.json"),
        else: nil
      ),
      "mv.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "mv.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find mv.json spec file"
  end
end
