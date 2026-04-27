#!/usr/bin/env ruby
# frozen_string_literal: true

# build.rb -- Command-Line Interface
# ===================================
#
# This is the entry point for the build tool CLI. It ties together all the
# modules: discovery, resolution, hashing, caching, execution, and reporting.
#
# Usage:
#
#     ruby build.rb                        # Auto-detect root, build changed
#     ruby build.rb --root /path/to/repo   # Specify root explicitly
#     ruby build.rb --force                # Rebuild everything
#     ruby build.rb --dry-run              # Show what would build
#     ruby build.rb --jobs 4               # Limit parallel workers
#     ruby build.rb --language python       # Only build Python packages
#     ruby build.rb --diff-base origin/main # Git ref for change detection
#     ruby build.rb --detect-languages     # Output needed language toolchains
#     ruby build.rb --emit-plan plan.json  # Write build plan and exit
#     ruby build.rb --plan-file plan.json  # Read plan, skip discovery
#
# The flow is:
#   1. Discover packages (walk recursive BUILD files)
#   2. Evaluate Starlark BUILD files
#   3. Filter by language if specified
#   4. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, etc.)
#   5. Git-diff change detection (default mode)
#   6. Emit plan or detect languages (early exit modes)
#   7. Hash all packages
#   8. Load cache, determine what needs building
#   9. Execute builds (parallel by level)
#  10. Update and save cache
#  11. Print report
#  12. Exit with code 1 if any builds failed

require "optparse"
require "pathname"
require "set"

# Optional dependency: progress bar for build output.
# If the gem is not installed, the build tool works fine without it.
begin
  require "coding_adventures_progress_bar"
rescue LoadError
  # Progress bar is optional — builds work without it.
end

# Load all build tool modules. We use require_relative so the tool works
# as a standalone script without needing to be installed as a gem.
require_relative "lib/build_tool/discovery"
require_relative "lib/build_tool/resolver"
require_relative "lib/build_tool/glob_match"
require_relative "lib/build_tool/hasher"
require_relative "lib/build_tool/cache"
require_relative "lib/build_tool/executor"
require_relative "lib/build_tool/reporter"
require_relative "lib/build_tool/starlark_evaluator"
require_relative "lib/build_tool/git_diff"
require_relative "lib/build_tool/ci_workflow"
require_relative "lib/build_tool/plan"
require_relative "lib/build_tool/validator"

