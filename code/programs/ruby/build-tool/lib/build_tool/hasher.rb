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
# 2. Normalize repository-relative paths to forward-slash UTF-8 and sort them.
# 3. Frame each path and raw file body with unsigned 64-bit big-endian lengths.
# 4. SHA256-hash that unambiguous byte stream to produce the package hash.
#
# This framed hashing means:
# - Reordering files doesn't change the hash (we sort first).
# - Adding or removing a file changes the hash (the framed stream changes).
# - Modifying any file's contents changes the hash.
# - Renaming a file changes the hash even when its raw contents do not.
#
# Dependency hashing
# ------------------
#
# A package should be rebuilt if any of its transitive dependencies changed.
# `hash_deps` takes a package name, the dependency graph, and the per-package
# hashes, then produces a single hash representing the state of all deps.

require "digest/sha2"
require "find"
require "pathname"
require_relative "glob_match"

module BuildTool
  module Hasher
    # SOURCE_EXTENSIONS -- File extensions that matter for each language.
    #
    # If any file with one of these extensions changes, the package needs
    # rebuilding. We use a frozen hash of frozen sets for safety.
    SOURCE_EXTENSIONS = {
      "python" => %w[.py .toml .cfg].freeze,
      "ruby" => %w[.rb .gemspec].freeze,
      "go" => %w[.go].freeze,
      "typescript" => %w[.ts .tsx .json].freeze,
      "rust" => %w[.rs .toml].freeze,
      "elixir" => %w[.ex .exs].freeze,
      "starlark" => %w[.star].freeze,
      "perl" => %w[.pl .pm .t .xs].freeze,
      "haskell" => %w[.hs .cabal].freeze,
      "ocaml" => %w[.ml .mli .opam].freeze
    }.freeze

    # SPECIAL_FILENAMES -- Files to always include regardless of extension.
    #
    # These are ecosystem-specific config files that affect the build but
    # don't have a standard source extension.
    SPECIAL_FILENAMES = {
      "python" => [].freeze,
      "ruby" => %w[Gemfile Rakefile].freeze,
      "go" => %w[go.mod go.sum].freeze,
      "typescript" => %w[package.json tsconfig.json vitest.config.ts].freeze,
      "rust" => %w[Cargo.toml Cargo.lock].freeze,
      "elixir" => %w[mix.exs mix.lock].freeze,
      "starlark" => [].freeze,
      "perl" => %w[Makefile.PL Build.PL cpanfile MANIFEST META.json META.yml].freeze,
      "haskell" => [].freeze,
      "ocaml" => %w[.ocamlformat dune dune-project].freeze
    }.freeze

    # Exact BUILD fronts supported by package discovery. Arbitrary BUILD_*
    # lookalikes are ordinary files and must not silently widen a digest.
    BUILD_FILENAMES = %w[
      BUILD BUILD_mac BUILD_linux BUILD_windows BUILD_mac_and_linux
    ].freeze

    # Manifest extensions included independently of declared source globs.
    # These manifests are package-root metadata, not nested dependency inputs.
    DECLARED_MANIFEST_EXTENSIONS = {
      "ocaml" => %w[.opam].freeze
    }.freeze

    # GENERATED_DIRECTORY_COMPONENTS -- Exact directories that are not source.
    #
    # This registry is deliberately case-sensitive and component-based. A
    # directory named `_build` is generated Dune output, while `_Build` and
    # `_build-example` may be authored source. Pruning happens before either
    # extension or declared-source matching so a broad glob cannot pull build
    # artifacts back into a package digest.
    GENERATED_DIRECTORY_COMPONENTS = %w[
      .build
      .cargo
      .claude
      .dart_tool
      .git
      .gradle
      .hg
      .mypy_cache
      .pytest_cache
      .ruff_cache
      .stack-work
      .svn
      .tox
      .venv
      Pods
      __pycache__
      _build
      build
      cover
      deps
      dist
      dist-newstyle
      gradle-build
      node_modules
      target
      vendor
    ].freeze

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

      each_source_file(package.path) do |filepath|
        basename = filepath.basename.to_s

        # Always include exact BUILD fronts.
        if BUILD_FILENAMES.include?(basename)
          files << filepath
          next
        end

        # Check extension.
        if extensions.include?(filepath.extname)
          files << filepath
          next
        end

        # Check special filenames.
        if special_names.include?(basename)
          files << filepath
          next
        end
      end

      # Sort by relative path for determinism, matching the Python behavior.
      sort_portable_paths(files, package.path)
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
      special_names = SPECIAL_FILENAMES.fetch(package.language, [])
      manifest_extensions = DECLARED_MANIFEST_EXTENSIONS.fetch(package.language, [])

      each_source_file(package.path) do |filepath|
        basename = filepath.basename.to_s

        # Always include exact BUILD fronts.
        if BUILD_FILENAMES.include?(basename)
          files << filepath
          next
        end

        # Exact package metadata remains a hashing input even when declared
        # source globs omit it. Extension manifests are root-scoped.
        if special_names.include?(basename) ||
            (filepath.dirname == package.path && manifest_extensions.include?(filepath.extname))
          files << filepath
          next
        end

        # Match against declared source patterns.
        rel = portable_relative_path(package.path, filepath)
        if declared_srcs.any? { |pattern| GlobMatch.match_path?(pattern, rel) }
          files << filepath
        end
      end

      sort_portable_paths(files, package.path)
    end

    # each_source_file -- Walk one package without entering generated trees.
    #
    # Find walks top-down and uses lstat before recursion, so directory links
    # retain the existing no-follow boundary. Calling Find.prune while the
    # directory itself is current prevents enumeration of every descendant;
    # filtering later would be both wasteful and too late for the contract.
    #
    # @param root [Pathname] Package directory to walk.
    # @return [Enumerator<Pathname>] Regular source candidates only.
    def each_source_file(root)
      return enum_for(__method__, root) unless block_given?

      root.find do |filepath|
        if filepath != root && File.lstat(filepath).symlink?
          Find.prune
        elsif filepath != root && filepath.directory? &&
            GENERATED_DIRECTORY_COMPONENTS.include?(filepath.basename.to_s)
          Find.prune
        end

        yield filepath if regular_unlinked_file?(filepath)
      end
    end

    # Convert one package-local path to normalized portable UTF-8. Filesystem
    # paths may arrive as binary strings on POSIX; interpreting those bytes as
    # UTF-8 is lossless, while malformed byte sequences are rejected.
    def portable_relative_path(root, filepath)
      relative = filepath.relative_path_from(root).to_s
      relative = relative.tr(File::ALT_SEPARATOR, File::SEPARATOR) if File::ALT_SEPARATOR
      normalized = relative.dup
      normalized = normalized.encode(Encoding::UTF_8) unless
        [Encoding::UTF_8, Encoding::ASCII_8BIT].include?(normalized.encoding)
      normalized.force_encoding(Encoding::UTF_8)

      components = normalized.split("/", -1)
      portable = normalized.valid_encoding? && !normalized.empty? &&
        !normalized.start_with?("/") && !normalized.include?("\0") &&
        !normalized.include?("\\") &&
        components.none? { |component| component.empty? || %w[. ..].include?(component) }
      raise ArgumentError, "source path is not portable UTF-8" unless portable

      normalized
    rescue ArgumentError, EncodingError
      raise ArgumentError, "source path is not portable UTF-8"
    end

    # Sort by normalized UTF-8 bytes rather than host locale or UTF-16 order.
    def sort_portable_paths(files, root)
      files.sort do |left, right|
        portable_relative_path(root, left).b <=> portable_relative_path(root, right).b
      end
    end

    def regular_unlinked_file?(filepath)
      status = File.lstat(filepath)
      status.file? && !status.symlink?
    rescue Errno::ENOENT, Errno::EACCES
      false
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
      path_status = File.lstat(filepath)
      raise IOError, "source link is not hashable" if path_status.symlink?

      filepath.open("rb") do |f|
        raise IOError, "source changed before hashing" unless same_file?(path_status, f.stat)

        while (chunk = f.read(8192))
          sha.update(chunk)
        end
      end
      sha.hexdigest
    end

    # hash_package -- Compute a SHA256 hash representing all source files.
    #
    # The hash changes if any source file is added, removed, or modified.
    # Hashing v1 frames every normalized repository-relative UTF-8 path and raw
    # content with unsigned 64-bit byte lengths. Boundaries are unambiguous and
    # checkout-specific absolute prefixes never enter the digest.
    #
    # @param package [Package] The package to hash.
    # @return [String] Hex-encoded SHA256 digest.
    def hash_package(package)
      files = collect_source_files(package)

      package_hash = Digest::SHA256.new
      package_root = repository_relative_package_path(package)
      files.each do |filepath|
        relative_path = portable_relative_path(package.path, filepath)
        update_file_frame(package_hash, "#{package_root}/#{relative_path}", filepath, package.path)
      end
      package_hash.hexdigest
    end

    def repository_relative_package_path(package)
      parts = package.path.expand_path.each_filename.to_a
      canonical_start = nil
      parts.each_index do |index|
        next unless parts[index] == "code"
        next unless %w[packages programs].include?(parts[index + 1])

        canonical_start = index
      end
      return validate_repository_path(parts[canonical_start..].join("/")) if canonical_start

      identity = package.name.split("/", -1)
      fallback = if identity.length == 3 && identity[1] == "programs"
        "code/programs/#{identity[0]}/#{identity[2]}"
      elsif identity.length == 2
        "code/packages/#{identity[0]}/#{identity[1]}"
      end
      raise ArgumentError, "cannot derive repository-relative package path" unless fallback

      validate_repository_path(fallback)
    end

    def validate_repository_path(path)
      utf8 = path.dup.force_encoding(Encoding::UTF_8)
      components = utf8.split("/", -1)
      portable = utf8.valid_encoding? && !utf8.include?("\0") &&
        !utf8.include?("\\") && !utf8.start_with?("/") &&
        components.none? { |component| component.empty? || %w[. ..].include?(component) }
      raise ArgumentError, "package path is not portable UTF-8" unless portable

      utf8
    end

    def update_file_frame(package_hash, repository_path, filepath, package_root)
      path_bytes = validate_repository_path(repository_path).b
      package_hash.update([path_bytes.bytesize].pack("Q>"))
      package_hash.update(path_bytes)

      ensure_unlinked_components!(package_root, filepath)
      path_status = File.lstat(filepath)
      raise IOError, "source link is not hashable" unless path_status.file? && !path_status.symlink?

      filepath.open("rb") do |source|
        opened_status = source.stat
        raise IOError, "source changed before hashing" unless same_file?(path_status, opened_status)

        signature = source_signature(opened_status)
        content_length = opened_status.size
        package_hash.update([content_length].pack("Q>"))

        bytes_read = 0
        while (chunk = source.read(8192))
          package_hash.update(chunk)
          bytes_read += chunk.bytesize
        end

        after_status = source.stat
        unless bytes_read == content_length && source_signature(after_status) == signature
          raise IOError, "source changed while hashing"
        end
      end

      ensure_unlinked_components!(package_root, filepath)
    end

    # Stable-state no-follow boundary. Every existing component from the
    # package root to the file is inspected lexically before and after reading.
    # This intentionally does not claim an atomic TOCTOU boundary on runtimes
    # that do not expose descriptor-relative no-follow opens.
    def ensure_unlinked_components!(package_root, filepath)
      relative = filepath.relative_path_from(package_root)
      current = package_root
      ([Pathname(".")] + relative.each_filename.map { |part| Pathname(part) }).each do |part|
        current /= part unless part.to_s == "."
        status = File.lstat(current)
        raise IOError, "source link component is not hashable" if status.symlink?
      end
    rescue ArgumentError
      raise IOError, "source path escapes package root"
    end

    def same_file?(left, right)
      same_device = left.dev == right.dev || left.dev.zero? || right.dev.zero?
      left.file? && right.file? && same_device && left.ino == right.ino
    end

    def source_signature(status)
      [status.dev, status.ino, status.size, status.mtime.to_r, status.ctime.to_r]
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
