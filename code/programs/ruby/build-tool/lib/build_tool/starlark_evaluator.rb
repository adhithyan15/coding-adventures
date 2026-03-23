# frozen_string_literal: true

require "etc"
require "set"

# ==========================================================================
# starlark_evaluator.rb -- Starlark BUILD File Evaluation
# ==========================================================================
#
# This module evaluates Starlark BUILD files using the Ruby starlark_interpreter
# package. It bridges the gap between declarative Starlark target definitions
# and the shell commands that the build tool's Executor actually runs.
#
# == Why Starlark BUILD files?
#
# Traditional BUILD files in this monorepo are plain shell scripts -- each
# line is a command executed sequentially. This works but has limitations:
#
#   - No change detection metadata: the build tool guesses which files
#     matter based on extensions, not explicit declarations.
#   - No dependency declarations: deps are parsed from language-specific
#     config files (pyproject.toml, .gemspec, go.mod) with heuristic matching.
#   - No validation: a typo in a BUILD file only surfaces at build time.
#
# Starlark BUILD files solve all three. They are real programs that declare
# targets with explicit srcs, deps, and build metadata. The build tool
# evaluates them using the starlark_interpreter gem and extracts the
# declared targets.
#
# == How evaluation works
#
# The pipeline follows five steps:
#
#   1. Read the BUILD file contents.
#   2. Create a Starlark interpreter with a file_resolver rooted at the
#      repo root (for load() statements).
#   3. Execute the BUILD file through the interpreter pipeline:
#        source -> lexer -> parser -> compiler -> VM -> result
#   4. Extract the _targets list from the result's variables.
#   5. Convert each target dict to a Target struct.
#
# == Detecting Starlark vs shell BUILD files
#
# We use a simple heuristic: if the BUILD file's first non-comment,
# non-blank line starts with "load(" or matches a known rule call pattern
# (like "py_library("), it's Starlark. Otherwise it's shell.
#
# This is the same detection logic used by the Go build tool's
# starlark/evaluator.go -- a direct Ruby port of IsStarlarkBuild().
#
# == Command generation
#
# Each rule type (py_library, go_binary, ruby_library, etc.) maps to a
# standard set of shell commands. For example:
#
#   py_library  ->  uv pip install + pytest
#   go_library  ->  go build + go test + go vet
#   ruby_library -> bundle install + rake test
#
# This mapping is identical to the Go build tool's GenerateCommands()
# function, ensuring consistent behavior regardless of which build tool
# implementation is used.
# ==========================================================================

