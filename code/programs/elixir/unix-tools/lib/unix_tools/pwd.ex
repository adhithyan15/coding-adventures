defmodule UnixTools.Pwd do
  @moduledoc """
  pwd — print the absolute pathname of the current working directory.

  ## What This Program Does

  This is a reimplementation of the POSIX `pwd` utility in Elixir. It prints
  the absolute path of the current working directory to standard output.

  ## How CLI Builder Powers This

  The entire command-line interface — flags, help text, version output,
  error messages — is defined in `pwd.json`. This program never parses
  a single argument by hand. Instead:

  1. We hand `pwd.json` and `argv` to CLI Builder's `Parser.parse/2`.
  2. The parser validates the input, enforces mutual exclusivity of
     `-L` and `-P`, generates help text, and returns a tagged result.
  3. We pattern-match on the result and run the business logic.

  The result is that *this file contains only business logic*. All parsing,
  validation, and help generation happen inside CLI Builder, driven by the
  JSON spec.

  ## Logical vs Physical Paths

  When you `cd` through a symbolic link, the shell updates the `$PWD`
  environment variable to reflect the path *as you typed it* — including
  the symlink. This is the "logical" path.

  The "physical" path resolves all symlinks. For example, if `/home` is
  a symlink to `/usr/home`:

      Logical:  /home/user       (what $PWD says)
      Physical: /usr/home/user   (what the filesystem says)

  By default (`-L`), we print the logical path. With `-P`, we resolve
  symlinks and print the physical path.

  ## POSIX Compliance Note

  If `$PWD` is not set, or if it doesn't match the actual current
  directory, even `-L` mode falls back to the physical path. This
  matches POSIX behavior.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Locate the JSON spec file.
  # ---------------------------------------------------------------------------
  # The spec file lives alongside the mix project. We resolve it relative to
  # the project root so that it works both during development (mix run) and
  # when running tests. The `spec_path/0` function handles this lookup.

  @doc """
  Entry point for the escript. Receives `argv` as a list of strings.

  ## How It Works

  1. Locate the `pwd.json` spec file relative to the project root.
  2. Prepend the program name `"pwd"` to argv (CLI Builder expects
     `argv[0]` to be the program name, just like C's `argv`).
  3. Call `Parser.parse/2` to validate and parse the arguments.
  4. Pattern-match on the result:
     - `HelpResult` => print help text and exit 0
     - `VersionResult` => print version and exit 0
     - `ParseResult` => check flags and print the appropriate path
     - `ParseErrors` => print errors to stderr and exit 1
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    # ---------------------------------------------------------------------------
    # Step 1: Parse arguments via CLI Builder
    # ---------------------------------------------------------------------------
    # We prepend "pwd" as the program name. CLI Builder expects the full argv
    # including the program name at position 0, mirroring how C programs and
    # shell utilities receive their arguments.

    case Parser.parse(spec_path, ["pwd" | argv]) do
      # -----------------------------------------------------------------------
      # Step 2a: Help was requested
      # -----------------------------------------------------------------------
      # The user passed --help or -h. CLI Builder has already generated the
      # full help text from the spec, including flag descriptions, usage
      # patterns, and the program description. We just print it.

      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      # -----------------------------------------------------------------------
      # Step 2b: Version was requested
      # -----------------------------------------------------------------------
      # The user passed --version. The version string comes from the spec file.

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      # -----------------------------------------------------------------------
      # Step 2c: Normal invocation — run the business logic
      # -----------------------------------------------------------------------
      # This is the *only* part specific to pwd. CLI Builder has already
      # validated the flags, enforced mutual exclusivity of -L and -P, and
      # populated the flags map. We just check which mode was requested.

      {:ok, %ParseResult{flags: flags}} ->
        if flags["physical"] do
          IO.puts(get_physical_pwd())
        else
          IO.puts(get_logical_pwd())
        end

      # -----------------------------------------------------------------------
      # Step 2d: Parse errors
      # -----------------------------------------------------------------------
      # CLI Builder collects all errors (unknown flags, conflicting flags,
      # etc.) and returns them together. We print each one to stderr and
      # exit with code 1, matching standard Unix conventions.

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "pwd: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business logic: logical path
  # ---------------------------------------------------------------------------

  @doc """
  Return the logical working directory.

  The logical path comes from the `$PWD` environment variable, which the
  shell maintains as the user navigates — including through symlinks.

  If `$PWD` is not set or is stale (doesn't match the real cwd), we fall
  back to the physical path. This matches POSIX behavior: the logical path
  is best-effort, never wrong.

  ## Why Validate $PWD?

  The `$PWD` variable could be stale if:
  - The directory was moved or deleted after the shell set it
  - The process changed directories without updating `$PWD`
  - A parent process set `$PWD` incorrectly

  So we resolve both `$PWD` and the actual cwd to their physical paths
  and compare. If they match, `$PWD` is trustworthy and we return it.
  Otherwise, we fall back to the physical path.
  """
  def get_logical_pwd do
    case System.get_env("PWD") do
      nil ->
        # $PWD is not set — fall back to physical path.
        get_physical_pwd()

      env_pwd ->
        # Verify that $PWD actually points to the current directory.
        # We resolve both paths to their canonical forms and compare.
        physical = get_physical_pwd()

        case resolve_realpath(env_pwd) do
          {:ok, resolved_env} when resolved_env == physical ->
            # $PWD matches the real cwd — return the logical path.
            env_pwd

          _ ->
            # $PWD is stale or invalid — fall back to physical path.
            physical
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Business logic: physical path
  # ---------------------------------------------------------------------------

  @doc """
  Return the physical working directory with all symlinks resolved.

  `File.cwd!/0` returns the current working directory. On most systems
  this already resolves symlinks (the kernel tracks the physical path).
  For full correctness, we also run the result through `realpath` to
  resolve any remaining symlinks in the path components.
  """
  def get_physical_pwd do
    cwd = File.cwd!()

    case resolve_realpath(cwd) do
      {:ok, resolved} -> resolved
      _ -> cwd
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    # During development and testing, the spec file is at the project root.
    # We use the Mix project directory to locate it reliably.
    #
    # For escript builds, Mix may not be available, so we also try the
    # current working directory as a fallback, and finally the directory
    # containing this beam file.

    candidates = [
      # When running under Mix (development / test)
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "pwd.json"),
        else: nil
      ),
      # Relative to current directory (works when run from project root)
      "pwd.json",
      # Alongside the escript
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "pwd.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find pwd.json spec file"
  end

  @doc """
  Resolve a path to its canonical form by following all symlinks.

  We shell out to the `realpath` command because Erlang/Elixir do not
  provide a built-in function to fully resolve symlinks in a path.
  `:file.read_link/1` only resolves one level; `realpath` handles the
  full chain.

  Returns `{:ok, resolved_path}` on success, `:error` on failure.
  """
  def resolve_realpath(path) do
    case System.cmd("realpath", [path], stderr_to_stdout: true) do
      {resolved, 0} -> {:ok, String.trim(resolved)}
      _ -> :error
    end
  end
end
