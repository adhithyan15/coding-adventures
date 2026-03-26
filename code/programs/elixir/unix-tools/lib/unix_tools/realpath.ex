defmodule UnixTools.Realpath do
  @moduledoc """
  realpath -- print the resolved absolute file name.

  ## What This Program Does

  This is a reimplementation of the GNU `realpath` utility in Elixir. For each
  FILE, it prints the resolved absolute pathname. All symbolic links, `.` and
  `..` components are resolved.

  ## How realpath Works

  realpath resolves a path to its canonical form:

      realpath .                    =>   /home/user/projects
      realpath ../foo               =>   /home/user/foo
      realpath /usr/bin/../lib      =>   /usr/lib

  ## Canonicalization Modes

  - Default: all components must exist, symlinks are resolved.
  - `-e`: All components must exist (strict, same as default).
  - `-m`: No component needs to exist (lenient).
  - `-s`: Don't resolve symlinks, just normalize.

  ## Relative Output

  `--relative-to=DIR` prints the result relative to DIR.
  `--relative-base=DIR` prints relative if the path starts with DIR.
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

    case Parser.parse(spec_path, ["realpath" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        canonicalize_existing = !!flags["canonicalize_existing"]
        canonicalize_missing = !!flags["canonicalize_missing"]
        no_symlinks = !!flags["no_symlinks"]
        quiet = !!flags["quiet"]
        relative_to = flags["relative_to"]
        relative_base = flags["relative_base"]
        zero = !!flags["zero"]

        file_delimiter = if zero, do: <<0>>, else: "\n"
        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          resolve_and_print(
            file,
            canonicalize_existing,
            canonicalize_missing,
            no_symlinks,
            quiet,
            relative_to,
            relative_base,
            file_delimiter
          )
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "realpath: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Path resolution.
  # ---------------------------------------------------------------------------

  @doc """
  Resolve a path to its canonical form.

  Uses different strategies based on the mode flags:
  - `-s`: Just normalize with `Path.expand/1` (no symlink resolution).
  - `-m`: Try to resolve, fall back to `Path.expand/1` for missing paths.
  - Default/`-e`: Resolve fully, error if any component is missing.
  """
  def resolve_path(file_path, _canonicalize_existing, _canonicalize_missing, true = _no_symlinks) do
    {:ok, Path.expand(file_path)}
  end

  def resolve_path(file_path, _canonicalize_existing, true = _canonicalize_missing, _no_symlinks) do
    # Try to resolve symlinks; fall back to expand for missing paths.
    case resolve_with_readlink(file_path) do
      {:ok, resolved} -> {:ok, resolved}
      {:error, _} -> {:ok, Path.expand(file_path)}
    end
  end

  def resolve_path(file_path, _canonicalize_existing, _canonicalize_missing, _no_symlinks) do
    # Default and -e: all components must exist.
    resolve_with_readlink(file_path)
  end

  defp resolve_with_readlink(file_path) do
    # Expand the path first to resolve . and .. components.
    expanded = Path.expand(file_path)

    # Check if the file exists (following symlinks).
    case File.stat(expanded) do
      {:ok, _stat} ->
        # File exists. Now resolve any symlinks in the path.
        case :file.read_link_all(to_charlist(expanded)) do
          {:ok, target} ->
            # It's a symlink -- resolve the target.
            target_path = to_string(target)
            abs_target = Path.expand(target_path, Path.dirname(expanded))
            {:ok, abs_target}

          {:error, :einval} ->
            # Not a symlink -- the expanded path is canonical.
            {:ok, expanded}

          {:error, _} ->
            {:ok, expanded}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp resolve_and_print(
         file,
         canonicalize_existing,
         canonicalize_missing,
         no_symlinks,
         quiet,
         relative_to,
         relative_base,
         file_delimiter
       ) do
    case resolve_path(file, canonicalize_existing, canonicalize_missing, no_symlinks) do
      {:ok, resolved} ->
        # Apply relative-to or relative-base adjustments.
        final =
          cond do
            relative_to != nil ->
              case resolve_path(relative_to, canonicalize_existing, canonicalize_missing, no_symlinks) do
                {:ok, base} -> Path.relative_to(resolved, base)
                {:error, _} -> resolved
              end

            relative_base != nil ->
              case resolve_path(relative_base, canonicalize_existing, canonicalize_missing, no_symlinks) do
                {:ok, base} ->
                  if String.starts_with?(resolved, base) do
                    Path.relative_to(resolved, base)
                  else
                    resolved
                  end

                {:error, _} ->
                  resolved
              end

            true ->
              resolved
          end

        IO.write(final <> file_delimiter)

      {:error, reason} ->
        unless quiet do
          IO.puts(:stderr, "realpath: #{file}: #{:file.format_error(reason)}")
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "realpath.json"),
        else: nil
      ),
      "realpath.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "realpath.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find realpath.json spec file"
  end
end
