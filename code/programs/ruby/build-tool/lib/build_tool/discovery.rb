# frozen_string_literal: true

# discovery.rb -- Package Discovery via DIRS/BUILD Files
# ======================================================
#
# This module walks a monorepo directory tree following DIRS files to discover
# packages. A "package" is any directory that contains a BUILD file. DIRS files
# act as a routing table: each non-blank, non-comment line names a subdirectory
# to descend into.
#
# The walk is recursive: if `code/DIRS` contains "packages", we look at
# `code/packages/`. If `code/packages/DIRS` contains "python" and "ruby",
# we look at both. When we find a BUILD file in a directory, we stop recursing
# there and register that directory as a package.
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
# Similarly for "ruby" and "go". The package name is `{language}/{dir-name}`.

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
  #   language        -- Inferred language: "python", "ruby", "go", or "unknown".
  # --------------------------------------------------------------------------
  Package = Data.define(:name, :path, :build_commands, :language)

  module Discovery
    # KNOWN_LANGUAGES lists the language directory names we look for when
    # inferring which ecosystem a package belongs to. If a package lives
    # under a directory with one of these names, we tag it accordingly.
    KNOWN_LANGUAGES = %w[python ruby go].freeze

    module_function

    # read_lines -- Read a file and return non-blank, non-comment lines.
    #
    # Blank lines and lines starting with '#' are stripped out. Leading and
    # trailing whitespace is removed from each line. This is the same
    # filtering we use for both DIRS files and BUILD files.
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

    # discover_packages -- Walk DIRS files recursively, collect packages.
    #
    # Starting from `root`, we read the DIRS file (if present) and descend
    # into each listed subdirectory. When we find a BUILD file, we register
    # that directory as a package and stop recursing into it.
    #
    # @param root [Pathname] The monorepo root (where the top-level DIRS is).
    # @return [Array<Package>] Discovered packages, sorted by name.
    def discover_packages(root)
      packages = []
      walk_dirs(root, packages)
      packages.sort_by(&:name)
    end

    # walk_dirs -- Recursively walk DIRS files and collect packages.
    #
    # If the current directory has a BUILD file, it is a package -- register
    # it and stop. Otherwise, if it has a DIRS file, read the listed
    # subdirectories and recurse into each one.
    #
    # @param directory [Pathname] The current directory.
    # @param packages [Array<Package>] Accumulator for discovered packages.
    def walk_dirs(directory, packages)
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

      # Not a package -- look for DIRS file to find subdirectories.
      dirs_file = directory / "DIRS"
      return unless dirs_file.exist?

      subdirs = read_lines(dirs_file)
      subdirs.each do |subdir_name|
        subdir_path = directory / subdir_name
        walk_dirs(subdir_path, packages) if subdir_path.directory?
      end
    end
  end
end
