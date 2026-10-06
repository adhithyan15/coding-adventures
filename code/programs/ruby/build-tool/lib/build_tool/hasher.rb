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
require "json"
require "pathname"
require_relative "glob_match"

module BuildTool
  module Hasher
    # The checked fixture is copied into package data at review time. Runtime
    # hashing never searches for a repository fixture or host configuration.
    SOURCE_INPUT_REGISTRY_PATH = Pathname(__dir__) / "language_source_input_registry.json"
    SOURCE_INPUT_REGISTRY = JSON.parse(SOURCE_INPUT_REGISTRY_PATH.binread, freeze: true)
    LANGUAGE_INPUTS = SOURCE_INPUT_REGISTRY.fetch("languages").to_h do |entry|
      [entry.fetch("language"), entry]
    end.freeze
    UNIVERSAL_INPUTS = SOURCE_INPUT_REGISTRY.fetch("universal_inputs").freeze
    BUILD_FILENAMES = UNIVERSAL_INPUTS.fetch("build_filenames").freeze
    GENERATED_DIRECTORY_COMPONENTS = UNIVERSAL_INPUTS.fetch("generated_directory_components").freeze

    module_function

    def source_input_registry
      SOURCE_INPUT_REGISTRY
    end

    # Canonical JSON sorts object keys while preserving semantically ordered
    # registry arrays. The domain and length prevent a digest from being
    # confused with another JSON document or a different registry version.
    def source_input_registry_digest
      canonical = canonical_json(SOURCE_INPUT_REGISTRY).b
      Digest::SHA256.hexdigest(
        "coding-adventures/build-tool-language-source-input-registry/v1\0".b +
        [canonical.bytesize].pack("Q>") + canonical
      )
    end

    def canonical_json(value)
      case value
      when Hash
        "{" + value.keys.sort.map { |key| "#{JSON.generate(key)}:#{canonical_json(value.fetch(key))}" }.join(",") + "}"
      when Array
        "[" + value.map { |item| canonical_json(item) }.join(",") + "]"
      else
        JSON.generate(value)
      end
    end

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
      language_inputs(package.language)
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
      entry = language_inputs(package.language)
      exact_inputs = package_exact_paths(package, entry)
      files = []

      each_source_file(package.path) do |filepath|
        relative = portable_relative_path(package.path, filepath)
        files << filepath if registry_input?(relative, entry, exact_inputs, declared: false)
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
      entry = language_inputs(package.language)
      compiled_patterns = GlobMatch.compile_patterns(declared_srcs)
      exact_inputs = package_exact_paths(package, entry)
      files = []

      each_source_file(package.path) do |filepath|
        rel = portable_relative_path(package.path, filepath)
        if registry_input?(rel, entry, exact_inputs, declared: true) ||
            compiled_patterns.any? { |pattern| GlobMatch.match_compiled_path?(pattern, rel) }
          files << filepath
        end
      end

      sort_portable_paths(files, package.path)
    end

    def language_inputs(language)
      LANGUAGE_INPUTS.fetch(language) { raise ArgumentError, "unknown source language: #{language}" }
    end

    def registry_input?(relative, entry, exact_inputs, declared:)
      basename = relative.split("/").last
      root = !relative.include?("/")
      return true if BUILD_FILENAMES.include?(basename)
      return true if root && UNIVERSAL_INPUTS.fetch("root_exact_basenames").include?(basename)
      return true if root && entry.fetch("root_exact_basenames").include?(basename)
      return true if root && entry.fetch("root_variable_suffixes").any? { |suffix| basename.end_with?(suffix) }
      return true if entry.fetch("root_exact_relative_paths").include?(relative)
      return true if exact_inputs.include?(relative)
      return false if declared

      return true if entry.fetch("recursive_suffixes").any? { |suffix| basename.end_with?(suffix) }
      return true if entry.fetch("recursive_exact_basenames").include?(basename)

      entry.fetch("scoped_inputs").any? do |rule|
        in_scope = if rule.fetch("scope") == "root"
          root
        else
          relative.start_with?("#{rule.fetch("path_prefix")}/")
        end
        in_scope && (rule.fetch("suffixes").any? { |suffix| basename.end_with?(suffix) } ||
          rule.fetch("exact_basenames").include?(basename))
      end
    end

    # Package-exact rules require a real canonical root in the checkout path.
    # The name-only fallback used by isolated hashing tests cannot grant a
    # checked-in native or site companion to an unrelated temporary package.
    def package_exact_paths(package, entry)
      root = canonical_source_root(package, entry)
      return [] unless root

      entry.fetch("package_exact_inputs")
        .select { |rule| rule.fetch("package_root") == root }
        .flat_map { |rule| rule.fetch("paths") }
    end

    def canonical_source_root(package, entry)
      parts = package.path.expand_path.each_filename.to_a
      (0...parts.length).reverse_each do |index|
        next unless parts[index] == "code"

        section = parts[index + 1]
        if %w[packages programs].include?(section)
          return nil unless parts.length >= index + 4 && parts[index + 2] == package.language
          return validate_repository_path(parts[index..].join("/"))
        end
        if section == "sites"
          root = parts[index..].join("/")
          registered = entry.fetch("package_exact_inputs")
            .any? { |rule| rule.fetch("package_root") == root }
          return validate_repository_path(root) if package.language == "typescript" && registered
          return nil
        end
      end
      nil
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
