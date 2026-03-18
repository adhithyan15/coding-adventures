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
#
# The flow is:
#   1. Discover packages (walk DIRS/BUILD files)
#   2. Filter by language if specified
#   3. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod)
#   4. Hash all packages
#   5. Load cache, determine what needs building
#   6. If --dry-run, print what would build and exit
#   7. Execute builds (parallel by level)
#   8. Update and save cache
#   9. Print report
#  10. Exit with code 1 if any builds failed

require "optparse"
require "pathname"
require "set"

# Load all build tool modules. We use require_relative so the tool works
# as a standalone script without needing to be installed as a gem.
require_relative "lib/build_tool/discovery"
require_relative "lib/build_tool/resolver"
require_relative "lib/build_tool/hasher"
require_relative "lib/build_tool/cache"
require_relative "lib/build_tool/executor"
require_relative "lib/build_tool/reporter"

module BuildTool
  module CLI
    module_function

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

    # main -- Main entry point for the build tool CLI.
    #
    # Parses command-line arguments using OptionParser (Ruby's built-in
    # equivalent of Python's argparse), then orchestrates the full pipeline:
    # discover -> resolve -> hash -> cache -> execute -> report.
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
        cache_file: Pathname(".build-cache.json")
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

        opts.on("--language LANG", %w[python ruby go all],
                "Only build packages of this language (python/ruby/go/all)") do |lang|
          options[:language] = lang
        end

        opts.on("--cache-file FILE", "Path to the build cache file") do |file|
          options[:cache_file] = Pathname(file)
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

      # -- Step 3: Filter by language -------------------------------------------
      if options[:language] != "all"
        packages = packages.select { |p| p.language == options[:language] }
        if packages.empty?
          $stderr.puts "No #{options[:language]} packages found."
          return 0
        end
      end

      puts "Discovered #{packages.size} packages"

      # -- Step 4: Resolve dependencies -----------------------------------------
      graph = Resolver.resolve_dependencies(packages)

      # -- Step 5: Hash all packages --------------------------------------------
      package_hashes = {}
      deps_hashes = {}

      packages.each do |pkg|
        package_hashes[pkg.name] = Hasher.hash_package(pkg)
        deps_hashes[pkg.name] = Hasher.hash_deps(pkg.name, graph, package_hashes)
      end

      # -- Step 6: Load cache ---------------------------------------------------
      cache_path = options[:cache_file]
      cache_path = root / cache_path unless cache_path.absolute?

      cache = BuildCache.new
      cache.load(cache_path)

      # -- Steps 7-8: Execute builds --------------------------------------------
      results = Executor.execute_builds(
        packages: packages,
        graph: graph,
        cache: cache,
        package_hashes: package_hashes,
        deps_hashes: deps_hashes,
        force: options[:force],
        dry_run: options[:dry_run],
        max_jobs: options[:jobs]
      )

      # -- Step 9: Save cache (unless dry run) ----------------------------------
      cache.save(cache_path) unless options[:dry_run]

      # -- Step 10: Print report ------------------------------------------------
      Reporter.print_report(results)

      # -- Step 11: Exit code ---------------------------------------------------
      has_failures = results.values.any? { |r| r.status == "failed" }
      has_failures ? 1 : 0
    end
  end
end

# Run the CLI when invoked directly.
exit(BuildTool::CLI.main) if $PROGRAM_NAME == __FILE__
