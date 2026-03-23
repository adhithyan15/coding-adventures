# frozen_string_literal: true

# plan.rb -- Build Plan Serialization and Deserialization
# =======================================================
#
# A build plan captures the results of the build tool's discovery,
# dependency resolution, and change detection steps as a JSON file.
# This enables CI to compute the plan once in a fast "detect" job and
# share it across build jobs on multiple platforms -- eliminating
# redundant computation.
#
# Schema versioning
# -----------------
#
# The plan uses a simple integer version scheme (schema_version field).
# Readers MUST reject plans with a version higher than what they support,
# falling back to the normal discovery flow. Writers always stamp the
# current version. See code/specs/build-plan-v1.md for the full spec.
#
# Path conventions
# ----------------
#
# All paths in the plan use forward slashes (/) regardless of platform.
# On write, backslashes are converted to forward slashes. On read,
# consumers can convert back to platform-native separators if needed.
#
# Nil vs empty affected_packages
# -------------------------------
#
# The affected_packages field has three-state semantics:
#
#     nil/null   => rebuild all (force mode or git diff unavailable)
#     []         => nothing changed, build nothing
#     [a, b, ...] => only these packages need building
#
# This three-state design avoids the ambiguity of using an empty array
# for both "nothing changed" and "rebuild all". JSON null maps to Ruby
# nil, making the round-trip clean.
#
# Atomic writes
# -------------
#
# Like the cache module, we write to a temporary file first, then
# rename atomically. This prevents corruption if the process is
# interrupted mid-write.

require "json"
require "pathname"

