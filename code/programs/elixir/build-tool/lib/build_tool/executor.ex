defmodule BuildTool.Executor do
  @moduledoc """
  Runs BUILD commands for packages that need rebuilding.

  ## Parallel execution by levels

  The key insight of the build system is that not all packages depend on
  each other. The dependency graph can be partitioned into "levels" where
  packages within the same level have no dependencies on each other. These
  can safely run in parallel.

  For example, in a diamond dependency graph A->B, A->C, B->D, C->D:

      Level 0: [A]     — no dependencies, build first
      Level 1: [B, C]  — depend only on A, can run in parallel
      Level 2: [D]     — depends on B and C, build last

  ## Elixir's concurrency advantage

  This is where Elixir shines. We use `Task.async_stream` with
  `max_concurrency` to limit parallel workers — similar to Go's goroutine +
  semaphore pattern, but with the BEAM's preemptive scheduler. Each build
  runs in its own lightweight process (~2KB initial heap), and the BEAM
  handles scheduling across CPU cores automatically.

  The pattern: for each level, launch tasks for all packages that need
  building. `Task.async_stream` handles concurrency limiting. We collect
  results and check for failures before moving to the next level.

  ## Failure propagation

  If a package fails, all its transitive dependents are marked "dep-skipped".
  There is no point building something whose dependency is broken.

  ## Progress tracking

  The executor accepts an optional progress tracker pid that receives events
  as packages are skipped, started, and finished. This powers a real-time
  progress bar in the terminal. The tracker is nil-safe — all calls are
  no-ops when the pid is nil.

  ## BuildResult

  Each package build produces a result map:

      %{
        package_name: "python/logic-gates",
        status: "built",        # or "failed", "skipped", "dep-skipped", "would-build"
        duration: 2.3,          # seconds
        stdout: "...",          # combined stdout from BUILD commands
        stderr: "...",          # combined stderr from BUILD commands
        return_code: 0          # exit code of the last failing command, or 0
      }
  """

  alias BuildTool.DirectedGraph
  alias BuildTool.Cache
  alias CodingAdventures.ProgressBar

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Executes builds for packages respecting dependency order.

  This is the main orchestrator. It:

    1. Gets `independent_groups` from the dependency graph
    2. For each level, determines which packages need building
    3. Skips packages whose deps failed ("dep-skipped")
    4. Skips packages not in the affected set (git-diff mode)
    5. Skips packages whose hashes haven't changed (cache fallback)
    6. In dry-run mode, marks packages as "would-build"
    7. Otherwise, launches tasks with concurrency-limited parallelism
    8. Updates the cache after each build
    9. Sends progress events to the tracker (if non-nil)

  ## Parameters

    - `packages` — list of package maps from discovery
    - `graph` — the dependency graph from the resolver
    - `cache` — the cache agent pid
    - `pkg_hashes` — map of package name to source hash
    - `deps_hashes` — map of package name to dependency hash
    - `opts` — keyword list with:
      - `:force` — rebuild everything (default: false)
      - `:dry_run` — show what would build (default: false)
      - `:max_jobs` — max parallel jobs (default: System.schedulers_online())
      - `:affected_set` — MapSet of affected package names, or nil
      - `:tracker` — progress bar tracker pid, or nil

  ## Returns

  A map from package name to result map.
  """
  def execute_builds(packages, graph, cache, pkg_hashes, deps_hashes, opts \\ []) do
    force = Keyword.get(opts, :force, false)
    dry_run = Keyword.get(opts, :dry_run, false)
    max_jobs = Keyword.get(opts, :max_jobs, System.schedulers_online())
    affected_set = Keyword.get(opts, :affected_set)
    tracker = Keyword.get(opts, :tracker)

    # Build a lookup from name to package for quick access.
    pkg_by_name = Map.new(packages, fn p -> {p.name, p} end)

    # Get the parallel execution levels from the dependency graph.
    case DirectedGraph.independent_groups(graph) do
      {:error, :cycle} ->
        # Cycle detected — return an error result for all packages.
        Map.new(packages, fn pkg ->
          {pkg.name,
           %{
             package_name: pkg.name,
             status: "failed",
             duration: 0.0,
             stdout: "",
             stderr: "cycle detected in dependency graph",
             return_code: 1
           }}
        end)

      {:ok, groups} ->
        # Process each level sequentially; within each level, build in parallel.
        {results, _failed} =
          Enum.reduce(groups, {%{}, MapSet.new()}, fn level, {results, failed_packages} ->
            process_level(
              level,
              pkg_by_name,
              graph,
              cache,
              pkg_hashes,
              deps_hashes,
              results,
              failed_packages,
              force,
              dry_run,
              max_jobs,
              affected_set,
              tracker
            )
          end)

        results
    end
  end

  # ---------------------------------------------------------------------------
  # Level processing
  # ---------------------------------------------------------------------------
  #
  # For each level in the dependency graph, we determine which packages
  # need building, which can be skipped, and which are dep-skipped due
  # to upstream failures. Packages that need building are executed in
  # parallel using Task.async_stream.

  defp process_level(
         level,
         pkg_by_name,
         graph,
         cache,
         pkg_hashes,
         deps_hashes,
         results,
         failed_packages,
         force,
         dry_run,
         max_jobs,
         affected_set,
         tracker
       ) do
    # Phase 1: classify each package in this level.
    {to_build, new_results, new_failed} =
      Enum.reduce(level, {[], results, failed_packages}, fn name, {build_list, res, failed} ->
        pkg = Map.get(pkg_by_name, name)

        cond do
          pkg == nil ->
            {build_list, res, failed}

          # Check if any transitive dependency of this package has failed.
          dep_failed?(name, graph, failed) ->
            result = %{
              package_name: name,
              status: "dep-skipped",
              duration: 0.0,
              stdout: "",
              stderr: "",
              return_code: 0
            }

            ProgressBar.send_event(tracker, :skipped, name)
            {build_list, Map.put(res, name, result), failed}

          # Check if the package is in the affected set (git-diff mode).
          affected_set != nil and not MapSet.member?(affected_set, name) ->
            result = %{
              package_name: name,
              status: "skipped",
              duration: 0.0,
              stdout: "",
              stderr: "",
              return_code: 0
            }

            ProgressBar.send_event(tracker, :skipped, name)
            {build_list, Map.put(res, name, result), failed}

          # Check if the package needs building (cache fallback).
          affected_set == nil and not force and
              not Cache.needs_build?(cache, name, pkg_hashes[name], deps_hashes[name]) ->
            result = %{
              package_name: name,
              status: "skipped",
              duration: 0.0,
              stdout: "",
              stderr: "",
              return_code: 0
            }

            ProgressBar.send_event(tracker, :skipped, name)
            {build_list, Map.put(res, name, result), failed}

          # Dry-run mode: don't actually build.
          dry_run ->
            result = %{
              package_name: name,
              status: "would-build",
              duration: 0.0,
              stdout: "",
              stderr: "",
              return_code: 0
            }

            ProgressBar.send_event(tracker, :skipped, name)
            {build_list, Map.put(res, name, result), failed}

          # Package needs building.
          true ->
            {[pkg | build_list], res, failed}
        end
      end)

    if to_build == [] or dry_run do
      {new_results, new_failed}
    else
      # Phase 2: execute builds in parallel using Task.async_stream.
      #
      # Task.async_stream runs a function for each element in the enumerable,
      # limiting concurrency to max_concurrency. It returns results in order.
      # This is equivalent to Go's goroutine + semaphore pattern.
      workers = if max_jobs > 0, do: max_jobs, else: min(length(to_build), 8)

      build_results =
        to_build
        |> Task.async_stream(
          fn pkg ->
            ProgressBar.send_event(tracker, :started, pkg.name)
            result = run_package_build(pkg)
            ProgressBar.send_event(tracker, :finished, pkg.name, result.status)
            result
          end,
          max_concurrency: workers,
          timeout: :infinity,
          ordered: false
        )
        |> Enum.map(fn {:ok, result} -> result end)

      # Phase 3: record results and update cache.
      Enum.reduce(build_results, {new_results, new_failed}, fn result, {res, failed} ->
        new_res = Map.put(res, result.package_name, result)

        case result.status do
          "built" ->
            Cache.record(
              cache,
              result.package_name,
              pkg_hashes[result.package_name],
              deps_hashes[result.package_name],
              "success"
            )

            {new_res, failed}

          "failed" ->
            Cache.record(
              cache,
              result.package_name,
              pkg_hashes[result.package_name],
              deps_hashes[result.package_name],
              "failed"
            )

            {new_res, MapSet.put(failed, result.package_name)}

          _ ->
            {new_res, failed}
        end
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Single package build
  # ---------------------------------------------------------------------------
  #
  # runPackageBuild executes all BUILD commands for a single package.
  #
  # Commands are run sequentially — each must succeed before the next starts.
  # This is because BUILD files are scripts: later commands may depend on
  # earlier ones (e.g., "install dependencies" before "run tests").
  #
  # We use System.cmd("sh", ["-c", command]) so that BUILD commands can
  # use shell features like pipes, redirects, and environment variables.

  defp run_package_build(pkg) do
    start = System.monotonic_time(:millisecond)

    # We use stderr_to_stdout: true so that System.cmd captures both streams.
    # This matches the Go implementation which captures both stdout and stderr
    # from the shell command.
    #
    # On Windows, we use "cmd /C" instead of "sh -c" to invoke the shell.
    # This is the same approach used by the Rust build tool. Python's
    # subprocess.run(shell=True) and Node's child_process.exec() handle this
    # automatically, but Elixir's System.cmd requires explicit selection.
    {shell, shell_flag} = shell_command()

    {all_output, final_status, final_code} =
      Enum.reduce_while(pkg.build_commands, {[], "built", 0}, fn command,
                                                                  {output_acc, _status, _code} ->
        case System.cmd(shell, [shell_flag, command],
               cd: pkg.path,
               stderr_to_stdout: true
             ) do
          {output, 0} ->
            {:cont, {[output | output_acc], "built", 0}}

          {output, exit_code} ->
            {:halt, {[output | output_acc], "failed", exit_code}}
        end
      end)

    elapsed = (System.monotonic_time(:millisecond) - start) / 1000.0

    combined_output = all_output |> Enum.reverse() |> Enum.join("")

    %{
      package_name: pkg.name,
      status: final_status,
      duration: elapsed,
      stdout: combined_output,
      stderr: "",
      return_code: final_code
    }
  end

  # ---------------------------------------------------------------------------
  # Dependency failure detection
  # ---------------------------------------------------------------------------
  #
  # Checks if any transitive predecessor (dependency) of the given package
  # has failed. If so, this package should be dep-skipped.

  defp dep_failed?(name, graph, failed_packages) do
    preds = DirectedGraph.transitive_predecessors(graph, name)

    preds
    |> MapSet.to_list()
    |> Enum.any?(fn dep -> MapSet.member?(failed_packages, dep) end)
  end

  # ---------------------------------------------------------------------------
  # Shell command selection
  # ---------------------------------------------------------------------------
  #
  # Returns the platform-appropriate shell and flag for executing BUILD
  # commands. On Windows, this is {"cmd", "/C"}; on Unix, {"sh", "-c"}.
  #
  # This is the same approach used by the Rust build tool (which uses
  # cfg!(target_os = "windows") to select between cmd and sh). Python's
  # subprocess.run(shell=True) and Node's child_process.exec() handle this
  # automatically, but Elixir's System.cmd requires explicit selection.

  defp shell_command do
    shell_command_for_os(:os.type())
  end

  @doc false
  def shell_command_for_os({:win32, _}), do: {"cmd", "/C"}
  def shell_command_for_os(_), do: {"sh", "-c"}
end
