defmodule BuildTool.CLI do
  @moduledoc """
  Main escript entry point for the Elixir build tool.

  This module parses command-line arguments and orchestrates the full build
  flow — the same 11 steps as the Go implementation. It is the "imperative
  shell" that wires together all the pure functional modules.

  ## The 11-step build flow

    1. Find the repo root (walk up looking for `.git`)
    2. Discover packages (walk BUILD files under `code/`)
    3. Filter by language if requested
    4. Resolve dependencies (parse metadata files)
    5. Git-diff change detection (`git diff --name-only <base>...HEAD`)
    6. Hash all packages and their dependencies
    7. Load cache (fallback when git diff is unavailable)
    8. If `--dry-run`, report what would build and exit
    9. Execute builds in parallel by dependency level
   10. Update and save cache
   11. Print report and exit with code 1 if any builds failed

  ## CLI flags

  The escript accepts the same flags as the Go build tool:

    - `--root PATH` — repo root directory (auto-detected from `.git`)
    - `--force` — rebuild everything regardless of cache
    - `--dry-run` — show what would build without executing
    - `--jobs N` — max parallel jobs (default: CPU count)
    - `--language LANG` — filter to a specific language
    - `--diff-base REF` — git ref to diff against (default: `origin/main`)
    - `--cache-file PATH` — path to cache file (default: `.build-cache.json`)
    - `--emit-plan PATH` — write a build plan JSON file and exit (no build)
    - `--plan-file PATH` — read a build plan instead of running discovery

  ## Usage

      mix escript.build
      ./build_tool --root /path/to/repo --force
  """

  alias BuildTool.{
    CIWorkflow,
    Cache,
    Discovery,
    DirectedGraph,
    Executor,
    GitDiff,
    Hasher,
    Plan,
    Reporter,
    Resolver,
    StarlarkEvaluator,
    Validator
  }

  alias CodingAdventures.ProgressBar

  @all_toolchains ["python", "ruby", "go", "typescript", "rust", "elixir", "lua", "perl", "swift", "haskell", "dotnet"]

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Main entry point for the escript. Parses arguments and runs the build.

  Exits with code 0 on success, 1 if any builds failed.
  """
  def main(argv) do
    exit_code = run(argv)
    System.halt(exit_code)
  end

  # ---------------------------------------------------------------------------
  # Core logic
  # ---------------------------------------------------------------------------
  #
  # The run/1 function contains the actual logic, separated from main/1
  # so we can return an exit code cleanly (and test it without halting).

  @doc false
  def run(argv) do
    # Parse CLI flags using OptionParser.
    {opts, _rest, _invalid} =
      OptionParser.parse(argv,
        strict: [
          root: :string,
          force: :boolean,
          dry_run: :boolean,
          jobs: :integer,
          language: :string,
          diff_base: :string,
          cache_file: :string,
          validate_build_files: :boolean,
          detect_languages: :boolean,
          emit_plan: :string,
          plan_file: :string
        ],
        aliases: [
          r: :root,
          f: :force,
          n: :dry_run,
          j: :jobs,
          l: :language
        ]
      )

    root = Keyword.get(opts, :root, "")
    force = Keyword.get(opts, :force, false)
    dry_run = Keyword.get(opts, :dry_run, false)
    jobs = Keyword.get(opts, :jobs, System.schedulers_online())
    language = Keyword.get(opts, :language, "all")
    diff_base = Keyword.get(opts, :diff_base, "origin/main")
    cache_file = Keyword.get(opts, :cache_file, ".build-cache.json")
    validate_build_files = Keyword.get(opts, :validate_build_files, false)
    detect_languages = Keyword.get(opts, :detect_languages, false)
    emit_plan_path = Keyword.get(opts, :emit_plan, nil)
    plan_file_path = Keyword.get(opts, :plan_file, nil)

    # Step 1: Find the repo root.
    repo_root =
      if root == "" do
        find_repo_root(File.cwd!())
      else
        Path.expand(root)
      end

    if repo_root == nil do
      IO.puts(:stderr, "Error: Could not find repo root (.git directory).")
      IO.puts(:stderr, "Use --root to specify the repo root.")
      1
    else
      do_build(
        repo_root,
        force,
        dry_run,
        jobs,
        language,
        diff_base,
        cache_file,
        validate_build_files,
        detect_languages, emit_plan_path, plan_file_path)
    end
  end

  defp do_build(repo_root, force, dry_run, jobs, language, diff_base, cache_file,
               validate_build_files, detect_languages, emit_plan_path, plan_file_path) do
    # The build starts from the code/ directory inside the repo root.
    code_root = Path.join(repo_root, "code")

    if not File.dir?(code_root) do
      IO.puts(:stderr, "Error: #{code_root} does not exist or is not a directory.")
      1
    else
      # Step 2: Discover packages.
      all_packages = Discovery.discover_packages(code_root)

      # Step 3: Filter by language if requested.
      packages =
        if language != "all" do
          Enum.filter(all_packages, fn pkg -> pkg.language == language end)
        else
          all_packages
        end

      cond do
        all_packages == [] ->
          IO.puts(:stderr, "No packages found.")
          0

        packages == [] ->
          IO.puts(:stderr, "No #{language} packages found.")
          0

        true ->
          # Step 3.5: Evaluate Starlark BUILD files.
          # For each package whose BUILD file is Starlark (not shell), we
          # evaluate it through the Starlark interpreter and replace the raw
          # shell commands with generated commands from the declared targets.
          packages = evaluate_starlark_builds(packages, repo_root)

          if validate_build_files do
            case Validator.validate_build_contracts(repo_root, packages) do
              nil ->
                do_build_packages(packages, repo_root, force, dry_run, jobs, diff_base,
                  cache_file, detect_languages, emit_plan_path, plan_file_path)

              validation_error ->
                IO.puts(:stderr, "BUILD/CI validation failed:")
                IO.puts(:stderr, "  - #{validation_error}")

                IO.puts(
                  :stderr,
                  "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct."
                )

                1
            end
          else
            do_build_packages(packages, repo_root, force, dry_run, jobs, diff_base,
              cache_file, detect_languages, emit_plan_path, plan_file_path)
          end
      end
    end
  end

  defp do_build_packages(packages, repo_root, force, dry_run, jobs, diff_base,
                         cache_file, detect_languages, emit_plan_path, _plan_file_path) do
    IO.puts("Discovered #{length(packages)} packages")

    # Step 4: Resolve dependencies.
    graph = Resolver.resolve_dependencies(packages)

    # Step 5: Git-diff change detection (default mode).
    {affected_set, force_override} =
      if force do
        {nil, force}
      else
        changed_files = GitDiff.get_changed_files(repo_root, diff_base)

        if length(changed_files) > 0 do
          ci_change =
            if Enum.member?(changed_files, CIWorkflow.ci_workflow_path()) do
              change = CIWorkflow.analyze_changes(repo_root, diff_base)

              if change.requires_full_rebuild do
                IO.puts("Git diff: ci.yml changed in shared ways — rebuilding everything")
              else
                toolchains = CIWorkflow.sorted_toolchains(change.toolchains)

                if toolchains != [] do
                  IO.puts(
                    "Git diff: ci.yml changed only toolchain-scoped setup for #{Enum.join(toolchains, ", ")}"
                  )
                end
              end

              change
            else
              %{toolchains: MapSet.new(), requires_full_rebuild: false}
            end

          if ci_change.requires_full_rebuild do
            {nil, true}
          else
            changed_pkgs = GitDiff.map_files_to_packages(changed_files, packages, repo_root)

            if MapSet.size(changed_pkgs) > 0 do
              affected = DirectedGraph.affected_nodes(graph, changed_pkgs)

              IO.puts(
                "Git diff: #{MapSet.size(changed_pkgs)} packages changed, #{MapSet.size(affected)} affected (including dependents)"
              )

              {affected, force}
            else
              IO.puts("Git diff: no package files changed — nothing to build")
              {MapSet.new(), force}
            end
          end
        else
          IO.puts("Git diff unavailable — falling back to hash-based cache")
          {nil, force}
        end
      end

    # --emit-plan: write the build plan to a file and exit without building.
    # This is used by CI to compute the plan in a fast "detect" job and
    # share it across build jobs on multiple platforms.
    cond do
      detect_languages ->
        languages_needed = compute_languages_needed(packages, affected_set, force_override)
        output_language_flags(languages_needed)

      emit_plan_path != nil ->
      return_emit_plan(packages, graph, repo_root, diff_base, force_override, affected_set,
        emit_plan_path)

      true ->
      do_execute_builds(packages, graph, repo_root, force_override, dry_run, jobs, diff_base,
        cache_file, affected_set)
    end
  end

  # ---------------------------------------------------------------------------
  # Build plan emission
  # ---------------------------------------------------------------------------
  #
  # When --emit-plan is specified, we serialize the discovery + change
  # detection results to a JSON file and exit. No builds are executed.

  defp return_emit_plan(
         packages,
         graph,
         repo_root,
         diff_base,
         force,
         affected_set,
         emit_plan_path
       ) do
    # Convert affected_set to a list (or nil for "rebuild all").
    affected_list =
      case affected_set do
        nil -> nil
        set -> MapSet.to_list(set) |> Enum.sort()
      end

    # Build package entries.
    pkg_entries =
      Enum.map(packages, fn pkg ->
        rel_path =
          Path.relative_to(pkg.path, repo_root)
          |> String.replace("\\", "/")

        %Plan.PackageEntry{
          name: pkg.name,
          rel_path: rel_path,
          language: pkg.language,
          build_commands: pkg.build_commands,
          is_starlark: Map.get(pkg, :is_starlark, false),
          declared_srcs: Map.get(pkg, :declared_srcs, []),
          declared_deps: Map.get(pkg, :declared_deps, [])
        }
      end)

    # Extract dependency edges from the graph's forward adjacency map.
    # Each entry in forward is {from_node, MapSet of successor nodes}.
    dep_edges =
      graph.forward
      |> Enum.flat_map(fn {from, successors} ->
        successors |> MapSet.to_list() |> Enum.map(fn to -> [from, to] end)
      end)
      |> Enum.sort()

    # Determine which languages are needed.
    langs_needed = compute_languages_needed(packages, affected_set, force)

    plan = %Plan{
      diff_base: diff_base,
      force: force,
      affected_packages: affected_list,
      packages: pkg_entries,
      dependency_edges: dep_edges,
      languages_needed: langs_needed
    }

    case Plan.write_plan(plan, emit_plan_path) do
      :ok ->
        IO.puts("Build plan written to #{emit_plan_path}")
        0

      {:error, reason} ->
        IO.puts(:stderr, "Error writing build plan: #{inspect(reason)}")
        1
    end
  end

  # ---------------------------------------------------------------------------
  # Build execution (normal flow)
  # ---------------------------------------------------------------------------

  defp do_execute_builds(
         packages,
         graph,
         repo_root,
         force,
         dry_run,
         jobs,
         _diff_base,
         cache_file,
         affected_set
       ) do
    # Step 6: Hash all packages.
    package_hashes = Map.new(packages, fn pkg -> {pkg.name, Hasher.hash_package(pkg)} end)

    deps_hashes =
      Map.new(packages, fn pkg ->
        {pkg.name, Hasher.hash_deps(pkg.name, graph, package_hashes)}
      end)

    # Step 7: Load cache (fallback if git diff didn't work).
    cache_path =
      if Path.type(cache_file) == :absolute do
        cache_file
      else
        Path.join(repo_root, cache_file)
      end

    {:ok, build_cache} = Cache.start_link()
    Cache.load(build_cache, cache_path)

    # Steps 8-9: Execute builds with progress tracking.
    tracker =
      if dry_run do
        nil
      else
        case ProgressBar.start_link(total: length(packages), writer: :stderr) do
          {:ok, pid} -> pid
          _ -> nil
        end
      end

    results =
      Executor.execute_builds(packages, graph, build_cache, package_hashes, deps_hashes,
        force: force,
        dry_run: dry_run,
        max_jobs: jobs,
        affected_set: affected_set,
        tracker: tracker
      )

    if tracker do
      ProgressBar.stop(tracker)
    end

    # Step 10: Save cache.
    if not dry_run do
      case Cache.save(build_cache, cache_path) do
        :ok -> :ok
        {:error, reason} -> IO.puts(:stderr, "Warning: could not save cache: #{inspect(reason)}")
      end
    end

    # Step 10: Print report.
    Reporter.print_report(results)

    # Step 11: Exit with code 1 if any builds failed.
    has_failure = results |> Map.values() |> Enum.any?(fn r -> r.status == "failed" end)
    if has_failure, do: 1, else: 0
  end

  # ---------------------------------------------------------------------------
  # Starlark BUILD evaluation
  # ---------------------------------------------------------------------------
  #
  # After discovering packages, we check each one's BUILD file content.
  # If it contains Starlark code (detected by looking for rule calls and
  # load() statements), we evaluate it through the Starlark interpreter
  # and replace the raw shell commands with generated commands.
  #
  # This mirrors the Go implementation's Starlark evaluation step in main.go.

  defp evaluate_starlark_builds(packages, repo_root) do
    {updated_packages, starlark_count} =
      Enum.map_reduce(packages, 0, fn pkg, count ->
        # Read the BUILD file content to check if it's Starlark.
        build_content = Enum.join(pkg.build_commands, "\n")

        if StarlarkEvaluator.starlark_build?(build_content) do
          build_file = Path.join(pkg.path, "BUILD")

          case StarlarkEvaluator.evaluate_build_file(build_file, pkg.path, repo_root) do
            {:ok, targets} when targets != [] ->
              # Use the first target's metadata (most BUILD files have one target).
              target = hd(targets)
              commands = StarlarkEvaluator.generate_commands(target)
              updated_pkg = %{pkg | build_commands: commands}
              {updated_pkg, count + 1}

            {:ok, _empty_targets} ->
              {pkg, count}

            {:error, reason} ->
              IO.puts(:stderr, "Warning: Starlark evaluation failed for #{pkg.name}: #{reason}")
              {pkg, count}
          end
        else
          {pkg, count}
        end
      end)

    if starlark_count > 0 do
      IO.puts("Evaluated #{starlark_count} Starlark BUILD file(s)")
    end

    updated_packages
  end

  # ---------------------------------------------------------------------------
  # Repo root detection
  # ---------------------------------------------------------------------------
  #
  # Walks up from the given directory looking for a .git directory.
  # Returns the directory containing .git, or nil if not found.

  @doc false
  def find_repo_root(start) do
    current = Path.expand(start)
    do_find_repo_root(current)
  end

  defp do_find_repo_root(current) do
    git_dir = Path.join(current, ".git")

    if File.dir?(git_dir) do
      current
    else
      parent = Path.dirname(current)

      if parent == current do
        # Reached filesystem root without finding .git.
        nil
      else
        do_find_repo_root(parent)
      end
    end
  end

  defp compute_languages_needed(_packages, _affected_set, true) do
    Map.new(@all_toolchains, fn toolchain -> {toolchain, true} end)
  end

  defp compute_languages_needed(_packages, nil, _force) do
    Map.new(@all_toolchains, fn toolchain -> {toolchain, true} end)
    |> Map.put("go", true)
  end

  defp compute_languages_needed(packages, affected_set, _force) do
    Enum.reduce(packages, Map.new(@all_toolchains, fn toolchain -> {toolchain, false} end), fn pkg, acc ->
      if MapSet.member?(affected_set, pkg.name) do
        Map.put(acc, toolchain_for_language(pkg.language), true)
      else
        acc
      end
    end)
    |> Map.put("go", true)
  end

  defp toolchain_for_language(language) do
    case language do
      "wasm" -> "rust"
      lang when lang in ["csharp", "fsharp", "dotnet"] -> "dotnet"
      _ -> language
    end
  end

  defp output_language_flags(languages_needed) do
    github_output = System.get_env("GITHUB_OUTPUT")

    Enum.each(@all_toolchains, fn toolchain ->
      value = Map.get(languages_needed, toolchain, false)
      line = "needs_#{toolchain}=#{if value, do: "true", else: "false"}"
      IO.puts(line)

      if github_output not in [nil, ""] do
        File.write!(github_output, line <> "\n", [:append])
      end
    end)

    0
  end

end