module BuildTool
  module Plan
    # CURRENT_SCHEMA_VERSION -- The version that this implementation
    # reads and writes. Plans with a higher version are rejected.
    CURRENT_SCHEMA_VERSION = 1

    # PackageEntry -- A single package in the build plan.
    #
    # We use Data.define (Ruby 3.2+) for immutable value semantics,
    # matching the convention used by Package, CacheEntry, and
    # BuildResult elsewhere in the build tool.
    #
    # Fields:
    #   name           -- Qualified package name: "language/package-name".
    #   rel_path       -- Repo-root-relative path, always forward slashes.
    #   language       -- The package's programming language.
    #   build_commands -- Shell commands to execute for building/testing.
    #   is_starlark    -- Whether the BUILD file uses Starlark syntax.
    #   declared_srcs  -- Glob patterns from the Starlark srcs field.
    #   declared_deps  -- Qualified names from the Starlark deps field.
    PackageEntry = Data.define(
      :name, :rel_path, :language, :build_commands,
      :is_starlark, :declared_srcs, :declared_deps
    ) do
      # Provide sensible defaults for optional fields.
      def initialize(name:, rel_path:, language:, build_commands:,
                     is_starlark: false, declared_srcs: [], declared_deps: [])
        super
      end
    end

    # BuildPlan -- The top-level structure serialized to JSON.
    #
    # Fields:
    #   schema_version    -- Format version. Readers reject versions
    #                        higher than CURRENT_SCHEMA_VERSION.
    #   diff_base         -- Git ref used for change detection (informational).
    #   force             -- Whether --force was set.
    #   affected_packages -- Package names needing building. nil => rebuild all,
    #                        [] => nothing changed, [...] => these packages.
    #   packages          -- ALL discovered packages (not just affected ones).
    #   dependency_edges  -- Directed edges [[from, to], ...] where from->to
    #                        means "to depends on from".
    #   languages_needed  -- Map of language names to booleans indicating
    #                        whether that language's toolchain is needed.
    BuildPlan = Data.define(
      :schema_version, :diff_base, :force, :affected_packages,
      :packages, :dependency_edges, :languages_needed
    ) do
      def initialize(schema_version: CURRENT_SCHEMA_VERSION, diff_base: "",
                     force: false, affected_packages: nil, packages: [],
                     dependency_edges: [], languages_needed: {})
        super
      end
    end

    module_function

    # write_plan -- Serialize a build plan to a JSON file.
    #
    # Always stamps schema_version to CURRENT_SCHEMA_VERSION, regardless
    # of what value the caller set. This ensures we never accidentally
    # write a future version.
    #
    # Uses atomic write: write to a .tmp file, then rename. On POSIX
    # systems, File.rename is atomic within the same filesystem.
    #
    # @param bp [BuildPlan] The plan to write.
    # @param path [String, Pathname] Output file path.
    def write_plan(bp, path)
      path = Pathname(path)

      data = {
        "schema_version" => CURRENT_SCHEMA_VERSION,
        "diff_base" => bp.diff_base,
        "force" => bp.force,
        "affected_packages" => bp.affected_packages,
        "packages" => bp.packages.map { |pkg| package_entry_to_hash(pkg) },
        "dependency_edges" => bp.dependency_edges,
        "languages_needed" => bp.languages_needed
      }

      tmp_path = path.parent / "#{path.basename}.tmp"
      tmp_path.write(JSON.pretty_generate(data) + "\n")
      File.rename(tmp_path.to_s, path.to_s)
    end

    # read_plan -- Deserialize a build plan from a JSON file.
    #
    # Returns the BuildPlan, or raises an error if:
    #   - The file doesn't exist or can't be read.
    #   - The JSON is malformed.
    #   - The schema_version is higher than CURRENT_SCHEMA_VERSION.
    #
    # A schema_version LOWER than or EQUAL to the current version is
    # accepted. This allows forward compatibility -- a newer tool can
    # read plans written by older tools.
    #
    # @param path [String, Pathname] Input file path.
    # @return [BuildPlan]
    # @raise [RuntimeError] If the file is missing, unparseable, or
    #   has a schema version higher than CURRENT_SCHEMA_VERSION.
    def read_plan(path)
      path = Pathname(path)

      unless path.exist?
        raise "build plan file not found: #{path}"
      end

      data = JSON.parse(path.read)

      version = data.fetch("schema_version", 0)
      if version > CURRENT_SCHEMA_VERSION
        raise "unsupported build plan version #{version} " \
              "(this tool supports up to #{CURRENT_SCHEMA_VERSION})"
      end

      packages = (data["packages"] || []).map { |h| hash_to_package_entry(h) }
      edges = (data["dependency_edges"] || []).map { |e| e.is_a?(Array) ? e : [] }
      languages = data["languages_needed"] || {}

      BuildPlan.new(
        schema_version: version,
        diff_base: data["diff_base"] || "",
        force: data["force"] || false,
        affected_packages: data["affected_packages"],
        packages: packages,
        dependency_edges: edges,
        languages_needed: languages
      )
    rescue JSON::ParserError => e
      raise "parse build plan: #{e.message}"
    rescue Errno::ENOENT => e
      raise "read build plan: #{e.message}"
    end

    # -- Private helpers -------------------------------------------------------

    # package_entry_to_hash -- Convert a PackageEntry to a JSON-ready hash.
    #
    # @param pkg [PackageEntry]
    # @return [Hash]
    def package_entry_to_hash(pkg)
      h = {
        "name" => pkg.name,
        "rel_path" => pkg.rel_path.tr("\\", "/"),
        "language" => pkg.language,
        "build_commands" => pkg.build_commands,
        "is_starlark" => pkg.is_starlark
      }
      # Only include srcs and deps if non-empty, matching Go's omitempty.
      h["declared_srcs"] = pkg.declared_srcs unless pkg.declared_srcs.empty?
      h["declared_deps"] = pkg.declared_deps unless pkg.declared_deps.empty?
      h
    end

    # hash_to_package_entry -- Convert a JSON hash to a PackageEntry.
    #
    # Missing fields get sensible defaults rather than raising errors.
    # This makes the reader resilient to plans from older tool versions
    # that might not include all fields.
    #
    # @param h [Hash]
    # @return [PackageEntry]
    def hash_to_package_entry(h)
      PackageEntry.new(
        name: h["name"] || "",
        rel_path: h["rel_path"] || "",
        language: h["language"] || "unknown",
        build_commands: h["build_commands"] || [],
        is_starlark: h["is_starlark"] || false,
        declared_srcs: h["declared_srcs"] || [],
        declared_deps: h["declared_deps"] || []
      )
    end
  end
end
