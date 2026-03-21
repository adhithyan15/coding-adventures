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
  #   language        -- Inferred language: "python", "ruby", "go", "rust", or "unknown".
  # --------------------------------------------------------------------------
  Package = Data.define(:name, :path, :build_commands, :language)

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
      __pycache__ node_modules vendor dist build target .claude Pods
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
    # Priority:
    #   1. BUILD_mac on macOS, BUILD_linux on Linux
    #   2. BUILD (fallback)
    #   3. nil if no BUILD file exists
    #
    # We use `RUBY_PLATFORM` to detect the OS. On macOS it contains "darwin";
    # on Linux it contains "linux".
    #
    # @param directory [Pathname] The directory to check.
    # @return [Pathname, nil] The BUILD file path, or nil.
    def get_build_file(directory)
      if RUBY_PLATFORM.include?("darwin")
        platform_build = directory / "BUILD_mac"
        return platform_build if platform_build.exist?
      end

      if RUBY_PLATFORM.include?("linux")
        platform_build = directory / "BUILD_linux"
        return platform_build if platform_build.exist?
      end

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
        # This directory is a package. Read the BUILD commands.
        commands = read_lines(build_file)
        language = infer_language(directory)
        name = infer_package_name(directory, language)

        packages << Package.new(
          name: name,
          path: directory,
          build_commands: commands,
          language: language
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
