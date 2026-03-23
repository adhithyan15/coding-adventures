defmodule UnixTools.Cp do
  @moduledoc """
  cp -- copy files and directories.

  ## What This Program Does

  This is a reimplementation of the GNU `cp` utility in Elixir. It copies
  files and directories from one location to another.

  ## How cp Works

  At its simplest:

      cp source.txt dest.txt        =>   copies source.txt to dest.txt
      cp file1.txt file2.txt dir/   =>   copies both files into dir/
      cp -r dir1/ dir2/             =>   recursively copies dir1 into dir2

  ## The Destination Problem

  cp must figure out what the destination means. The last argument is always
  the destination. The behavior depends on what it is:

  | Sources     | Destination         | What Happens                      |
  |-------------|---------------------|-----------------------------------|
  | 1 file      | nonexistent path    | Creates a new file at that path   |
  | 1 file      | existing file       | Overwrites the existing file      |
  | 1 file      | existing directory  | Copies file INTO the directory    |
  | 2+ files    | existing directory  | Copies all files INTO directory   |
  | 2+ files    | not a directory     | ERROR — can't copy many to one   |

  ## Overwrite Modes

  Three flags control what happens when the destination already exists:

  - `-f` (force): Remove the destination and try again if it can't be opened
  - `-i` (interactive): Would prompt before overwriting (not implemented here)
  - `-n` (no-clobber): Never overwrite an existing file

  These are mutually exclusive — only the last one on the command line wins.

  ## Recursive Copy

  Without `-R`, cp refuses to copy directories. With `-R`, it copies the
  entire directory tree, preserving the structure. This is implemented using
  `File.cp_r!/3` which handles nested directories and files.

  ## Implementation Approach

  Business logic is kept in pure functions where possible:

  1. `resolve_destination/2` determines the actual destination path for each source.
  2. `copy_single/4` handles copying one source to one destination.
  3. `should_skip?/2` checks the no-clobber condition.
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

    case Parser.parse(spec_path, ["cp" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          recursive: !!flags["recursive"] || !!flags["archive"],
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
          IO.puts(:stderr, "cp: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Run the copy operation for a list of sources and a destination.

  ## How It Works

  1. If there are multiple sources, the destination must be a directory.
  2. For each source, resolve the actual destination path.
  3. Copy each source to its destination, respecting the flags.

  ## Examples

      iex> # Single file copy (conceptual — needs real files)
      iex> UnixTools.Cp.resolve_destination("a.txt", "/tmp/b.txt", false)
      "/tmp/b.txt"
  """
  def run(sources, dest, opts) do
    dest_is_dir = File.dir?(dest)

    # -------------------------------------------------------------------------
    # Validation: multiple sources require a directory destination.
    # -------------------------------------------------------------------------

    if length(sources) > 1 and not dest_is_dir do
      IO.puts(:stderr, "cp: target '#{dest}' is not a directory")
      System.halt(1)
    end

    Enum.each(sources, fn source ->
      actual_dest = resolve_destination(source, dest, dest_is_dir)
      copy_single(source, actual_dest, opts)
    end)
  end

  @doc """
  Resolve the actual destination path for a given source.

  If the destination is an existing directory, the source's basename is
  appended:

      resolve_destination("docs/readme.txt", "/backup", true)
      => "/backup/readme.txt"

  If the destination is not a directory, it's used as-is:

      resolve_destination("a.txt", "b.txt", false)
      => "b.txt"
  """
  def resolve_destination(source, dest, dest_is_dir) do
    if dest_is_dir do
      Path.join(dest, Path.basename(source))
    else
      dest
    end
  end

  @doc """
  Check whether a copy should be skipped.

  Returns `true` if:
  - `no_clobber` is set and the destination already exists
  - `update_only` is set and the destination is newer than (or same age as)
    the source

  ## Truth Table

  | no_clobber | dest_exists | update | dest_newer | Result |
  |------------|-------------|--------|------------|--------|
  | true       | true        | *      | *          | SKIP   |
  | true       | false       | *      | *          | copy   |
  | false      | *           | true   | true       | SKIP   |
  | false      | *           | true   | false      | copy   |
  | false      | *           | false  | *          | copy   |
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
  Copy a single source to a single destination.

  Handles:
  - Skipping if no-clobber or update conditions apply
  - Directory vs file distinction
  - Recursive copy for directories
  - Force flag for removing stubborn destinations
  - Verbose output
  """
  def copy_single(source, dest, opts) do
    # -------------------------------------------------------------------------
    # Does the source exist?
    # -------------------------------------------------------------------------

    unless File.exists?(source) do
      IO.puts(:stderr, "cp: cannot stat '#{source}': No such file or directory")
      return_with_error()
    end

    # -------------------------------------------------------------------------
    # Is the source a directory but -R not specified?
    # -------------------------------------------------------------------------

    if File.dir?(source) and not opts.recursive do
      IO.puts(:stderr, "cp: -r not specified; omitting directory '#{source}'")
      return_with_error()
    end

    # -------------------------------------------------------------------------
    # Should we skip this copy? (no-clobber / update)
    # -------------------------------------------------------------------------

    unless should_skip?(source, dest, opts) do
      # -----------------------------------------------------------------------
      # Force: remove destination if it exists and force is set.
      # -----------------------------------------------------------------------

      if opts.force and File.exists?(dest) do
        File.rm_rf(dest)
      end

      # -----------------------------------------------------------------------
      # Perform the copy.
      # -----------------------------------------------------------------------

      if File.dir?(source) do
        File.cp_r!(source, dest)
      else
        # Ensure the destination directory exists.
        dest_dir = Path.dirname(dest)

        unless File.dir?(dest_dir) do
          File.mkdir_p!(dest_dir)
        end

        File.cp!(source, dest)
      end

      if opts.verbose do
        IO.puts("'#{source}' -> '#{dest}'")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp return_with_error, do: :error

  @doc """
  Split the arguments list into sources and destination.

  The last argument is always the destination; everything before it is a source:

      split_sources_and_dest(["a.txt", "b.txt", "dir/"])
      => {["a.txt", "b.txt"], "dir/"}
  """
  def split_sources_and_dest(args) do
    {Enum.slice(args, 0, length(args) - 1), List.last(args)}
  end

  defp normalize_args(args) when is_list(args), do: args
  defp normalize_args(arg) when is_binary(arg), do: [arg]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "cp.json"),
        else: nil
      ),
      "cp.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "cp.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find cp.json spec file"
  end
end
