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
# We infer the language from the directory path. If the path contains
# `packages/python/X` or `programs/python/X`, the language is "python".
# Similarly for "ruby", "go", and "rust". The package name is
# `{language}/{dir-name}`.

module BuildTool
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
    # KNOWN_LANGUAGES lists the language directory names we look for when
    # inferring which ecosystem a package belongs to. If a package lives
    # under a directory with one of these names, we tag it accordingly.
    KNOWN_LANGUAGES = %w[python ruby go rust typescript elixir].freeze

    # SKIP_DIRS is the set of directory names that should never be traversed
    # during package discovery. These are known to contain non-source files
    # (caches, dependencies, build artifacts) that would waste time to scan.
    SKIP_DIRS = Set.new(%w[
      .git .hg .svn .venv .tox .mypy_cache .pytest_cache .ruff_cache
      __pycache__ node_modules vendor dist build target .claude Pods .gradle gradle-build
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
    # We split the path into its component parts and look for a known language
    # directory name. The first match wins. For example, a path like
    # `/repo/code/packages/python/logic-gates` yields "python".
    #
    # @param path [Pathname] The package directory.
    # @return [String] The inferred language, or "unknown".
    def infer_language(path)
      parts = path.to_s.split(File::SEPARATOR)
      KNOWN_LANGUAGES.find { |lang| parts.include?(lang) } || "unknown"
    end

    # infer_package_name -- Build a qualified package name.
    #
    # The name follows the pattern `{language}/{directory-basename}`. For
    # instance, if language is "python" and the directory is "logic-gates",
    # the name becomes "python/logic-gates".
    #
    # @param path [Pathname] The package directory.
    # @param language [String] The inferred language.
    # @return [String] The qualified package name.
    def infer_package_name(path, language)
      "#{language}/#{path.basename}"
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
      packages.sort_by(&:name)
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
