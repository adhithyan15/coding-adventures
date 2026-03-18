# frozen_string_literal: true

# executor.rb -- Parallel Build Execution
# ========================================
#
# This module runs BUILD commands for packages that need rebuilding. It
# respects the dependency graph by building packages in topological levels:
# packages in the same level have no dependencies on each other and can run
# in parallel.
#
# Execution strategy
# ------------------
#
# 1. Get the `independent_groups` from the dependency graph -- these are the
#    parallel levels.
# 2. For each level, run all packages in that level concurrently using
#    Ruby threads (`Thread.new`).
# 3. For each package, execute its BUILD commands sequentially via
#    `Open3.capture3`, with the working directory set to the package dir.
# 4. If a package fails (any command returns non-zero), mark all transitive
#    dependents as "dep-skipped" -- there's no point building them.
#
# Build results
# -------------
#
# Each package gets a BuildResult with its status, stdout/stderr output,
# and wall-clock duration.

require "open3"
require "pathname"

module BuildTool
  # --------------------------------------------------------------------------
  # BuildResult -- The result of building a single package.
  #
  # We use Data.define for the same reasons as Package and CacheEntry:
  # immutable value semantics, named fields, and structural equality.
  #
  # Fields:
  #   package_name  -- The package's qualified name.
  #   status        -- One of "built", "failed", "skipped", "dep-skipped",
  #                    "would-build".
  #   duration      -- Wall-clock seconds spent building (0.0 for skipped).
  #   stdout        -- Combined stdout from all BUILD commands.
  #   stderr        -- Combined stderr from all BUILD commands.
  #   return_code   -- Exit code of the last failing command, or 0.
  # --------------------------------------------------------------------------
  BuildResult = Data.define(:package_name, :status, :duration, :stdout, :stderr, :return_code) do
    # Provide sensible defaults. Data.define in Ruby 3.2+ supports
    # overriding `initialize` to set defaults for keyword arguments.
    def initialize(package_name:, status:, duration: 0.0, stdout: "", stderr: "", return_code: 0)
      super
    end
  end

  module Executor
    module_function

    # run_package_build -- Execute all BUILD commands for a single package.
    #
    # Commands are run sequentially. If any command fails (non-zero exit),
    # we stop immediately and return a "failed" result. All commands run
    # with the working directory set to the package directory.
    #
    # We use `Open3.capture3` instead of backticks because it gives us
    # separate stdout, stderr, and the exit status object. This is the
    # Ruby equivalent of Python's `subprocess.run(capture_output=True)`.
    #
    # @param package [Package] The package to build.
    # @return [BuildResult]
    def run_package_build(package)
      start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
      all_stdout = []
      all_stderr = []

      package.build_commands.each do |command|
        begin
          stdout, stderr, status = Open3.capture3(command, chdir: package.path.to_s)
        rescue StandardError => e
          elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start
          return BuildResult.new(
            package_name: package.name,
            status: "failed",
            duration: elapsed,
            stdout: all_stdout.join,
            stderr: all_stderr.join + "\n#{e.message}",
            return_code: 1
          )
        end

        all_stdout << stdout
        all_stderr << stderr

        unless status.success?
          elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start
          return BuildResult.new(
            package_name: package.name,
            status: "failed",
            duration: elapsed,
            stdout: all_stdout.join,
            stderr: all_stderr.join,
            return_code: status.exitstatus || 1
          )
        end
      end

      elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start
      BuildResult.new(
        package_name: package.name,
        status: "built",
        duration: elapsed,
        stdout: all_stdout.join,
        stderr: all_stderr.join,
        return_code: 0
      )
    end

    # execute_builds -- Run BUILD commands for packages respecting dep order.
    #
    # Uses `independent_groups` from the dependency graph to determine which
    # packages can run in parallel. For each level, packages are built
    # concurrently with Ruby threads. Each thread calls `run_package_build`
    # and stores the result.
    #
    # If a package fails, all its transitive dependents are marked as
    # "dep-skipped" using the graph's `transitive_dependents` method.
    #
    # @param packages [Array<Package>] All discovered packages.
    # @param graph [DirectedGraph] The dependency graph.
    # @param cache [BuildCache] The build cache (for skip detection).
    # @param package_hashes [Hash<String, String>] Per-package source hashes.
    # @param deps_hashes [Hash<String, String>] Per-package dep hashes.
    # @param force [Boolean] Rebuild everything regardless of cache.
    # @param dry_run [Boolean] Don't build, just report what would build.
    # @param max_jobs [Integer, nil] Max parallel workers (nil = 8).
    # @return [Hash<String, BuildResult>]
    def execute_builds(packages:, graph:, cache:, package_hashes:, deps_hashes:,
                       force: false, dry_run: false, max_jobs: nil)
      # Build a lookup from name to Package.
      pkg_by_name = packages.each_with_object({}) { |p, h| h[p.name] = p }

      # Get the parallel levels from Kahn's algorithm.
      groups = graph.independent_groups

      results = {}
      failed_packages = Set.new

      groups.each do |level|
        to_build = []

        level.each do |name|
          next unless pkg_by_name.key?(name)

          # Check if a dependency failed. In our graph, edges go dep -> pkg,
          # so a package's dependencies are found via transitive_dependents.
          dep_failed = graph.transitive_dependents(name).any? { |dep| failed_packages.include?(dep) }

          if dep_failed
            results[name] = BuildResult.new(package_name: name, status: "dep-skipped")
            next
          end

          # Check cache to see if we need to build.
          pkg_hash = package_hashes.fetch(name, "")
          dep_hash = deps_hashes.fetch(name, "")

          if !force && !cache.needs_build?(name, pkg_hash, dep_hash)
            results[name] = BuildResult.new(package_name: name, status: "skipped")
            next
          end

          if dry_run
            results[name] = BuildResult.new(package_name: name, status: "would-build")
            next
          end

          to_build << pkg_by_name[name]
        end

        next if to_build.empty? || dry_run

        # Execute this level in parallel using threads.
        #
        # Ruby threads are real OS threads (since MRI 1.9), but the GIL
        # means only one runs Ruby code at a time. However, since our
        # workload is I/O-bound (waiting for subprocesses), threads work
        # perfectly here -- the GIL is released during I/O waits.
        workers = max_jobs || [to_build.size, 8].min
        mutex = Mutex.new
        queue = to_build.dup
        threads = []

        workers.times do
          threads << Thread.new do
            loop do
              pkg = mutex.synchronize { queue.shift }
              break unless pkg

              begin
                result = run_package_build(pkg)
              rescue StandardError => e
                result = BuildResult.new(
                  package_name: pkg.name,
                  status: "failed",
                  stderr: e.message,
                  return_code: 1
                )
              end

              mutex.synchronize do
                results[pkg.name] = result

                if result.status == "built"
                  cache.record(
                    pkg.name,
                    package_hashes.fetch(pkg.name, ""),
                    deps_hashes.fetch(pkg.name, ""),
                    "success"
                  )
                elsif result.status == "failed"
                  failed_packages.add(pkg.name)
                  cache.record(
                    pkg.name,
                    package_hashes.fetch(pkg.name, ""),
                    deps_hashes.fetch(pkg.name, ""),
                    "failed"
                  )
                end
              end
            end
          end
        end

        threads.each(&:join)
      end

      results
    end
  end
end
