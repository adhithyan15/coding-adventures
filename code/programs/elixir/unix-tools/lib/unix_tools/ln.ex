defmodule UnixTools.Ln do
  @moduledoc """
  ln -- make links between files.

  ## What This Program Does

  This is a reimplementation of the GNU `ln` utility in Elixir. It creates
  links between files -- either hard links (default) or symbolic links
  (with -s).

  ## Hard Links vs Symbolic Links

  A **hard link** is another directory entry pointing to the same inode.
  Both names are equal -- deleting one doesn't affect the other. Hard links
  cannot span filesystems and cannot link to directories.

  A **symbolic link** (symlink) is a special file that contains a path to
  another file. It's like a shortcut that can point anywhere.

      ln target link           =>   hard link
      ln -s target link        =>   symlink

  ## Usage Patterns

  - Two-argument form: `ln TARGET LINK_NAME`
  - One-argument form: `ln TARGET` (creates link with same basename in cwd)
  - Multiple targets: `ln TARGET... DIRECTORY` (creates links in DIRECTORY)
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

    case Parser.parse(spec_path, ["ln" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        symbolic = !!flags["symbolic"]
        force = !!flags["force"]
        verbose = !!flags["verbose"]

        targets = normalize_targets(arguments["targets"])

        case length(targets) do
          1 ->
            # One-argument form: link basename in current directory.
            target = hd(targets)
            link_name = Path.basename(target)
            create_link(target, link_name, symbolic, force, verbose)

          2 ->
            # Check if last arg is a directory.
            [target, last] = targets

            if File.dir?(last) do
              link_name = Path.join(last, Path.basename(target))
              create_link(target, link_name, symbolic, force, verbose)
            else
              create_link(target, last, symbolic, force, verbose)
            end

          _ ->
            # Multiple targets: last must be a directory.
            {file_targets, [dir]} = Enum.split(targets, length(targets) - 1)

            if File.dir?(dir) do
              Enum.each(file_targets, fn target ->
                link_name = Path.join(dir, Path.basename(target))
                create_link(target, link_name, symbolic, force, verbose)
              end)
            else
              IO.puts(:stderr, "ln: target '#{dir}' is not a directory")
              System.halt(1)
            end
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "ln: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Create a single link.
  # ---------------------------------------------------------------------------

  @doc """
  Create a single link (hard or symbolic).

  If `force` is true, we remove the destination first if it exists.
  If `symbolic` is true, we create a symbolic link; otherwise a hard link.
  """
  def create_link(target, link_name, symbolic, force, verbose) do
    # Adjust link_name if it points to an existing directory.
    final_link_name =
      if File.dir?(link_name) do
        Path.join(link_name, Path.basename(target))
      else
        link_name
      end

    # Remove existing file if force is set.
    if force do
      File.rm(final_link_name)
    end

    result =
      if symbolic do
        File.ln_s(target, final_link_name)
      else
        File.ln(target, final_link_name)
      end

    case result do
      :ok ->
        if verbose do
          arrow = if symbolic, do: " -> ", else: " => "
          IO.puts("'#{final_link_name}'#{arrow}'#{target}'")
        end

      {:error, reason} ->
        IO.puts(:stderr, "ln: #{:file.format_error(reason)}")
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_targets(targets) when is_list(targets), do: targets
  defp normalize_targets(target) when is_binary(target), do: [target]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "ln.json"),
        else: nil
      ),
      "ln.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "ln.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find ln.json spec file"
  end
end
