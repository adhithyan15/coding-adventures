# frozen_string_literal: true

# hasher.rb -- SHA256 File Hashing for Change Detection
# =====================================================
#
# This module computes SHA256 hashes for package source files. The hash of a
# package is a single string that changes whenever any source file in the
# package is modified, added, or removed.
#
# How hashing works
# -----------------
#
# 1. Collect all source files in the package directory, filtered by the
#    language's relevant extensions. Always include the BUILD file.
# 2. Sort the file list lexicographically (by relative path) for determinism.
# 3. SHA256-hash each file's contents individually.
# 4. Concatenate all individual hashes into one string.
# 5. SHA256-hash that concatenated string to produce the final package hash.
#
# This two-level hashing means:
# - Reordering files doesn't change the hash (we sort first).
# - Adding or removing a file changes the hash (the concatenated string changes).
# - Modifying any file's contents changes the hash.
#
# Dependency hashing
# ------------------
#
# A package should be rebuilt if any of its transitive dependencies changed.
# `hash_deps` takes a package name, the dependency graph, and the per-package
# hashes, then produces a single hash representing the state of all deps.

require "digest/sha2"
require "pathname"
require_relative "glob_match"

module BuildTool
  module Hasher
    # SOURCE_EXTENSIONS -- File extensions that matter for each language.
    #
    # If any file with one of these extensions changes, the package needs
    # rebuilding. We use a frozen hash of frozen sets for safety.
    SOURCE_EXTENSIONS = {
      "python"     => %w[.py .toml .cfg].freeze,
      "ruby"       => %w[.rb .gemspec].freeze,
      "go"         => %w[.go].freeze,
      "typescript" => %w[.ts .tsx .json].freeze,
      "rust"       => %w[.rs .toml].freeze,
      "elixir"     => %w[.ex .exs].freeze,
      "starlark"   => %w[.star].freeze,
      "perl"       => %w[.pl .pm .t .xs].freeze
    }.freeze

    # SPECIAL_FILENAMES -- Files to always include regardless of extension.
    #
    # These are ecosystem-specific config files that affect the build but
    # don't have a standard source extension.
    SPECIAL_FILENAMES = {
      "python"     => [].freeze,
      "ruby"       => %w[Gemfile Rakefile].freeze,
      "go"         => %w[go.mod go.sum].freeze,
      "typescript" => %w[package.json tsconfig.json vitest.config.ts].freeze,
      "rust"       => %w[Cargo.toml Cargo.lock].freeze,
      "elixir"     => %w[mix.exs mix.lock].freeze,
      "starlark"   => [].freeze,
      "perl"       => %w[Makefile.PL Build.PL cpanfile MANIFEST META.json META.yml].freeze
    }.freeze

    module_function

    # collect_source_files -- Gather all source files in a package directory.
    #
    # There are two modes of operation:
    #
    # 1. **Extension-based** (shell BUILD or Starlark without declared_srcs):
    #    Files are filtered by the language's relevant extensions and special
    #    filenames. BUILD files are always included.
    #
    # 2. **Glob-based** (Starlark with declared_srcs):
    #    Files are matched against the declared source glob patterns using
    #    the GlobMatch module. BUILD files are always included. This mode
    #    is more precise -- only files explicitly declared in the Starlark
    #    BUILD file are considered for hashing.
    #
    # Returns a sorted list of Pathname objects (sorted by relative path
    # for determinism).
    #
    # @param package [Package] The package to scan.
    # @return [Array<Pathname>] Sorted absolute paths to source files.
    def collect_source_files(package)
      # Check if this package has declared_srcs (Starlark metadata).
      # The Package struct might not have this field (older code), so we
      # use respond_to? for safety.
      declared_srcs = if package.respond_to?(:declared_srcs)
                        package.declared_srcs || []
                      else
                        []
                      end

      if declared_srcs.any?
        collect_source_files_glob(package, declared_srcs)
      else
        collect_source_files_extension(package)
      end
    end

    # collect_source_files_extension -- Extension-based file collection.
    #
    # The original algorithm: filter by language extensions and special
    # filenames. Used for shell BUILD packages and Starlark packages
    # without declared_srcs.
    #
    # @param package [Package] The package to scan.
    # @return [Array<Pathname>] Sorted absolute paths to source files.
    def collect_source_files_extension(package)
      extensions = SOURCE_EXTENSIONS.fetch(package.language, [])
      special_names = SPECIAL_FILENAMES.fetch(package.language, [])

      files = []

      # Pathname#find recursively walks the directory tree, like Python's
      # Path.rglob("*"). We skip directories and only collect files.
      package.path.find do |filepath|
        next unless filepath.file?

        # Always include BUILD files.
        if %w[BUILD BUILD_mac BUILD_linux].include?(filepath.basename.to_s)
          files << filepath
          next
        end

        # Check extension.
        if extensions.include?(filepath.extname)
          files << filepath
          next
        end

        # Check special filenames.
        if special_names.include?(filepath.basename.to_s)
          files << filepath
          next
        end
      end

      # Sort by relative path for determinism, matching the Python behavior.
      files.sort_by { |f| f.relative_path_from(package.path).to_s }
    end

    # collect_source_files_glob -- Glob-based file collection.
    #
    # For Starlark packages with declared_srcs, we match each file in the
    # package directory against the declared source patterns. BUILD files
    # are always included regardless of patterns.
    #
    # This uses the GlobMatch module for ** support, ensuring consistent
    # behavior with git_diff's strict filtering and the Go build tool.
    #
    # @param package [Package] The package to scan.
    # @param declared_srcs [Array<String>] Glob patterns from Starlark srcs.
    # @return [Array<Pathname>] Sorted absolute paths to source files.
    def collect_source_files_glob(package, declared_srcs)
      files = []

      package.path.find do |filepath|
        next unless filepath.file?

        basename = filepath.basename.to_s

        # Always include BUILD files.
        if basename == "BUILD" || basename.start_with?("BUILD_")
          files << filepath
          next
        end

        # Match against declared source patterns.
        rel = filepath.relative_path_from(package.path).to_s
        if declared_srcs.any? { |pattern| GlobMatch.match_path?(pattern, rel) }
          files << filepath
        end
      end

      files.sort_by { |f| f.relative_path_from(package.path).to_s }
    end

    # hash_file -- Compute the SHA256 hex digest of a single file.
    #
    # We read in 8 KiB chunks, identical to the Python implementation, to
    # handle large files without loading them entirely into memory.
    #
    # @param filepath [Pathname] The file to hash.
    # @return [String] Hex-encoded SHA256 digest.
    def hash_file(filepath)
      sha = Digest::SHA256.new
      filepath.open("rb") do |f|
        while (chunk = f.read(8192))
          sha.update(chunk)
        end
      end
      sha.hexdigest
    end

    # hash_package -- Compute a SHA256 hash representing all source files.
    #
    # The hash changes if any source file is added, removed, or modified.
    # We hash each file individually, concatenate the hex digests, then hash
    # the concatenated string. This two-level approach is deterministic and
    # efficient.
    #
    # @param package [Package] The package to hash.
    # @return [String] Hex-encoded SHA256 digest.
    def hash_package(package)
      files = collect_source_files(package)

      if files.empty?
        # No source files -- hash the empty string for consistency.
        return Digest::SHA256.hexdigest("")
      end

      file_hashes = files.map { |f| hash_file(f) }
      combined = file_hashes.join
      Digest::SHA256.hexdigest(combined)
    end

    # hash_deps -- Compute a SHA256 hash of all transitive dependency hashes.
    #
    # If any transitive dependency's source files changed, this hash will
    # change too, triggering a rebuild of the dependent package.
    #
    # In our graph, edges go dep -> pkg (dependency points to dependent),
    # so a package's dependencies are found by walking reverse edges
    # (`transitive_dependents`).
    #
    # @param package_name [String] The package whose deps we're hashing.
    # @param graph [DirectedGraph] The dependency graph.
    # @param package_hashes [Hash<String, String>] Per-package source hashes.
    # @return [String] Hex-encoded SHA256 digest.
    def hash_deps(package_name, graph, package_hashes)
      unless graph.has_node?(package_name)
        return Digest::SHA256.hexdigest("")
      end

      transitive_deps = graph.transitive_dependents(package_name)

      if transitive_deps.empty?
        return Digest::SHA256.hexdigest("")
      end

      # Sort dependency names for determinism, concatenate their hashes.
      sorted_deps = transitive_deps.to_a.sort
      combined = sorted_deps.map { |dep| package_hashes.fetch(dep, "") }.join
      Digest::SHA256.hexdigest(combined)
    end
  end
end