module BuildTool
  module CLI
    module_function

    # ALL_LANGUAGES is the canonical list of package languages this CLI builds.
    ALL_LANGUAGES = %w[
      python ruby go typescript rust elixir lua perl swift java kotlin
      haskell wasm csharp fsharp dotnet
    ].freeze

    # ALL_TOOLCHAINS is the canonical list of CI toolchains we can request.
    ALL_TOOLCHAINS = %w[
      python ruby go typescript rust elixir lua perl swift java kotlin haskell dotnet
    ].freeze

    # SHARED_PREFIXES are repo paths that, when changed, still mean every
    # toolchain needs rebuilding. ci.yml is handled separately via patch
    # analysis so toolchain-scoped edits do not fan out across the repo.
    SHARED_PREFIXES = [].freeze

    def toolchain_for_package_language(language)
      case language
      when "wasm" then "rust"
      when "csharp", "fsharp", "dotnet" then "dotnet"
      else language
      end
    end

    # find_repo_root -- Walk up from `start` (or cwd) looking for a `.git` dir.
    #
    # Returns the directory containing `.git`, or nil if not found. This is
    # how we auto-detect the monorepo root when --root is not specified.
    #
    # @param start [Pathname, nil] Starting directory (defaults to cwd).
    # @return [Pathname, nil]
    def find_repo_root(start = nil)
      current = Pathname(start || Dir.pwd).expand_path

      loop do
        return current if (current / ".git").exist?

        parent = current.parent
        return nil if parent == current # Reached filesystem root

        current = parent
      end
    end

    # expand_affected_set_with_prereqs -- Ensure all transitive prerequisites
    # of the currently affected packages are also scheduled.
    #
    # This matters on fresh CI runners: some package BUILD steps materialize
    # local dependency state (for example sibling TypeScript file: dependencies
    # under node_modules), and dependents may fail if those prerequisite
    # packages are skipped just because their own sources didn't change.
    #
    # @param graph [DirectedGraph] The dependency graph.
    # @param affected_set [Hash<String, Boolean>, nil] Packages from git diff.
    # @return [Hash<String, Boolean>, nil] Expanded set with all prereqs.
    def expand_affected_set_with_prereqs(graph, affected_set)
      return affected_set if graph.nil? || affected_set.nil?

      expanded = affected_set.dup
      queue = affected_set.keys.dup

      until queue.empty?
        current = queue.shift
        graph.predecessors(current).each do |pred|
          next if expanded.key?(pred)

          expanded[pred] = true
          queue << pred
        end
      end

      expanded
    end

    # main -- Main entry point for the build tool CLI.
    #
    # Parses command-line arguments using OptionParser (Ruby's built-in
    # equivalent of Python's argparse), then orchestrates the full pipeline:
    # discover -> starlark eval -> resolve -> git diff -> emit plan ->
    # hash -> cache -> execute -> report.
    #
    # @param argv [Array<String>] Command-line arguments.
    # @return [Integer] Exit code: 0 for success, 1 if any builds failed.
    def main(argv = ARGV)
      # -- Parse command-line options -------------------------------------------
      options = {
        root: nil,
        force: false,
        dry_run: false,
        jobs: nil,
        language: "all",
        diff_base: "origin/main",
        cache_file: Pathname(".build-cache.json"),
        detect_languages: false,
        emit_plan: nil,
        plan_file: nil,
        validate_build_files: false
      }

      parser = OptionParser.new do |opts|
        opts.banner = "Usage: ruby build.rb [options]"
        opts.separator ""
        opts.separator "Incremental, parallel monorepo build tool"
        opts.separator ""

        opts.on("--root DIR", "Repo root directory (auto-detect from .git if not given)") do |dir|
          options[:root] = Pathname(dir)
        end

        opts.on("--force", "Rebuild everything regardless of cache") do
          options[:force] = true
        end

        opts.on("--dry-run", "Show what would build without actually building") do
          options[:dry_run] = true
        end

        opts.on("--jobs N", Integer, "Maximum number of parallel build jobs") do |n|
          options[:jobs] = n
        end

        opts.on("--language LANG",
                "Only build packages of this language (#{ALL_LANGUAGES.join('/')}/all)") do |lang|
          options[:language] = lang
        end

        opts.on("--diff-base REF", "Git ref to diff against for change detection (default: origin/main)") do |ref|
          options[:diff_base] = ref
        end

        opts.on("--cache-file FILE", "Path to the build cache file") do |file|
          options[:cache_file] = Pathname(file)
        end

        opts.on("--detect-languages",
                "Output which language toolchains are needed based on git diff, then exit") do
          options[:detect_languages] = true
        end

        opts.on("--emit-plan FILE", "Write build plan to FILE and exit (no build)") do |file|
          options[:emit_plan] = Pathname(file)
        end

        opts.on("--plan-file FILE", "Read build plan from FILE instead of discovering") do |file|
          options[:plan_file] = Pathname(file)
        end

        opts.on("--validate-build-files", "Validate BUILD/CI metadata contracts before continuing") do
          options[:validate_build_files] = true
        end

        opts.on("-h", "--help", "Show this help message") do
          puts opts
          return 0
        end
      end

      parser.parse!(argv)

      # -- Step 1: Find repo root -----------------------------------------------
      root = options[:root]
      if root.nil?
        root = find_repo_root
        if root.nil?
          $stderr.puts "Error: Could not find repo root (.git directory)."
          $stderr.puts "Use --root to specify the repo root."
          return 1
        end
      end

      root = root.expand_path

      # ── Plan-based execution path ─────────────────────────────────────────
      #
      # When --plan-file is set, we skip the expensive discovery/resolution/
      # git-diff steps and reconstruct state from a pre-computed plan.
      # This is used by CI build jobs that receive a plan artifact from the
      # detect job.
      if options[:plan_file]
        return run_from_plan(options, root)
      end

      # The build starts from the code/ directory.
      code_root = root / "code"
      unless code_root.exist?
        $stderr.puts "Error: #{code_root} does not exist."
        return 1
      end

      # -- Step 2: Discover packages --------------------------------------------
      packages = Discovery.discover_packages(code_root)

      if packages.empty?
        $stderr.puts "No packages found."
        return 0
      end

      # -- Step 2b: Evaluate Starlark BUILD files --------------------------------
      #
      # For each discovered package, check if its BUILD file is Starlark.
      # If so, evaluate it through the Ruby starlark_interpreter to extract
      # declared targets (with srcs, deps, build commands). This replaces
      # the raw shell command lines with generated commands from the rule.
      starlark_count = 0
      packages = packages.map do |pkg|
        next pkg unless StarlarkEvaluator.starlark_build?(pkg.build_content)

        pkg = pkg.with(is_starlark: true)
        build_file = pkg.path / "BUILD"
        result = begin
          StarlarkEvaluator.evaluate_build_file(build_file.to_s, pkg.path.to_s, root.to_s)
        rescue StandardError => e
          $stderr.puts "Warning: Starlark eval failed for #{pkg.name}: #{e.message}"
          nil
        end

        if result && result.targets.any?
          t = result.targets.first
          pkg = pkg.with(
            declared_srcs: t.srcs,
            declared_deps: t.deps,
            build_commands: StarlarkEvaluator.generate_commands(t)
          )
          starlark_count += 1
        end

        pkg
      end

      puts "Evaluated #{starlark_count} Starlark BUILD files" if starlark_count > 0

      # -- Step 3: Filter by language -------------------------------------------
      if options[:language] != "all"
        packages = packages.select { |p| p.language == options[:language] }
        if packages.empty?
          $stderr.puts "No #{options[:language]} packages found."
          return 0
        end
      end

      if options[:validate_build_files]
        validation_error = Validator.validate_build_contracts(root, packages)
        if validation_error
          $stderr.puts "BUILD/CI validation failed:"
          $stderr.puts "  - #{validation_error}"
          $stderr.puts "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct."
          return 1
        end
      end

      puts "Discovered #{packages.size} packages"

      # -- Step 4: Resolve dependencies -----------------------------------------
      graph = Resolver.resolve_dependencies(packages)

      # -- Step 5: Git-diff change detection (default mode) ---------------------
      #
      # Git is the source of truth — no cache file needed for primary workflow.
      # Fallback: hash-based cache (when git diff is unavailable).
      affected_set = nil
      ci_toolchains = Set.new

      unless options[:force]
        changed_files = GitDiff.get_changed_files(root.to_s, options[:diff_base])
        if changed_files && !changed_files.empty?
          if changed_files.include?(CIWorkflow::CI_WORKFLOW_PATH)
            ci_change = CIWorkflow.analyze_changes(root, options[:diff_base])
            if ci_change.requires_full_rebuild
              puts "Git diff: ci.yml changed in shared ways — rebuilding everything"
              options[:force] = true
              affected_set = nil
            else
              ci_toolchains = ci_change.toolchains
              unless ci_toolchains.empty?
                puts "Git diff: ci.yml changed only toolchain-scoped setup for " \
                     "#{CIWorkflow.sorted_toolchains(ci_toolchains).join(', ')}"
              end
            end
          end

          shared_changed = changed_files.any? do |f|
            next false if f == CIWorkflow::CI_WORKFLOW_PATH
            SHARED_PREFIXES.any? { |prefix| f == prefix || f.start_with?("#{prefix}/") }
          end

          if shared_changed
            puts "Git diff: shared files changed — rebuilding everything"
            options[:force] = true
            affected_set = nil
          else
            changed_pkgs = GitDiff.map_files_to_packages(changed_files, packages, root)
            if changed_pkgs.any?
              # Mark all transitive dependents (packages that depend on changed ones).
              affected_set = {}
              changed_pkgs.each_key do |name|
                affected_set[name] = true
                graph.transitive_dependents(name).each { |dep| affected_set[dep] = true }
              end
              affected_set = expand_affected_set_with_prereqs(graph, affected_set)
              puts "Git diff: #{changed_pkgs.size} packages changed, " \
                   "#{affected_set.size} affected (including dependents and prerequisites)"
            else
              puts "Git diff: no package files changed — nothing to build"
              affected_set = {}
            end
          end
        elsif changed_files
          puts "Git diff: no files changed — nothing to build"
          affected_set = {}
        else
          puts "Git diff unavailable — falling back to hash-based cache"
        end
      end

      # -- Step 6a: Emit plan (early exit) --------------------------------------
      if options[:emit_plan]
        return emit_build_plan(options, root, packages, graph, affected_set, ci_toolchains)
      end

      # -- Step 6b: Detect languages (early exit) -------------------------------
      if options[:detect_languages]
        return detect_needed_languages(packages, affected_set, options[:force], ci_toolchains)
      end

      # -- Step 7: Hash all packages --------------------------------------------
      package_hashes = {}
      deps_hashes = {}

      packages.each do |pkg|
        package_hashes[pkg.name] = Hasher.hash_package(pkg)
        deps_hashes[pkg.name] = Hasher.hash_deps(pkg.name, graph, package_hashes)
      end

      # -- Step 8: Load cache ---------------------------------------------------
      cache_path = options[:cache_file]
      cache_path = root / cache_path unless cache_path.absolute?

      cache = BuildCache.new
      cache.load(cache_path)

      # -- Steps 9-10: Execute builds -------------------------------------------

      # Create a progress bar tracker unless we're in dry-run mode.
      tracker = nil
      unless options[:dry_run]
        begin
          tracker = CodingAdventures::ProgressBar::Tracker.new(packages.size, $stderr, "")
          tracker.start
        rescue NameError
          # Progress bar not available
        end
      end

      results = Executor.execute_builds(
        packages: packages,
        graph: graph,
        cache: cache,
        package_hashes: package_hashes,
        deps_hashes: deps_hashes,
        force: options[:force],
        dry_run: options[:dry_run],
        max_jobs: options[:jobs],
        tracker: tracker,
        affected_set: affected_set
      )

      # Shut down the progress bar renderer thread.
      tracker&.stop

      # -- Step 11: Save cache (unless dry run) ---------------------------------
      cache.save(cache_path) unless options[:dry_run]

      # -- Step 12: Print report ------------------------------------------------
      Reporter.print_report(results)

      # -- Step 13: Exit code ---------------------------------------------------
      has_failures = results.values.any? { |r| r.status == "failed" }
      has_failures ? 1 : 0
    end

    # emit_build_plan -- Serialize the build plan to a JSON file and exit.
    #
    # This is the --emit-plan code path. It constructs a BuildPlan from the
    # discovery, resolution, and change detection results, writes it to the
    # specified path, and exits without building anything.
    #
    # If --detect-languages is also set, language flags are printed to stdout
    # after writing the plan.
    #
    # @param options [Hash] Parsed CLI options.
    # @param root [Pathname] Repository root directory.
    # @param packages [Array<Package>] Discovered packages.
    # @param graph [DirectedGraph] Dependency graph.
    # @param affected_set [Hash<String, Boolean>, nil] Affected packages.
    # @return [Integer] Exit code.
    def emit_build_plan(options, root, packages, graph, affected_set, ci_toolchains)
      # Convert affected_set to a sorted array (or nil for "all").
      affected_list = if affected_set
                        affected_set.keys.sort
                      end

      # Build PackageEntry list from discovered packages.
      package_entries = packages.map do |pkg|
        rel_path = begin
          pkg.path.relative_path_from(root).to_s.tr("\\", "/")
        rescue ArgumentError
          pkg.path.to_s.tr("\\", "/")
        end

        Plan::PackageEntry.new(
          name: pkg.name,
          rel_path: rel_path,
          language: pkg.language,
          build_commands: pkg.build_commands,
          is_starlark: pkg.respond_to?(:is_starlark) ? (pkg.is_starlark || false) : false,
          declared_srcs: pkg.respond_to?(:declared_srcs) ? (pkg.declared_srcs || []) : [],
          declared_deps: pkg.respond_to?(:declared_deps) ? (pkg.declared_deps || []) : []
        )
      end

      # Build dependency edges from the graph.
      edges = []
      graph.nodes.each do |node|
        graph.successors(node).each do |succ|
          edges << [node, succ]
        end
      end

      # Compute languages needed.
      languages_needed = compute_languages_needed(packages, affected_set, options[:force], ci_toolchains)

      bp = Plan::BuildPlan.new(
        diff_base: options[:diff_base],
        force: options[:force],
        affected_packages: affected_list,
        packages: package_entries,
        dependency_edges: edges,
        languages_needed: languages_needed
      )

      begin
        Plan.write_plan(bp, options[:emit_plan])
        puts "Build plan written to #{options[:emit_plan]} (#{packages.size} packages)"
      rescue StandardError => e
        $stderr.puts "Error writing build plan: #{e.message}"
        return 1
      end

      # If --detect-languages was also set, output language flags.
      if options[:detect_languages]
        output_language_flags(languages_needed)
      end

      0
    end

    # run_from_plan -- Load a build plan from file and run the build.
    #
    # This is the --plan-file code path. It reads a pre-computed plan,
    # reconstructs the packages and dependency graph, and runs the build
    # without re-doing discovery or change detection.
    #
    # On any error (missing file, invalid JSON, unsupported version), it
    # falls back to the normal discovery flow.
    #
    # @param options [Hash] Parsed CLI options.
    # @param root [Pathname] Repository root directory.
    # @return [Integer] Exit code.
    def run_from_plan(options, root)
      bp = begin
        Plan.read_plan(options[:plan_file])
      rescue StandardError => e
        $stderr.puts "Warning: could not read plan file #{options[:plan_file]}: #{e.message}"
        $stderr.puts "Falling back to normal discovery flow"
        options[:plan_file] = nil
        return main(rebuild_argv(options))
      end

      # Reconstruct Package objects from the plan entries.
      packages = bp.packages.map do |pe|
        pkg_path = root / pe.rel_path

        # Re-read the platform-appropriate BUILD file for non-Starlark packages.
        # Plan commands were generated on the detect job's OS and may use
        # shell syntax that differs on the current platform.
        build_commands = pe.build_commands
        unless pe.is_starlark
          platform_build = Discovery.get_build_file(pkg_path)
          if platform_build
            platform_cmds = Discovery.read_lines(platform_build)
            build_commands = platform_cmds if platform_cmds.any?
          end
        end

        Discovery::Package.new(
          name: pe.name,
          path: pkg_path,
          build_commands: build_commands,
          language: pe.language,
          build_content: "",
          is_starlark: pe.is_starlark,
          declared_srcs: pe.declared_srcs,
          declared_deps: pe.declared_deps
        )
      end

      # Filter by language if specified.
      if options[:language] != "all"
        packages = packages.select { |p| p.language == options[:language] }
        if packages.empty?
          $stderr.puts "No #{options[:language]} packages found in plan."
          return 0
        end
      end

      if options[:validate_build_files]
        validation_error = Validator.validate_build_contracts(root, packages)
        if validation_error
          $stderr.puts "BUILD/CI validation failed:"
          $stderr.puts "  - #{validation_error}"
          $stderr.puts "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct."
          return 1
        end
      end

      # Reconstruct the dependency graph from the plan's edges.
      graph = BuildTool::DirectedGraph.new
      packages.each { |pkg| graph.add_node(pkg.name) }
      bp.dependency_edges.each do |edge|
        from_node, to_node = edge
        graph.add_edge(from_node, to_node) if graph.has_node?(from_node) && graph.has_node?(to_node)
      end

      # Reconstruct the affected set.
      affected_set = nil
      if bp.affected_packages
        affected_set = bp.affected_packages.each_with_object({}) { |name, h| h[name] = true }
      end

      puts "Loaded plan: #{packages.size} packages from #{options[:plan_file]}"

      # From here, the flow is identical to the normal build path:
      # hash, cache, execute, report.
      package_hashes = {}
      deps_hashes = {}
      packages.each do |pkg|
        package_hashes[pkg.name] = Hasher.hash_package(pkg)
        deps_hashes[pkg.name] = Hasher.hash_deps(pkg.name, graph, package_hashes)
      end

      cache_path = options[:cache_file]
      cache_path = root / cache_path unless cache_path.absolute?

      cache = BuildCache.new
      cache.load(cache_path)

      tracker = nil
      unless options[:dry_run]
        begin
          tracker = CodingAdventures::ProgressBar::Tracker.new(packages.size, $stderr, "")
          tracker.start
        rescue NameError
          # Progress bar not available
        end
      end

      force = bp.force || options[:force]
      results = Executor.execute_builds(
        packages: packages,
        graph: graph,
        cache: cache,
        package_hashes: package_hashes,
        deps_hashes: deps_hashes,
        force: force,
        dry_run: options[:dry_run],
        max_jobs: options[:jobs],
        tracker: tracker,
        affected_set: affected_set
      )

      tracker&.stop
      cache.save(cache_path) unless options[:dry_run]
      Reporter.print_report(results)

      has_failures = results.values.any? { |r| r.status == "failed" }
      has_failures ? 1 : 0
    end

    # detect_needed_languages -- Determine which language toolchains CI needs.
    #
    # Outputs one line per language in the format "needs_<lang>=true|false" to
    # both stdout and $GITHUB_OUTPUT (if the environment variable is set).
    #
    # Go is always needed because the build tool is written in Go.
    #
    # @param packages [Array<Package>] All discovered packages.
    # @param affected_set [Hash<String, Boolean>, nil] Affected packages.
    # @param force [Boolean] Whether --force was set.
    # @return [Integer] Exit code (always 0).
    def detect_needed_languages(packages, affected_set, force, ci_toolchains)
      languages_needed = CIWorkflow.compute_languages_needed(packages, affected_set, force, ci_toolchains)
      output_language_flags(languages_needed)
      0
    end

    # compute_languages_needed -- Determine which language toolchains are needed.
    #
    # Go is always needed (build tool is written in Go). If force mode or
    # shared files changed (affectedSet=nil), all languages are needed.
    #
    # @param packages [Array<Package>]
    # @param affected_set [Hash<String, Boolean>, nil]
    # @param force [Boolean]
    # @return [Hash<String, Boolean>]
    def compute_languages_needed(packages, affected_set, force, ci_toolchains = Set.new)
      CIWorkflow.compute_languages_needed(packages, affected_set, force, ci_toolchains)
    end

    # output_language_flags -- Print language flags to stdout and $GITHUB_OUTPUT.
    #
    # Output format: "needs_<lang>=true" or "needs_<lang>=false" per language.
    #
    # @param languages_needed [Hash<String, Boolean>]
    def output_language_flags(languages_needed)
      gh_output_path = ENV["GITHUB_OUTPUT"]
      gh_file = if gh_output_path && !gh_output_path.empty?
                  begin
                    File.open(gh_output_path, "a")
                  rescue StandardError => e
                    $stderr.puts "Warning: could not open $GITHUB_OUTPUT: #{e.message}"
                    nil
                  end
                end

      ALL_TOOLCHAINS.each do |lang|
        value = languages_needed.fetch(lang, false)
        line = "needs_#{lang}=#{value}"
        puts line
        gh_file&.puts(line)
      end

      gh_file&.close
    end

    # rebuild_argv -- Reconstruct argv from options hash (for fallback re-execution).
    #
    # When --plan-file fails, we need to re-run main() without the --plan-file
    # flag. This helper rebuilds the argument list.
    #
    # @param options [Hash] Parsed CLI options.
    # @return [Array<String>]
    def rebuild_argv(options)
      argv = []
      argv.push("--root", options[:root].to_s) if options[:root]
      argv << "--force" if options[:force]
      argv << "--dry-run" if options[:dry_run]
      argv.push("--jobs", options[:jobs].to_s) if options[:jobs]
      argv.push("--language", options[:language]) if options[:language] != "all"
      argv << "--validate-build-files" if options[:validate_build_files]
      argv.push("--diff-base", options[:diff_base])
      argv.push("--cache-file", options[:cache_file].to_s)
      argv << "--detect-languages" if options[:detect_languages]
      argv
    end
  end
end

# Run the CLI when invoked directly.
exit(BuildTool::CLI.main) if $PROGRAM_NAME == __FILE__
