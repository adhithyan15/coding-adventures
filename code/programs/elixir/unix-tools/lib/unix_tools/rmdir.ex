defmodule UnixTools.Rmdir do
  @moduledoc """
  rmdir -- remove empty directories.

  ## What This Program Does

  This is a reimplementation of the GNU `rmdir` utility in Elixir. It removes
  each specified directory, but only if the directory is empty.

  ## How rmdir Works

  rmdir is the safe counterpart to `rm -r`. It refuses to delete directories
  that contain files:

      rmdir emptydir           =>   removes emptydir (if empty)
      rmdir notempty           =>   error: directory not empty
      rmdir -p a/b/c           =>   removes c, then b, then a

  ## The -p Flag (Parents)

  With `-p`, rmdir removes each component of the path. For example,
  `rmdir -p a/b/c` is equivalent to running rmdir on a/b/c, then a/b, then a.
  Each directory must be empty at the time of removal.

  ## --ignore-fail-on-non-empty

  This flag suppresses errors when a directory cannot be removed because it
  is not empty. Other errors (permission denied, etc.) are still reported.
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

    case Parser.parse(spec_path, ["rmdir" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        parents = !!flags["parents"]
        verbose = !!flags["verbose"]
        ignore_fail = !!flags["ignore_fail"]

        dirs = normalize_dirs(arguments["directories"])

        Enum.each(dirs, fn dir ->
          chain = if parents, do: get_parent_chain(dir), else: [dir]
          remove_chain(chain, verbose, ignore_fail)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "rmdir: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Remove directories and parent chains.
  # ---------------------------------------------------------------------------

  @doc """
  Get the parent chain for a path.

  For "a/b/c", returns ["a/b/c", "a/b", "a"] -- deepest first.
  We repeatedly take the dirname until we reach "." or the path stops changing.
  """
  def get_parent_chain(dir) do
    do_parent_chain(dir, [dir])
  end

  defp do_parent_chain(current, acc) do
    parent = Path.dirname(current)

    if parent == current or parent == "." do
      acc
    else
      do_parent_chain(parent, acc ++ [parent])
    end
  end

  @doc """
  Remove a chain of directories in order.

  If any removal fails, we stop processing the remaining directories in the
  chain (because they can't be empty if a child still exists).
  """
  def remove_chain(chain, verbose, ignore_fail) do
    Enum.reduce_while(chain, :ok, fn dir, _acc ->
      case File.rmdir(dir) do
        :ok ->
          if verbose do
            IO.puts("rmdir: removing directory, '#{dir}'")
          end

          {:cont, :ok}

        {:error, :enotempty} when ignore_fail ->
          {:halt, :ignored}

        {:error, reason} ->
          IO.puts(:stderr, "rmdir: failed to remove '#{dir}': #{:file.format_error(reason)}")
          {:halt, :error}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_dirs(dirs) when is_list(dirs), do: dirs
  defp normalize_dirs(dir) when is_binary(dir), do: [dir]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "rmdir.json"),
        else: nil
      ),
      "rmdir.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "rmdir.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find rmdir.json spec file"
  end
end
