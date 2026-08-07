# frozen_string_literal: true

# discovery.rb -- Package Discovery via Recursive BUILD File Walk
# ================================================================
#
# This module walks a monorepo directory tree to discover packages. A "package"
# is any directory that contains a BUILD file. The walk is recursive: starting
# from the root, we list all subdirectories and descend into each one, skipping
# known non-source directories (.git, .venv, node_modules, etc.).
#
# When we find a BUILD file in a directory, we stop recursing there and register
# that directory as a package. This is the same approach used by Bazel, Buck,
# and Pants — no configuration files are needed to route the walk.
#
# Platform-specific BUILD files
# -----------------------------
#
# If we're on macOS and a `BUILD_mac` file exists, we use that instead of
# `BUILD`. Similarly, `BUILD_linux` on Linux. This lets packages define
# platform-specific build commands (e.g., different compiler flags).
#
# Language inference
# -----------------
#
# We infer the language only from the exact directory immediately below a
# `packages` or `programs` component. Programs retain a `programs` identity
# segment so they cannot collide with a library package of the same basename.

module BuildTool
  # Two or more discovered directories normalized to one graph identity.
  class DuplicatePackageIdentityError < StandardError
    attr_reader :code, :package, :paths

    def initialize(package:, paths:)
      @code = "DUPLICATE_PACKAGE_IDENTITY"
      @package = package
      @paths = paths.freeze
      super("#{@code}: package=#{@package} paths=#{@paths.join(',')}")
    end
  end

  # --------------------------------------------------------------------------
  # Package -- A value object representing a discovered package.
  #
  # Ruby 3.2+ introduced `Data.define` for immutable value objects. It is the
  # closest Ruby equivalent to Python's `@dataclass(frozen=True)`. We use it
  # here so that Package instances are simple, transparent records -- you can
  # pattern-match on them, compare them by value, and inspect them easily.
  #
  # Fields:
  #   name            -- A qualified name like "python/logic-gates".
  #   path            -- Absolute path (Pathname) to the package directory.
  #   build_commands  -- Lines from the BUILD file (commands to execute).
  #   language        -- Inferred language: "python", "ruby", "go", "rust", etc.
  #   build_content   -- Raw BUILD file content (for Starlark detection).
  #   is_starlark     -- Whether the BUILD file uses Starlark syntax.
  #   declared_srcs   -- Glob patterns from the Starlark srcs field.
  #   declared_deps   -- Qualified names from the Starlark deps field.
  # --------------------------------------------------------------------------
  Package = Data.define(
    :name, :path, :build_commands, :language,
    :build_content, :is_starlark, :declared_srcs, :declared_deps
  ) do
    def initialize(name:, path:, build_commands:, language:,
                   build_content: "", is_starlark: false,
                   declared_srcs: [], declared_deps: [])
      super
    end
  end

  module Discovery
    # Canonical repository buckets understood by package discovery. The parity
    # denominator is defined by package_parity_report.py; dotnet is retained as
    # the shared host bucket for .NET programs.
    KNOWN_LANGUAGES = %w[
      csharp dart elixir fsharp go haskell java kotlin lua perl python ruby rust
      swift typescript c cpp ocaml wasm mosaic twig starlark dotnet
    ].freeze

    # SKIP_DIRS is the set of directory names that should never be traversed
    # during package discovery. These are known to contain non-source files
    # (caches, dependencies, build artifacts) that would waste time to scan.
    SKIP_DIRS = Set.new(%w[
      .git .hg .svn .venv .tox .mypy_cache .pytest_cache .ruff_cache
      __pycache__ node_modules vendor dist build target .claude specs Pods
      .dart_tool .build .gradle gradle-build
    ]).freeze

    module_function

    # read_lines -- Read a file and return non-blank, non-comment lines.
    #
    # Blank lines and lines starting with '#' are stripped out. Leading and
    # trailing whitespace is removed from each line. This is the same
    # filtering we use for BUILD files.
    #
    # @param filepath [Pathname] The file to read.
    # @return [Array<String>] The cleaned lines.
    def read_lines(filepath)
      return [] unless filepath.exist?

      filepath.read.lines.map(&:strip).reject { |line| line.empty? || line.start_with?("#") }
    end

    # infer_language -- Infer the programming language from the directory path.
    #
    # The sole discriminator is the exact component immediately below a
    # `packages` or `programs` component. Canonical-looking words elsewhere in
    # the path do not change the result. The final boundary wins so temporary
    # fixture roots nested beneath a checkout retain their own identity.
    #
    # @param path [Pathname] The package directory.
    # @return [String] The inferred language, or "unknown".
    def infer_language(path)
      parts = path.to_s.tr("\\", "/").split("/").reject(&:empty?)
      language = "unknown"
      parts.each_cons(2) do |kind, bucket|
        next unless %w[packages programs].include?(kind)

        language = KNOWN_LANGUAGES.include?(bucket) ? bucket : "unknown"
      end
      language
    end

    # infer_package_name -- Build a qualified package name.
    #
    # Library names follow `{language}/{directory-basename}`. Program names use
    # `{language}/programs/{directory-basename}` so package/program pairs remain
    # distinct graph nodes.
    #
    # @param path [Pathname] The package directory.
    # @param language [String] The inferred language.
    # @return [String] The qualified package name.
    def infer_package_name(path, language)
      parts = path.to_s.tr("\\", "/").split("/").reject(&:empty?)
      kind = nil
      parts.each_cons(2) do |candidate, _bucket|
        kind = candidate if %w[packages programs].include?(candidate)
      end
      infix = kind == "programs" ? "programs/" : ""
      "#{language}/#{infix}#{path.basename}"
    end

    # get_build_file -- Return the appropriate BUILD file for the current platform.
    #
    # Priority (most specific wins):
    #   1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
    #   2. Shared: BUILD_mac_and_linux (macOS or Linux — for Unix-like systems)
    #   3. Generic: BUILD (all platforms)
    #   4. nil if no BUILD file exists
    #
    # This layering lets packages provide Windows-specific build commands via
    # BUILD_windows while sharing a single BUILD_mac_and_linux for the common
    # Unix case, falling back to BUILD when no platform differences exist.
    #
    # We use `RUBY_PLATFORM` to detect the OS. On macOS it contains "darwin";
    # on Linux it contains "linux"; on Windows it contains "mingw" or "mswin".
    #
    # @param directory [Pathname] The directory to check.
    # @return [Pathname, nil] The BUILD file path, or nil.
    def get_build_file(directory)
      os = if RUBY_PLATFORM.include?("darwin")
             "darwin"
           elsif RUBY_PLATFORM.include?("linux")
             "linux"
           elsif RUBY_PLATFORM =~ /mingw|mswin|cygwin/
             "windows"
           else
             "unknown"
           end
      get_build_file_for_platform(directory, os)
    end

    # get_build_file_for_platform -- Like get_build_file but accepts an explicit
    # OS name. This is useful for testing platform-specific behavior without
    # running on that platform.
    #
    # @param directory [Pathname] The directory to check.
    # @param os [String] The OS name: "darwin", "linux", or "windows".
    # @return [Pathname, nil] The BUILD file path, or nil.
    def get_build_file_for_platform(directory, os)
      # Step 1: Check for the most specific platform file.
      if os == "darwin"
        platform_build = directory / "BUILD_mac"
        return platform_build if platform_build.exist?
      end

      if os == "linux"
        platform_build = directory / "BUILD_linux"
        return platform_build if platform_build.exist?
      end

      if os == "windows"
        platform_build = directory / "BUILD_windows"
        return platform_build if platform_build.exist?
      end

      # Step 2: Check for the shared Unix file (macOS + Linux).
      if os == "darwin" || os == "linux"
        shared_build = directory / "BUILD_mac_and_linux"
        return shared_build if shared_build.exist?
      end

      # Step 3: Fall back to the generic BUILD file.
      generic_build = directory / "BUILD"
      return generic_build if generic_build.exist?

      nil
    end

    # discover_packages -- Recursively walk directories, collect packages.
    #
    # Starting from `root`, we list all subdirectories and descend into
    # each one (skipping directories in the skip list). When we find a
    # BUILD file, we register that directory as a package and stop
    # recursing into it.
    #
    # @param root [Pathname] The monorepo root.
    # @return [Array<Package>] Discovered packages, sorted by name.
    def discover_packages(root)
      packages = []
      walk_dirs(root, packages)
      packages.sort_by! { |package| [package.name, package.path.to_s] }

      duplicate = packages.group_by(&:name).find { |_name, group| group.length > 1 }
      if duplicate
        name, group = duplicate
        paths = group.map { |package| repository_package_path(root, package.path) }.sort
        raise DuplicatePackageIdentityError.new(package: name, paths: paths)
      end

      packages
    end

    # Convert a package directory to a stable repository-relative diagnostic.
    # The final code/packages or code/programs boundary is authoritative so a
    # nested temporary checkout cannot disclose its host prefix.
    def repository_package_path(root, path)
      parts = path.to_s.tr("\\", "/").split("/").reject(&:empty?)
      canonical_start = nil
      parts.each_index do |index|
        next unless parts[index] == "code"
        next unless %w[packages programs].include?(parts[index + 1])

        canonical_start = index
      end
      return parts[canonical_start..].join("/") if canonical_start

      path.relative_path_from(root).to_s.tr("\\", "/")
    rescue ArgumentError
      path.basename.to_s
    end

    # walk_dirs -- Recursively walk directories and collect packages.
    #
    # If the current directory's name is in the skip list, ignore it entirely.
    # If the current directory has a BUILD file, it is a package -- register
    # it and stop. Otherwise, list all subdirectories and recurse into each.
    #
    # @param directory [Pathname] The current directory.
    # @param packages [Array<Package>] Accumulator for discovered packages.
    def walk_dirs(directory, packages)
      # Skip known non-source directories.
      return if SKIP_DIRS.include?(directory.basename.to_s)

      build_file = get_build_file(directory)

      if build_file
        # This directory is a package. Read the BUILD commands and raw content.
        commands = read_lines(build_file)
        content = begin
          build_file.read
        rescue StandardError
          ""
        end
        language = infer_language(directory)
        name = infer_package_name(directory, language)

        packages << Package.new(
          name: name,
          path: directory,
          build_commands: commands,
          language: language,
          build_content: content
        )
        return
      end

      # Not a package -- list subdirectories and recurse into each one.
      directory.children.select(&:directory?).sort.each do |child|
        walk_dirs(child, packages)
      end
    rescue Errno::EACCES
      # Permission denied -- skip this directory.
    end
  end
end
