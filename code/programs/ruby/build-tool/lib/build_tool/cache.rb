# frozen_string_literal: true

# cache.rb -- Build Cache Management
# ===================================
#
# This module manages a JSON-based cache file (`.build-cache.json`) that
# records the state of each package after its last build. By comparing current
# hashes against cached hashes, we determine which packages need rebuilding.
#
# Cache format
# ------------
#
# The cache file is a JSON object mapping package names to cache entries:
#
#     {
#       "python/logic-gates": {
#         "package_hash": "abc123...",
#         "deps_hash": "def456...",
#         "last_built": "2024-01-15T10:30:00+00:00",
#         "status": "success"
#       }
#     }
#
# Atomic writes
# -------------
#
# To prevent corruption if the process is interrupted mid-write, we write to
# a temporary file (`.build-cache.json.tmp`) first, then atomically rename
# it to the final path. On POSIX systems, `File.rename` is atomic within the
# same filesystem.

require "json"
require "pathname"
require "time"

module BuildTool
  # --------------------------------------------------------------------------
  # CacheEntry -- A single package's cached build state.
  #
  # We use Data.define again (same rationale as Package): it gives us an
  # immutable value object with named fields, structural equality, and
  # a readable #inspect. This is the Ruby equivalent of a Python dataclass.
  #
  # Fields:
  #   package_hash  -- SHA256 of the package's source files.
  #   deps_hash     -- SHA256 of transitive dependency hashes.
  #   last_built    -- ISO 8601 timestamp of the last build.
  #   status        -- "success" or "failed".
  # --------------------------------------------------------------------------
  CacheEntry = Data.define(:package_hash, :deps_hash, :last_built, :status)

  class BuildCache
    # The cache stores entries in a plain Hash keyed by package name.
    # All mutation goes through `record`; all reads go through `needs_build?`.
    attr_reader :entries

    def initialize
      @entries = {}
    end

    # load -- Read cache entries from a JSON file.
    #
    # If the file doesn't exist or is malformed, we start with an empty
    # cache (no error raised -- a missing cache just means everything gets
    # rebuilt). This is a deliberate design choice: we never want a corrupt
    # cache file to prevent builds from running.
    #
    # @param path [Pathname] Path to the cache file.
    def load(path)
      path = Pathname(path)
      unless path.exist?
        @entries = {}
        return
      end

      begin
        text = path.read
        data = JSON.parse(text)
      rescue JSON::ParserError, IOError, SystemCallError
        @entries = {}
        return
      end

      @entries = {}
      data.each do |name, entry_data|
        begin
          @entries[name] = CacheEntry.new(
            package_hash: entry_data.fetch("package_hash"),
            deps_hash: entry_data.fetch("deps_hash"),
            last_built: entry_data.fetch("last_built"),
            status: entry_data.fetch("status")
          )
        rescue KeyError, TypeError
          # Skip malformed entries, just like the Python implementation.
          next
        end
      end
    end

    # save -- Write cache entries to a JSON file with atomic write.
    #
    # We write to a temporary file first, then rename. This prevents
    # corruption if the process is interrupted mid-write. The `File.rename`
    # call is atomic on POSIX systems when source and destination are on
    # the same filesystem.
    #
    # @param path [Pathname] Path to the cache file.
    def save(path)
      path = Pathname(path)

      data = {}
      @entries.sort_by { |name, _| name }.each do |name, entry|
        data[name] = {
          "package_hash" => entry.package_hash,
          "deps_hash"    => entry.deps_hash,
          "last_built"   => entry.last_built,
          "status"       => entry.status
        }
      end

      tmp_path = path.parent / "#{path.basename}.tmp"
      tmp_path.write(JSON.pretty_generate(data) + "\n")
      File.rename(tmp_path.to_s, path.to_s)
    end

    # needs_build? -- Determine if a package needs rebuilding.
    #
    # A package needs rebuilding if:
    #   1. It's not in the cache at all (never built).
    #   2. Its source hash changed (files were modified).
    #   3. Its dependency hash changed (a dependency was modified).
    #   4. Its last build failed.
    #
    # @param name [String] Package name.
    # @param pkg_hash [String] Current SHA256 of source files.
    # @param deps_hash [String] Current SHA256 of dependency hashes.
    # @return [Boolean]
    def needs_build?(name, pkg_hash, deps_hash)
      return true unless @entries.key?(name)

      entry = @entries[name]
      return true if entry.status == "failed"
      return true if entry.package_hash != pkg_hash
      return true if entry.deps_hash != deps_hash

      false
    end

    # record -- Record a build result in the cache.
    #
    # This creates or updates the cache entry for a package. The timestamp
    # is always the current UTC time in ISO 8601 format, matching the
    # Python implementation's use of `datetime.now(timezone.utc).isoformat()`.
    #
    # @param name [String] Package name.
    # @param pkg_hash [String] SHA256 of source files at build time.
    # @param deps_hash [String] SHA256 of dependency hashes at build time.
    # @param status [String] "success" or "failed".
    def record(name, pkg_hash, deps_hash, status)
      @entries[name] = CacheEntry.new(
        package_hash: pkg_hash,
        deps_hash: deps_hash,
        last_built: Time.now.utc.iso8601,
        status: status
      )
    end
  end
end
