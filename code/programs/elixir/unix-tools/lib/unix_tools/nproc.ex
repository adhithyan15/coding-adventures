defmodule UnixTools.Nproc do
  @moduledoc """
  nproc -- print the number of processing units available.

  ## What This Program Does

  This is a reimplementation of the GNU `nproc` utility in Elixir. It prints
  the number of processing units (CPU cores) available to the current process.

  ## How nproc Works

      $ nproc             =>   8
      $ nproc --all       =>   8
      $ nproc --ignore=2  =>   6

  ## Available vs All Processors

  By default, `nproc` reports the number of processors *available* to the
  current process. On most systems this equals the total number of installed
  processors, but it can be less if:

  - CPU affinity has been set (e.g., `taskset` on Linux)
  - The process is running in a container with CPU limits
  - cgroups restrict the available CPUs

  The `--all` flag reports the total number of installed processors,
  ignoring any restrictions.

  ## The --ignore Flag

  `--ignore=N` subtracts N from the result, but the output is never less
  than 1. This is useful for leaving some cores free:

      make -j$(nproc --ignore=2)   =>   build with 2 fewer cores

  ## Erlang/Elixir Implementation

  Elixir runs on the BEAM virtual machine, which provides excellent
  introspection of the runtime environment:

  - `System.schedulers_online()` returns the number of schedulers
    currently online, which typically equals the number of available
    CPU cores.
  - `:erlang.system_info(:logical_processors)` returns the total number
    of logical processors detected by the runtime.

  These map naturally to nproc's default and `--all` behaviors.
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
  3. Determine CPU count based on --all flag.
  4. Subtract --ignore value if specified.
  5. Print the result (minimum 1).
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["nproc" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags}} ->
        # -----------------------------------------------------------------------
        # Business logic: count processors and apply flags.
        # -----------------------------------------------------------------------

        use_all = !!flags["all"]
        ignore_count = flags["ignore"] || 0

        count = get_cpu_count(use_all)
        result = compute_result(count, ignore_count)

        IO.puts(Integer.to_string(result))

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "nproc: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get the number of CPUs.

  When `use_all` is `true`, returns the total number of logical processors
  installed on the system. When `false`, returns the number of schedulers
  currently online (available to this process).

  ## How the BEAM Tracks CPUs

  The BEAM (Erlang VM) detects CPU topology at startup. It creates one
  scheduler per available CPU core by default. The number of online
  schedulers can be changed at runtime (e.g., for testing), but typically
  matches the number of available cores.

  - `System.schedulers_online/0` — available schedulers (≈ available cores)
  - `:erlang.system_info(:logical_processors)` — total logical CPUs

  ## Parameters

  - `use_all` — if `true`, return total installed processors; if `false`,
    return available processors
  """
  def get_cpu_count(use_all) do
    if use_all do
      # Total logical processors installed on the system.
      # This function returns :unknown on some platforms, so we fall back
      # to schedulers_online if that happens.
      case :erlang.system_info(:logical_processors) do
        count when is_integer(count) and count > 0 -> count
        _ -> System.schedulers_online()
      end
    else
      # Processors available to this process.
      System.schedulers_online()
    end
  end

  @doc """
  Compute the final result after applying --ignore.

  Subtracts `ignore_count` from `count`, but never returns less than 1.
  This ensures the output is always a valid number of processors — you
  can't have zero or negative processors.

  ## Examples

      iex> UnixTools.Nproc.compute_result(8, 0)
      8

      iex> UnixTools.Nproc.compute_result(8, 2)
      6

      iex> UnixTools.Nproc.compute_result(8, 10)
      1

      iex> UnixTools.Nproc.compute_result(1, 1)
      1
  """
  def compute_result(count, ignore_count) do
    max(1, count - ignore_count)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "nproc.json"),
        else: nil
      ),
      "nproc.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "nproc.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find nproc.json spec file"
  end
end