module BuildTool
  # --------------------------------------------------------------------------
  # Target -- A single build target declared in a Starlark BUILD file.
  #
  # Each call to py_library(), go_library(), etc. in a Starlark BUILD file
  # produces one Target. We use Data.define (Ruby 3.2+) for immutable value
  # semantics, matching the convention used by Package and BuildResult
  # elsewhere in the build tool.
  #
  # Fields:
  #   rule        -- Rule type: "py_library", "go_binary", etc.
  #   name        -- Target name: "starlark-vm", "build-tool", etc.
  #   srcs        -- Declared source file patterns for change detection.
  #   deps        -- Dependencies as "language/package-name" strings.
  #   test_runner -- Test framework: "pytest", "vitest", "minitest", etc.
  #   entry_point -- Binary entry point: "main.py", "src/index.ts", etc.
  #
  # Example:
  #   target = Target.new(
  #     rule: "py_library",
  #     name: "logic-gates",
  #     srcs: ["src/**/*.py"],
  #     deps: ["python/grammar-tools"],
  #     test_runner: "pytest",
  #     entry_point: ""
  #   )
  # --------------------------------------------------------------------------
  Target = Data.define(:rule, :name, :srcs, :deps, :test_runner, :entry_point, :commands) do
    def initialize(rule:, name:, srcs: [], deps: [], test_runner: "", entry_point: "", commands: [])
      super
    end
  end

  # --------------------------------------------------------------------------
  # BuildFileResult -- The targets extracted from evaluating a Starlark BUILD
  # file. Wraps an array of Target structs.
  #
  # Why a separate struct instead of a bare Array? Two reasons:
  #   1. It mirrors the Go implementation's BuildResult struct.
  #   2. It provides a place to attach future metadata (e.g., warnings,
  #      evaluation time, source maps for error reporting).
  # --------------------------------------------------------------------------
  BuildFileResult = Data.define(:targets) do
    def initialize(targets: [])
      super
    end
  end

  module StarlarkEvaluator
    # Schema version for the _ctx build context dict.
    CTX_SCHEMA_VERSION = 1

    # OS name normalization: Gem::Platform.local.os -> runtime.GOOS equivalents.
    OS_MAP = {"darwin" => "darwin", "linux" => "linux", "mingw32" => "windows"}.freeze

    # Characters that trigger quoting in shell strings.
    SHELL_META = Set.new(" \t\"'$`\\|&;()<>!#*?[]{}".chars).freeze

    # KNOWN_RULES lists the Starlark rule function call patterns that indicate
    # a BUILD file contains Starlark code rather than shell commands.
    #
    # Each entry is a string like "py_library(" -- we check if a line starts
    # with any of these patterns. The list covers all six supported languages.
    KNOWN_RULES = %w[
      py_library( py_binary(
      go_library( go_binary(
      ruby_library( ruby_binary(
      ts_library( ts_binary(
      rust_library( rust_binary(
      elixir_library( elixir_binary(
    ].freeze

    module_function

    # starlark_build? -- Detect whether a BUILD file contains Starlark code.
    #
    # We scan lines from the top, skipping blanks and comments. The first
    # "significant" line determines the format:
    #
    #   - Starts with "load("        -> Starlark (import statement)
    #   - Starts with "def "         -> Starlark (function definition)
    #   - Starts with a known rule   -> Starlark (target declaration)
    #   - Anything else              -> Shell (traditional BUILD file)
    #
    # Truth table for common cases:
    #
    #   First significant line          | Result
    #   --------------------------------|--------
    #   load("//rules.star", "py_lib")  | true
    #   def custom_rule(name):          | true
    #   py_library(name = "foo")        | true
    #   echo "hello world"              | false
    #   pip install -e .                | false
    #   bundle exec rake test           | false
    #
    # @param content [String] The raw BUILD file contents.
    # @return [Boolean] True if the file appears to be Starlark.
    def starlark_build?(content)
      content.each_line do |line|
        trimmed = line.strip

        # Skip blank lines and comments -- they don't tell us anything.
        next if trimmed.empty? || trimmed.start_with?("#")

        # Check for Starlark-specific patterns.
        return true if trimmed.start_with?("load(")
        return true if trimmed.start_with?("def ")

        KNOWN_RULES.each do |rule|
          return true if trimmed.start_with?(rule)
        end

        # If the first significant line doesn't match any Starlark pattern,
        # it's almost certainly a shell command. Stop checking -- we only
        # need the first line to decide.
        break
      end

      false
    end

    # evaluate_build_file -- Run the Starlark interpreter on a BUILD file
    # and extract the declared targets.
    #
    # The evaluation process:
    #
    #   1. Read the BUILD file from disk.
    #   2. Create a file_resolver lambda that resolves load() paths relative
    #      to the repo root. For example:
    #        load("code/packages/starlark/library-rules/python_library.star", "py_library")
    #      resolves to <repo_root>/code/packages/starlark/library-rules/python_library.star
    #   3. Create an interpreter with the file_resolver.
    #   4. Execute the BUILD file through the full pipeline.
    #   5. Extract _targets from the result's variables.
    #   6. Convert each target dict to a Target struct.
    #
    # @param build_file_path [String] Path to the BUILD file.
    # @param pkg_dir [String] Package directory (for future glob() support).
    # @param repo_root [String] Repository root (for resolving load() paths).
    # @return [BuildFileResult] The extracted targets.
    # @raise [RuntimeError] If the file cannot be read or evaluated.
    def evaluate_build_file(build_file_path, pkg_dir, repo_root)
      # Step 1: Read the BUILD file.
      content = File.read(build_file_path)

      # Step 2: Create a file resolver for load() statements.
      #
      # The resolver takes a label string (the first argument to load()) and
      # returns the file contents as a string. Labels are filesystem paths
      # relative to the repo root.
      #
      # Example:
      #   load("code/packages/starlark/library-rules/python_library.star", "py_library")
      #   -> reads <repo_root>/code/packages/starlark/library-rules/python_library.star
      file_resolver = ->(label) {
        full_path = File.join(repo_root, label)
        if File.exist?(full_path)
          File.read(full_path)
        else
          nil
        end
      }

      # Step 3: Create the interpreter and execute.
      #
      # We lazy-require the starlark_interpreter gem here rather than at
      # the top of the file. This keeps the build tool functional for
      # shell BUILD files even when the interpreter gem isn't installed.
      require "coding_adventures_starlark_interpreter"

      # Build the _ctx dict — the build context injected into every Starlark
      # scope.  See spec 15 for the full schema.
      os_name = Gem::Platform.local.os
      ctx_dict = {
        "version" => CTX_SCHEMA_VERSION,
        "os" => OS_MAP.fetch(os_name, os_name),
        "arch" => RbConfig::CONFIG["target_cpu"],
        "cpu_count" => Etc.respond_to?(:nprocessors) ? Etc.nprocessors : 1,
        "ci" => !ENV.fetch("CI", "").empty?,
        "repo_root" => repo_root
      }

      # We use the CodingAdventures::StarlarkInterpreter module-level
      # convenience method, which creates an Interpreter instance with
      # the file_resolver and globals, and runs the full pipeline.
      result = CodingAdventures::StarlarkInterpreter.interpret(
        content,
        file_resolver: file_resolver,
        globals: {"_ctx" => ctx_dict}
      )

      # Step 4: Extract targets from the result.
      targets = extract_targets(result.variables)

      BuildFileResult.new(targets: targets)
    rescue Errno::ENOENT => e
      raise "reading BUILD file: #{e.message}"
    rescue StandardError => e
      raise "evaluating BUILD file #{build_file_path}: #{e.message}"
    end

    # generate_commands -- Convert a Target into shell commands.
    #
    # Each rule type maps to a standard set of commands that the build tool's
    # Executor can run. This is the Ruby equivalent of the Go build tool's
    # GenerateCommands() function -- the mappings are identical to ensure
    # consistent behavior across build tool implementations.
    #
    # Rule-to-command mapping:
    #
    #   Rule             | Commands
    #   -----------------|-------------------------------------------------
    #   py_library       | uv pip install + pytest (or unittest)
    #   py_binary        | uv pip install + pytest
    #   go_library/bin   | go build + go test + go vet
    #   ruby_library/bin | bundle install + rake test
    #   ts_library/bin   | npm install + vitest
    #   rust_library/bin | cargo build + cargo test
    #   elixir_lib/bin   | mix deps.get + mix test
    #
    # @param target [Target] The target to generate commands for.
    # @return [Array<String>] Shell commands to execute.
    def generate_commands(target)
      case target.rule
      when "py_library"
        runner = target.test_runner.empty? ? "pytest" : target.test_runner
        if runner == "pytest"
          [
            'uv pip install --system -e ".[dev]"',
            "python -m pytest --cov --cov-report=term-missing"
          ]
        else
          [
            'uv pip install --system -e ".[dev]"',
            "python -m unittest discover tests/"
          ]
        end

      when "py_binary"
        [
          'uv pip install --system -e ".[dev]"',
          "python -m pytest --cov --cov-report=term-missing"
        ]

      when "go_library", "go_binary"
        [
          "go build ./...",
          "go test ./... -v -cover",
          "go vet ./..."
        ]

      when "ruby_library", "ruby_binary"
        [
          "bundle install --quiet",
          "bundle exec rake test"
        ]

      when "ts_library", "ts_binary"
        [
          "npm install --silent",
          "npx vitest run --coverage"
        ]

      when "rust_library", "rust_binary"
        [
          "cargo build",
          "cargo test"
        ]

      when "elixir_library", "elixir_binary"
        [
          "mix deps.get",
          "mix test --cover"
        ]

      else
        ["echo 'Unknown rule: #{target.rule}'"]
      end
    end

    # -- Private helpers -------------------------------------------------------

    # extract_targets -- Convert the _targets list from interpreter variables
    # into an array of Target structs.
    #
    # Starlark BUILD files declare targets by calling rule functions like
    # py_library() and go_binary(). Those functions append dicts to a global
    # _targets list. Each dict has keys: rule, name, srcs, deps, and
    # optionally test_runner and entry_point.
    #
    # If _targets doesn't exist (e.g., a BUILD file that only defines helper
    # functions), we return an empty array. This is valid and not an error.
    #
    # @param variables [Hash] The interpreter result's variable namespace.
    # @return [Array<Target>] The extracted targets.
    def extract_targets(variables)
      raw_targets = variables["_targets"]

      # No _targets variable -- the BUILD file didn't declare any targets.
      return [] if raw_targets.nil?

      unless raw_targets.is_a?(Array)
        raise "_targets is not a list (got #{raw_targets.class})"
      end

      raw_targets.each_with_index.map do |raw, i|
        unless raw.is_a?(Hash)
          raise "_targets[#{i}] is not a dict (got #{raw.class})"
        end

        Target.new(
          rule: get_string(raw, "rule"),
          name: get_string(raw, "name"),
          srcs: get_string_list(raw, "srcs"),
          deps: get_string_list(raw, "deps"),
          test_runner: get_string(raw, "test_runner"),
          entry_point: get_string(raw, "entry_point"),
          commands: get_dict_list(raw, "commands")
        )
      end
    end

    # get_string -- Safely extract a string value from a hash.
    #
    # Returns "" if the key doesn't exist or the value isn't a String.
    # This defensive approach means malformed target dicts produce empty
    # fields rather than crashing the build tool.
    #
    # @param hash [Hash] The target dict.
    # @param key [String] The key to look up.
    # @return [String] The value, or "".
    def get_string(hash, key)
      value = hash[key]
      value.is_a?(String) ? value : ""
    end

    # get_string_list -- Safely extract an Array<String> from a hash.
    #
    # Returns [] if the key doesn't exist or the value isn't an Array.
    # Non-string elements within the array are silently skipped.
    #
    # @param hash [Hash] The target dict.
    # @param key [String] The key to look up.
    # @return [Array<String>] The string values, or [].
    def get_string_list(hash, key)
      value = hash[key]
      return [] unless value.is_a?(Array)

      value.select { |item| item.is_a?(String) }
    end

    def get_dict_list(hash, key)
      value = hash[key]
      return [] unless value.is_a?(Array)

      value.select { |item| item.is_a?(Hash) }
    end

    # -- Command Rendering ---------------------------------------------------

    # render_command -- Convert a single command dict to a shell string.
    #
    # @param cmd_dict [Hash] A command dict with "program" and optional "args".
    # @return [String] A shell-safe command string.
    def render_command(cmd_dict)
      program = cmd_dict["program"]
      raise "command dict missing 'program' key: #{cmd_dict}" unless program.is_a?(String) && !program.empty?

      parts = [quote_arg(program)]
      args = cmd_dict["args"]
      if args.is_a?(Array)
        args.each { |arg| parts << quote_arg(arg.to_s) }
      end
      parts.join(" ")
    end

    # render_commands -- Convert a list of command dicts to shell strings.
    #
    # @param cmds [Array] List of command dicts (nils are skipped).
    # @return [Array<String>] Shell command strings.
    def render_commands(cmds)
      cmds.filter_map { |cmd| render_command(cmd) if cmd.is_a?(Hash) }
    end

    def quote_arg(arg)
      return '""' if arg.empty?
      return arg unless arg.chars.any? { |c| SHELL_META.include?(c) }

      escaped = arg.gsub('\\', '\\\\\\\\').gsub('"', '\\"')
      "\"#{escaped}\""
    end
  end
end
