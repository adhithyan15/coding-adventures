# frozen_string_literal: true

# generator.rb -- Main scaffold generation logic
# ================================================
#
# === What This File Does ===
#
# This file contains all the logic for generating CI-ready package scaffolding
# across all six languages. It handles:
#
#   1. Name normalization (kebab-case to snake_case, CamelCase, joinedlower)
#   2. Dependency reading from existing package metadata files
#   3. Transitive closure computation via BFS
#   4. Topological sort via Kahn's algorithm for install ordering
#   5. File generation for Python, Go, Ruby, TypeScript, Rust, and Elixir
#
# === Why This Tool Exists ===
#
# The lessons.md file documents 12+ recurring categories of CI failures
# caused by agents hand-crafting packages inconsistently:
#
#   - Missing BUILD files
#   - TypeScript "main" pointing to dist/ instead of src/
#   - Missing transitive dependency installs in BUILD files
#   - Ruby require ordering (deps before own modules)
#   - Rust workspace Cargo.toml not updated
#
# This tool eliminates those failures. Run it, get a package that compiles,
# lints, and passes tests. Then fill in the business logic.

require "json"
require "date"
require "fileutils"

module CodingAdventures
  module ScaffoldGenerator
    # =====================================================================
    # Constants
    # =====================================================================

    # The list of all supported target languages.
    VALID_LANGUAGES = %w[python go ruby typescript rust elixir].freeze

    # Validates that a package name is kebab-case: lowercase letters and
    # digits, segments separated by single hyphens.
    #
    # Examples of valid names:   "my-package", "logic-gates", "alu"
    # Examples of invalid names: "MyPackage", "my_package", "-bad"
    KEBAB_RE = /\A[a-z][a-z0-9]*(-[a-z0-9]+)*\z/

    # =====================================================================
    # Name Normalization
    # =====================================================================
    #
    # The input package name is always kebab-case (e.g., "my-package").
    # Each language has different naming conventions:
    #
    #   kebab-case:   my-package    (used by Go dirs, TS dirs, Rust dirs)
    #   snake_case:   my_package    (used by Python, Ruby, Elixir)
    #   CamelCase:    MyPackage     (used by Ruby modules, Elixir modules)
    #   joinedlower:  mypackage     (used by Go package names)
    #
    # These functions convert between them.

    # Convert "my-package" to "my_package".
    #
    # @param kebab [String] a kebab-case name
    # @return [String] the snake_case equivalent
    def self.to_snake_case(kebab)
      kebab.tr("-", "_")
    end

    # Convert "my-package" to "MyPackage".
    #
    # Each hyphen-separated segment gets its first letter capitalized,
    # then they are joined together. For example:
    #
    #   "logic-gates" => "LogicGates"
    #   "alu"         => "Alu"
    #   "my-cool-pkg" => "MyCoolPkg"
    #
    # @param kebab [String] a kebab-case name
    # @return [String] the CamelCase equivalent
    def self.to_camel_case(kebab)
      kebab.split("-").map(&:capitalize).join
    end

    # Convert "my-package" to "mypackage" (Go package name convention).
    #
    # Go package names must be a single lowercase word, no hyphens or
    # underscores. We just remove all hyphens.
    #
    # @param kebab [String] a kebab-case name
    # @return [String] the joined-lowercase equivalent
    def self.to_joined_lower(kebab)
      kebab.delete("-")
    end

    # Return the directory name for a package in a given language.
    #
    # Ruby and Elixir use snake_case directory names (e.g., "my_package").
    # All other languages use kebab-case (e.g., "my-package").
    #
    # @param kebab [String] the kebab-case package name
    # @param lang [String] the target language
    # @return [String] the directory name
    def self.dir_name(kebab, lang)
      case lang
      when "ruby", "elixir"
        to_snake_case(kebab)
      else
        kebab
      end
    end

    # =====================================================================
    # Dependency Resolution
    # =====================================================================
    #
    # Each language stores dependency information in different files and
    # formats. These readers extract the list of local sibling dependencies
    # from each language's metadata files.
    #
    # The dependency names are always returned in kebab-case, regardless of
    # what the metadata file uses internally. This normalization makes it
    # possible to do cross-language dependency resolution with a single
    # algorithm.

    # Read direct local dependencies of a package from its metadata files.
    #
    # @param pkg_dir [String] absolute path to the package directory
    # @param lang [String] one of the VALID_LANGUAGES
    # @return [Array<String>] kebab-case dependency names
    def self.read_deps(pkg_dir, lang)
      case lang
      when "python"  then read_python_deps(pkg_dir)
      when "go"      then read_go_deps(pkg_dir)
      when "ruby"    then read_ruby_deps(pkg_dir)
      when "typescript" then read_ts_deps(pkg_dir)
      when "rust"    then read_rust_deps(pkg_dir)
      when "elixir"  then read_elixir_deps(pkg_dir)
      else []
      end
    end

    # Parse a Python BUILD file for `-e ../dep-name` entries.
    #
    # Python BUILD files install local dependencies with pip's editable
    # mode: `pip install -e ../dep-name`. We scan each line for this
    # pattern and extract the directory name after `../`.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_python_deps(pkg_dir)
      build_path = File.join(pkg_dir, "BUILD")
      return [] unless File.exist?(build_path)

      deps = []
      File.readlines(build_path).each do |line|
        # Find ALL -e ../ entries on each line (new format puts them all on one line)
        remaining = line
        loop do
          idx = remaining.index("-e ../")
          idx = remaining.index('-e "../') if idx.nil?
          break if idx.nil?

          rest = remaining[idx..]
          if rest.start_with?('-e "../')
            rest = rest[7..] # skip `-e "../`
          else
            rest = rest[6..] # skip `-e ../`
          end
          dep = ""
          rest.each_char do |c|
            break if [" ", '"', "'", "\n"].include?(c)

            dep += c
          end
          deps << dep if !dep.empty? && dep != "."
          remaining = remaining[(idx + 6)..]
        end
      end
      deps
    end

    # Parse a Go go.mod file for `=> ../dep` replace directives.
    #
    # Go modules use `replace` directives to point to local siblings:
    #   replace github.com/.../dep => ../dep
    # We extract the directory name after `=> ../`.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_go_deps(pkg_dir)
      mod_path = File.join(pkg_dir, "go.mod")
      return [] unless File.exist?(mod_path)

      deps = []
      File.readlines(mod_path).each do |line|
        if line.include?("=> ../")
          idx = line.index("=> ../")
          rest = line[(idx + 6)..].strip
          dep = rest.split(/\s/).first || ""
          deps << dep unless dep.empty?
        end
      end
      deps
    end

    # Parse a Ruby Gemfile for `path: "../dep"` entries.
    #
    # Ruby Gemfiles reference local siblings with:
    #   gem "coding_adventures_dep_name", path: "../dep_name"
    # We extract the directory name after `../` and convert underscores
    # to hyphens to normalize back to kebab-case.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_ruby_deps(pkg_dir)
      gemfile_path = File.join(pkg_dir, "Gemfile")
      return [] unless File.exist?(gemfile_path)

      deps = []
      File.readlines(gemfile_path).each do |line|
        if line.include?("path:") && line.include?('"../')
          idx = line.index('"../')
          rest = line[(idx + 4)..]
          dep = ""
          rest.each_char do |c|
            break if c == '"'

            dep += c
          end
          dep = dep.tr("_", "-")
          deps << dep unless dep.empty?
        end
      end
      deps
    end

    # Parse a TypeScript package.json for `file:../dep` dependency values.
    #
    # TypeScript packages reference local siblings with:
    #   "@coding-adventures/dep": "file:../dep"
    # We extract the directory name after `file:../`.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_ts_deps(pkg_dir)
      pkg_json_path = File.join(pkg_dir, "package.json")
      return [] unless File.exist?(pkg_json_path)

      begin
        pkg = JSON.parse(File.read(pkg_json_path))
      rescue JSON::ParserError
        return []
      end

      deps_obj = pkg.fetch("dependencies", {})
      deps = []
      deps_obj.each_value do |val|
        if val.is_a?(String) && val.start_with?("file:../")
          dep = val.delete_prefix("file:../")
          deps << dep unless dep.empty?
        end
      end
      deps
    end

    # Parse a Rust Cargo.toml for `path = "../dep"` entries.
    #
    # Rust crates reference local siblings with:
    #   dep-name = { path = "../dep-name" }
    # We extract the directory name after `path = "../`.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_rust_deps(pkg_dir)
      cargo_path = File.join(pkg_dir, "Cargo.toml")
      return [] unless File.exist?(cargo_path)

      deps = []
      File.readlines(cargo_path).each do |line|
        if line.include?('path = "../')
          idx = line.index('path = "../')
          rest = line[(idx + 11)..]
          dep = ""
          rest.each_char do |c|
            break if c == '"'

            dep += c
          end
          deps << dep unless dep.empty?
        end
      end
      deps
    end

    # Parse an Elixir mix.exs for `path: "../dep"` entries.
    #
    # Elixir projects reference local siblings with:
    #   {:coding_adventures_dep_name, path: "../dep_name"}
    # We extract the directory name after `path: "../` and convert
    # underscores to hyphens to normalize back to kebab-case.
    #
    # @param pkg_dir [String] path to the package directory
    # @return [Array<String>] kebab-case dependency names
    def self.read_elixir_deps(pkg_dir)
      mix_path = File.join(pkg_dir, "mix.exs")
      return [] unless File.exist?(mix_path)

      deps = []
      File.readlines(mix_path).each do |line|
        if line.include?('path: "../')
          idx = line.index('path: "../')
          rest = line[(idx + 10)..]
          dep = ""
          rest.each_char do |c|
            break if c == '"'

            dep += c
          end
          dep = dep.tr("_", "-")
          deps << dep unless dep.empty?
        end
      end
      deps
    end

    # =====================================================================
    # Transitive Closure (BFS)
    # =====================================================================
    #
    # Given a list of direct dependencies, compute ALL transitive
    # dependencies using breadth-first search. For example, if A depends
    # on B, and B depends on C, then transitive_closure([A]) returns
    # [A, B, C] (sorted).
    #
    # This is critical for BUILD files, which must install every transitive
    # dependency before running tests. Missing even one causes CI failures.

    # Compute all transitive dependencies via BFS.
    #
    # @param direct_deps [Array<String>] direct dependency names (kebab-case)
    # @param lang [String] one of the VALID_LANGUAGES
    # @param base_dir [String] path to the packages directory for this language
    # @return [Array<String>] sorted list of all transitive dependencies
    def self.transitive_closure(direct_deps, lang, base_dir)
      visited = Set.new
      queue = direct_deps.dup

      until queue.empty?
        dep = queue.shift
        next if visited.include?(dep)

        visited.add(dep)
        dep_dir = File.join(base_dir, dir_name(dep, lang))
        read_deps(dep_dir, lang).each do |dd|
          queue << dd unless visited.include?(dd)
        end
      end

      visited.to_a.sort
    end

    # =====================================================================
    # Topological Sort (Kahn's Algorithm)
    # =====================================================================
    #
    # Once we know all transitive dependencies, we need to install them in
    # the right order: leaves first, roots last. If C has no deps, B depends
    # on C, and A depends on B, the install order must be [C, B, A].
    #
    # Kahn's algorithm works by:
    #   1. Compute in-degree for each node (how many deps it has within the set)
    #   2. Start with nodes that have in-degree 0 (leaves)
    #   3. Process each leaf, reducing in-degree of nodes that depend on it
    #   4. Repeat until all nodes are processed
    #
    # If the result has fewer items than the input, there is a cycle.

    # Return dependencies in leaf-first (install) order via Kahn's algorithm.
    #
    # @param all_deps [Array<String>] all transitive dependency names
    # @param lang [String] one of the VALID_LANGUAGES
    # @param base_dir [String] path to the packages directory for this language
    # @return [Array<String>] dependencies in install order (leaves first)
    # @raise [RuntimeError] if a circular dependency is detected
    def self.topological_sort(all_deps, lang, base_dir)
      dep_set = all_deps.to_set

      # Build graph: for each dep, list what it depends on (within our set)
      graph = {}
      all_deps.each do |dep|
        dep_dir = File.join(base_dir, dir_name(dep, lang))
        graph[dep] = read_deps(dep_dir, lang).select { |dd| dep_set.include?(dd) }
      end

      # In-degree: how many deps does this node have within the set
      in_degree = {}
      all_deps.each { |dep| in_degree[dep] = graph[dep].size }

      # Start with leaves (in_degree == 0), sorted for deterministic output
      queue = all_deps.select { |dep| in_degree[dep].zero? }.sort
      result = []

      until queue.empty?
        node = queue.shift
        result << node
        # Decrease in-degree for nodes that depend on this one
        all_deps.each do |dep|
          if graph[dep].include?(node)
            in_degree[dep] -= 1
            if in_degree[dep].zero?
              queue << dep
              queue.sort!
            end
          end
        end
      end

      if result.size != all_deps.size
        raise "circular dependency detected: resolved #{result.size} of #{all_deps.size}"
      end

      result
    end

    # =====================================================================
    # File Writing Helper
    # =====================================================================

    # Write content to a file, creating parent directories as needed.
    #
    # @param path [String] the file path to write to
    # @param content [String] the content to write
    def self.write_file(path, content)
      FileUtils.mkdir_p(File.dirname(path))
      File.write(path, content)
    end

    # =====================================================================
    # Python File Generation
    # =====================================================================

    # Generate all files for a Python package.
    #
    # Python packages use:
    #   - pyproject.toml for metadata (hatchling build system)
    #   - src/<snake>/__init__.py for the package entry point
    #   - tests/test_<snake>.py for Minitest equivalent (pytest)
    #   - BUILD file that installs deps with uv, then runs pytest
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    # @param ordered_deps [Array<String>] all deps in install order
    def self.generate_python(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      snake = to_snake_case(pkg_name)

      pyproject = <<~TOML
        [build-system]
        requires = ["hatchling"]
        build-backend = "hatchling.build"

        [project]
        name = "coding-adventures-#{pkg_name}"
        version = "0.1.0"
        description = "#{description}"
        requires-python = ">=3.12"
        license = "MIT"
        authors = [{ name = "Adhithya Rajasekaran" }]
        readme = "README.md"

        [project.optional-dependencies]
        dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

        [tool.hatch.build.targets.wheel]
        packages = ["src/#{snake}"]

        [tool.ruff]
        target-version = "py312"
        line-length = 88

        [tool.ruff.lint]
        select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

        [tool.pytest.ini_options]
        testpaths = ["tests"]
        addopts = "--cov=#{snake} --cov-report=term-missing --cov-fail-under=80"

        [tool.coverage.run]
        source = ["src/#{snake}"]

        [tool.coverage.report]
        fail_under = 80
        show_missing = true
      TOML

      init_py = <<~PY
        """#{pkg_name} — #{description}

        This package is part of the coding-adventures monorepo, a ground-up
        implementation of the computing stack from transistors to operating systems.
        #{layer_ctx}"""

        __version__ = "0.1.0"
      PY

      test_py = <<~PY
        """Tests for #{pkg_name}."""

        from #{snake} import __version__


        class TestVersion:
            """Verify the package is importable and has a version."""

            def test_version_exists(self) -> None:
                assert __version__ == "0.1.0"
      PY

      install_parts = ["pip install"]
      ordered_deps.each { |dep| install_parts << "-e ../#{dep}" }
      install_parts.push("-e .[dev]", "--quiet")
      build_lines = [install_parts.join(" ")]
      build_lines << "python -m pytest tests/ -v"
      build = build_lines.join("\n") + "\n"

      write_file(File.join(target_dir, "pyproject.toml"), pyproject)
      write_file(File.join(target_dir, "src", snake, "__init__.py"), init_py)
      write_file(File.join(target_dir, "tests", "__init__.py"), "")
      write_file(File.join(target_dir, "tests", "test_#{snake}.py"), test_py)
      write_file(File.join(target_dir, "BUILD"), build)
    end

    # =====================================================================
    # Go File Generation
    # =====================================================================

    # Generate all files for a Go package.
    #
    # Go packages use:
    #   - go.mod for module definition and dependency management
    #   - <snake>.go for the package source
    #   - <snake>_test.go for tests
    #   - BUILD file that runs `go test ./... -v -cover`
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    # @param all_deps [Array<String>] all transitive dependencies
    def self.generate_go(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)
      go_pkg = to_joined_lower(pkg_name)
      snake = to_snake_case(pkg_name)

      go_mod = "module github.com/adhithyan15/coding-adventures/code/packages/go/#{pkg_name}\n\ngo 1.26\n"
      if direct_deps.any?
        go_mod += "\nrequire (\n"
        direct_deps.each do |dep|
          go_mod += "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/#{dep} v0.0.0\n"
        end
        go_mod += ")\n\nreplace (\n"
        all_deps.each do |dep|
          go_mod += "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/#{dep} => ../#{dep}\n"
        end
        go_mod += ")\n"
      end

      src = <<~GO
        // Package #{go_pkg} provides #{description}.
        //
        // This package is part of the coding-adventures monorepo, a ground-up
        // implementation of the computing stack from transistors to operating systems.
        // #{layer_ctx}
        package #{go_pkg}
      GO

      test = <<~GO
        package #{go_pkg}

        import "testing"

        func TestPackageLoads(t *testing.T) {
        \tt.Log("#{pkg_name} package loaded successfully")
        }
      GO

      write_file(File.join(target_dir, "go.mod"), go_mod)
      write_file(File.join(target_dir, "#{snake}.go"), src)
      write_file(File.join(target_dir, "#{snake}_test.go"), test)
      write_file(File.join(target_dir, "BUILD"), "go test ./... -v -cover\n")
    end

    # =====================================================================
    # Ruby File Generation
    # =====================================================================

    # Generate all files for a Ruby package.
    #
    # Ruby packages use:
    #   - <gem_name>.gemspec for gem metadata
    #   - Gemfile with path dependencies for local siblings
    #   - Rakefile for running tests via minitest
    #   - lib/<gem_name>.rb as entry point (requires deps FIRST)
    #   - lib/coding_adventures/<snake>/version.rb for VERSION constant
    #   - test/test_<snake>.rb for minitest tests
    #   - BUILD: `bundle install --quiet && bundle exec rake test`
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    # @param all_deps [Array<String>] all transitive dependencies
    def self.generate_ruby(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)
      snake = to_snake_case(pkg_name)
      camel = to_camel_case(pkg_name)

      gemspec = "# frozen_string_literal: true\n\n"
      gemspec += "require_relative \"lib/coding_adventures/#{snake}/version\"\n\n"
      gemspec += "Gem::Specification.new do |spec|\n"
      gemspec += "  spec.name          = \"coding_adventures_#{snake}\"\n"
      gemspec += "  spec.version       = CodingAdventures::#{camel}::VERSION\n"
      gemspec += "  spec.authors       = [\"Adhithya Rajasekaran\"]\n"
      gemspec += "  spec.summary       = \"#{description}\"\n"
      gemspec += "  spec.homepage      = \"https://github.com/adhithyan15/coding-adventures\"\n"
      gemspec += "  spec.license       = \"MIT\"\n"
      gemspec += "  spec.required_ruby_version = \">= 3.3.0\"\n\n"
      gemspec += "  spec.files         = Dir[\"lib/**/*.rb\", \"README.md\", \"CHANGELOG.md\"]\n"
      gemspec += "  spec.require_paths = [\"lib\"]\n\n"
      gemspec += "  spec.metadata = {\n"
      gemspec += "    \"source_code_uri\"        => \"https://github.com/adhithyan15/coding-adventures\",\n"
      gemspec += "    \"rubygems_mfa_required\"  => \"true\"\n"
      gemspec += "  }\n\n"
      direct_deps.each do |dep|
        dep_snake = to_snake_case(dep)
        gemspec += "  spec.add_dependency \"coding_adventures_#{dep_snake}\", \"~> 0.1\"\n"
      end
      gemspec += "  spec.add_development_dependency \"minitest\", \"~> 5.0\"\n"
      gemspec += "  spec.add_development_dependency \"rake\", \"~> 13.0\"\n"
      gemspec += "end\n"

      gemfile = "# frozen_string_literal: true\n\nsource \"https://rubygems.org\"\ngemspec\n"
      if all_deps.any?
        gemfile += "\n# All transitive path dependencies.\n"
        all_deps.each do |dep|
          dep_snake = to_snake_case(dep)
          gemfile += "gem \"coding_adventures_#{dep_snake}\", path: \"../#{dep_snake}\"\n"
        end
      end

      rakefile = <<~RUBY
        # frozen_string_literal: true

        require "rake/testtask"

        Rake::TestTask.new(:test) do |t|
          t.libs << "test"
          t.libs << "lib"
          t.test_files = FileList["test/**/test_*.rb"]
        end

        task default: :test
      RUBY

      entry = "# frozen_string_literal: true\n\n"
      if direct_deps.any?
        entry += "# IMPORTANT: Require dependencies FIRST, before own modules.\n"
        direct_deps.each do |dep|
          dep_snake = to_snake_case(dep)
          entry += "require \"coding_adventures_#{dep_snake}\"\n"
        end
        entry += "\n"
      end
      entry += "require_relative \"coding_adventures/#{snake}/version\"\n\n"
      entry += "module CodingAdventures\n  # #{description}\n  module #{camel}\n  end\nend\n"

      version_rb = <<~RUBY
        # frozen_string_literal: true

        module CodingAdventures
          module #{camel}
            VERSION = "0.1.0"
          end
        end
      RUBY

      test_rb = <<~RUBY
        # frozen_string_literal: true

        require "minitest/autorun"
        require "coding_adventures_#{snake}"

        class Test#{camel} < Minitest::Test
          def test_version_exists
            refute_nil CodingAdventures::#{camel}::VERSION
          end
        end
      RUBY

      write_file(File.join(target_dir, "coding_adventures_#{snake}.gemspec"), gemspec)
      write_file(File.join(target_dir, "Gemfile"), gemfile)
      write_file(File.join(target_dir, "Rakefile"), rakefile)
      write_file(File.join(target_dir, "lib", "coding_adventures_#{snake}.rb"), entry)
      write_file(File.join(target_dir, "lib", "coding_adventures", snake, "version.rb"), version_rb)
      write_file(File.join(target_dir, "test", "test_#{snake}.rb"), test_rb)
      write_file(File.join(target_dir, "BUILD"), "bundle install --quiet\nbundle exec rake test\n")
    end

    # =====================================================================
    # TypeScript File Generation
    # =====================================================================

    # Generate all files for a TypeScript package.
    #
    # TypeScript packages use:
    #   - package.json with "main": "src/index.ts" (NOT dist/index.js!)
    #   - tsconfig.json for TypeScript configuration
    #   - vitest.config.ts for test configuration
    #   - src/index.ts for the package entry point
    #   - tests/<name>.test.ts for vitest tests
    #   - BUILD file that chain-installs transitive deps
    #
    # CRITICAL: "main" MUST point to "src/index.ts" so Vitest can resolve
    # file: dependencies without a compile step. See lessons.md.
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    # @param ordered_deps [Array<String>] all deps in install order
    def self.generate_typescript(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      package_json = "{\n"
      package_json += "  \"name\": \"@coding-adventures/#{pkg_name}\",\n"
      package_json += "  \"version\": \"0.1.0\",\n"
      package_json += "  \"description\": \"#{description}\",\n"
      package_json += "  \"type\": \"module\",\n"
      package_json += "  \"main\": \"src/index.ts\",\n"
      package_json += "  \"scripts\": {\n"
      package_json += "    \"build\": \"tsc\",\n"
      package_json += "    \"test\": \"vitest run\",\n"
      package_json += "    \"test:coverage\": \"vitest run --coverage\"\n"
      package_json += "  },\n"
      package_json += "  \"author\": \"Adhithya Rajasekaran\",\n"
      package_json += "  \"license\": \"MIT\",\n"
      package_json += "  \"dependencies\": {\n"
      if direct_deps.any?
        entries = direct_deps.map { |dep| "    \"@coding-adventures/#{dep}\": \"file:../#{dep}\"" }
        package_json += entries.join(",\n") + "\n"
      end
      package_json += "  },\n"
      package_json += "  \"devDependencies\": {\n"
      package_json += "    \"typescript\": \"^5.0.0\",\n"
      package_json += "    \"vitest\": \"^3.0.0\",\n"
      package_json += "    \"@vitest/coverage-v8\": \"^3.0.0\"\n"
      package_json += "  }\n"
      package_json += "}\n"

      tsconfig = <<~JSON
        {
          "compilerOptions": {
            "target": "ES2022",
            "module": "ESNext",
            "moduleResolution": "bundler",
            "strict": true,
            "esModuleInterop": true,
            "skipLibCheck": true,
            "outDir": "dist",
            "rootDir": "src",
            "declaration": true
          },
          "include": ["src"]
        }
      JSON

      vitest_config = <<~TS
        import { defineConfig } from "vitest/config";

        export default defineConfig({
          test: {
            coverage: {
              provider: "v8",
              thresholds: {
                lines: 80,
              },
            },
          },
        });
      TS

      index_ts = <<~TS
        /**
         * @coding-adventures/#{pkg_name}
         *
         * #{description}
         *
         * This package is part of the coding-adventures monorepo.
         * #{layer_ctx}
         */

        export const VERSION = "0.1.0";
      TS

      test_ts = <<~TS
        import { describe, it, expect } from "vitest";
        import { VERSION } from "../src/index.js";

        describe("#{pkg_name}", () => {
          it("has a version", () => {
            expect(VERSION).toBe("0.1.0");
          });
        });
      TS

      build = "npm ci --quiet\nnpx vitest run --coverage\n"

      write_file(File.join(target_dir, "package.json"), package_json)
      write_file(File.join(target_dir, "tsconfig.json"), tsconfig)
      write_file(File.join(target_dir, "vitest.config.ts"), vitest_config)
      write_file(File.join(target_dir, "src", "index.ts"), index_ts)
      write_file(File.join(target_dir, "tests", "#{pkg_name}.test.ts"), test_ts)
      write_file(File.join(target_dir, "BUILD"), build)
    end

    # =====================================================================
    # Rust File Generation
    # =====================================================================

    # Generate all files for a Rust crate.
    #
    # Rust crates use:
    #   - Cargo.toml for crate metadata and dependencies
    #   - src/lib.rs for the crate entry point
    #   - BUILD file that runs `cargo test -p <name>`
    #
    # IMPORTANT: After creating a Rust crate, the workspace Cargo.toml must
    # be updated to include the new crate in its members list. This tool
    # handles that automatically.
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    def self.generate_rust(target_dir, pkg_name, description, layer_ctx, direct_deps)
      cargo = <<~TOML
        [package]
        name = "#{pkg_name}"
        version = "0.1.0"
        edition = "2021"
        description = "#{description}"

        [dependencies]
      TOML
      direct_deps.each do |dep|
        cargo += "#{dep} = { path = \"../#{dep}\" }\n"
      end

      lib_rs = <<~RS
        //! # #{pkg_name}
        //!
        //! #{description}
        //!
        //! This crate is part of the coding-adventures monorepo.
        //! #{layer_ctx}

        #[cfg(test)]
        mod tests {
            #[test]
            fn it_loads() {
                assert!(true, "#{pkg_name} crate loaded successfully");
            }
        }
      RS

      write_file(File.join(target_dir, "Cargo.toml"), cargo)
      write_file(File.join(target_dir, "src", "lib.rs"), lib_rs)
      write_file(File.join(target_dir, "BUILD"), "cargo test -p #{pkg_name} -- --nocapture\n")
    end

    # =====================================================================
    # Elixir File Generation
    # =====================================================================

    # Generate all files for an Elixir project.
    #
    # Elixir projects use:
    #   - mix.exs for project definition and dependencies
    #   - lib/coding_adventures/<snake>.ex for the main module
    #   - test/<snake>_test.exs for ExUnit tests
    #   - test/test_helper.exs to start ExUnit
    #   - BUILD file that chain-installs transitive deps
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param layer_ctx [String] layer context string for docs
    # @param direct_deps [Array<String>] direct dependency names
    # @param ordered_deps [Array<String>] all deps in install order
    def self.generate_elixir(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      snake = to_snake_case(pkg_name)
      camel = to_camel_case(pkg_name)

      deps_str = ""
      direct_deps.each_with_index do |dep, i|
        dep_snake = to_snake_case(dep)
        comma = i < direct_deps.size - 1 ? "," : ""
        deps_str += "      {:coding_adventures_#{dep_snake}, path: \"../#{dep_snake}\"}#{comma}\n"
      end

      mix_exs = <<~EX
        defmodule CodingAdventures.#{camel}.MixProject do
          use Mix.Project

          def project do
            [
              app: :coding_adventures_#{snake},
              version: "0.1.0",
              elixir: "~> 1.14",
              start_permanent: Mix.env() == :prod,
              deps: deps(),
              test_coverage: [
                summary: [threshold: 80]
              ]
            ]
          end

          def application do
            [
              extra_applications: [:logger]
            ]
          end

          defp deps do
            [
        #{deps_str}    ]
          end
        end
      EX

      lib_ex = <<~EX
        defmodule CodingAdventures.#{camel} do
          @moduledoc \"""
          #{description}

          This module is part of the coding-adventures monorepo.
          #{layer_ctx}
          \"""
        end
      EX

      test_exs = <<~EX
        defmodule CodingAdventures.#{camel}Test do
          use ExUnit.Case

          test "module loads" do
            assert Code.ensure_loaded?(CodingAdventures.#{camel})
          end
        end
      EX

      if ordered_deps.any?
        parts = ordered_deps.map { |dep| "cd ../#{to_snake_case(dep)} && mix deps.get --quiet && mix compile --quiet" }
        parts << "cd ../#{snake} && mix deps.get --quiet && mix test --cover"
        build = parts.join(" && \\\n") + "\n"
      else
        build = "mix deps.get --quiet && mix test --cover\n"
      end

      write_file(File.join(target_dir, "mix.exs"), mix_exs)
      write_file(File.join(target_dir, "lib", "coding_adventures", "#{snake}.ex"), lib_ex)
      write_file(File.join(target_dir, "test", "#{snake}_test.exs"), test_exs)
      write_file(File.join(target_dir, "test", "test_helper.exs"), "ExUnit.start()\n")
      write_file(File.join(target_dir, "BUILD"), build)
    end

    # =====================================================================
    # Common Files (README.md, CHANGELOG.md)
    # =====================================================================

    # Generate README.md and CHANGELOG.md for any language.
    #
    # @param target_dir [String] path where files will be written
    # @param pkg_name [String] kebab-case package name
    # @param description [String] one-line description
    # @param lang [String] one of the VALID_LANGUAGES
    # @param layer [Integer] layer number (0 for no layer context)
    # @param direct_deps [Array<String>] direct dependency names
    def self.generate_common_files(target_dir, pkg_name, description, lang, layer, direct_deps)
      today = Date.today.iso8601

      changelog = <<~MD
        # Changelog

        All notable changes to this package will be documented in this file.

        ## [0.1.0] - #{today}

        ### Added

        - Initial package scaffolding generated by scaffold-generator
      MD

      readme = "# #{pkg_name}\n\n#{description}\n"
      if layer.positive?
        readme += "\n## Layer #{layer}\n\nThis package is part of Layer #{layer} of the coding-adventures computing stack.\n"
      end
      if direct_deps.any?
        readme += "\n## Dependencies\n\n"
        direct_deps.each { |dep| readme += "- #{dep}\n" }
      end
      readme += "\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n"

      write_file(File.join(target_dir, "README.md"), readme)
      write_file(File.join(target_dir, "CHANGELOG.md"), changelog)
    end

    # =====================================================================
    # Rust Workspace Update
    # =====================================================================

    # Add a crate to the workspace Cargo.toml members list.
    #
    # @param repo_root [String] path to the repository root
    # @param pkg_name [String] kebab-case crate name
    # @return [Boolean] true on success
    def self.update_rust_workspace(repo_root, pkg_name)
      workspace_path = File.join(repo_root, "code", "packages", "rust", "Cargo.toml")
      return false unless File.exist?(workspace_path)

      content = File.read(workspace_path)
      return true if content.include?("\"#{pkg_name}\"")

      idx = content.index("members = [")
      return false unless idx

      close_idx = content.index("]", idx)
      new_entry = "  \"#{pkg_name}\",\n"
      content = content[0...close_idx] + new_entry + content[close_idx..]
      File.write(workspace_path, content)
      true
    end

    # =====================================================================
    # Repository Root Detection
    # =====================================================================

    # Walk up from cwd to find the git root directory.
    #
    # @return [String] path to the repository root
    # @raise [RuntimeError] if not inside a git repository
    def self.find_repo_root
      d = Dir.pwd
      loop do
        return d if Dir.exist?(File.join(d, ".git"))

        parent = File.dirname(d)
        raise "not inside a git repository" if parent == d

        d = parent
      end
    end

    # =====================================================================
    # Single-Language Scaffold
    # =====================================================================

    # Scaffold a package for a single language.
    #
    # This is the core orchestration method. It:
    #   1. Determines the target directory
    #   2. Validates that the target doesn't exist and deps are present
    #   3. Computes transitive closure and topological sort
    #   4. Calls the language-specific generator
    #   5. Generates common files (README, CHANGELOG)
    #
    # @param pkg_name [String] kebab-case package name
    # @param pkg_type [String] "library" or "program"
    # @param lang [String] one of the VALID_LANGUAGES
    # @param direct_deps [Array<String>] direct dependency names
    # @param layer [Integer] layer number
    # @param description [String] one-line description
    # @param dry_run [Boolean] if true, print plan but don't write
    # @param repo_root [String] path to the repository root
    # @param output [IO] output stream (default: $stdout)
    # @param err_output [IO] error stream (default: $stderr)
    def self.scaffold_one(pkg_name, pkg_type, lang, direct_deps, layer, description,
                          dry_run, repo_root, output: $stdout, err_output: $stderr)
      base_category = pkg_type == "library" ? "packages" : "programs"
      base_dir = File.join(repo_root, "code", base_category, lang)
      d_name = dir_name(pkg_name, lang)
      target_dir = File.join(base_dir, d_name)

      if Dir.exist?(target_dir)
        raise "directory already exists: #{target_dir}"
      end

      direct_deps.each do |dep|
        dep_dir = File.join(base_dir, dir_name(dep, lang))
        unless Dir.exist?(dep_dir)
          raise "dependency #{dep.inspect} not found for #{lang} at #{dep_dir}"
        end
      end

      all_deps = transitive_closure(direct_deps, lang, base_dir)
      ordered_deps = topological_sort(all_deps, lang, base_dir)

      layer_ctx = layer.positive? ? "Layer #{layer} in the computing stack." : ""

      if dry_run
        output.puts "[dry-run] Would create #{lang} package at: #{target_dir}"
        output.puts "  Direct deps: #{direct_deps}"
        output.puts "  All transitive deps: #{all_deps}"
        output.puts "  Install order: #{ordered_deps}"
        return
      end

      FileUtils.mkdir_p(target_dir)

      case lang
      when "python"
        generate_python(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      when "go"
        generate_go(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)
      when "ruby"
        generate_ruby(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)
      when "typescript"
        generate_typescript(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      when "rust"
        generate_rust(target_dir, pkg_name, description, layer_ctx, direct_deps)
      when "elixir"
        generate_elixir(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
      end

      generate_common_files(target_dir, pkg_name, description, lang, layer, direct_deps)

      output.puts "Created #{lang} package at: #{target_dir}"

      case lang
      when "rust"
        if update_rust_workspace(repo_root, pkg_name)
          output.puts "  Updated code/packages/rust/Cargo.toml workspace members"
        else
          err_output.puts "  WARNING: Manually add \"#{pkg_name}\" to code/packages/rust/Cargo.toml members"
        end
        output.puts "  Run: cargo build --workspace (to verify)"
      when "typescript"
        output.puts "  Run: cd #{target_dir} && npm install (to generate package-lock.json)"
      when "go"
        output.puts "  Run: cd #{target_dir} && go mod tidy"
      end
    end

    # =====================================================================
    # Main Entry Point
    # =====================================================================

    # Parse CLI arguments via CLI Builder and scaffold packages.
    #
    # This is the main entry point for the scaffold-generator program.
    # The entire CLI interface is defined in scaffold-generator.json;
    # this method just orchestrates the business logic.
    def self.main
      spec_file = File.join(File.dirname(__FILE__), "..", "..", "..", "scaffold-generator.json")

      begin
        result = CodingAdventures::CliBuilder::Parser.new(spec_file, ["scaffold-generator"] + ARGV).parse
      rescue CodingAdventures::CliBuilder::ParseErrors => e
        e.errors.each { |err| warn "scaffold-generator: #{err.message}" }
        exit 1
      end

      case result
      when CodingAdventures::CliBuilder::HelpResult
        puts result.text
        exit 0
      when CodingAdventures::CliBuilder::VersionResult
        puts result.version
        exit 0
      when CodingAdventures::CliBuilder::ParseResult
        pkg_name = result.arguments.fetch("package-name", "")
        pkg_type = result.flags.fetch("type", "library") || "library"
        lang_str = result.flags.fetch("language", "all") || "all"
        deps_str = result.flags.fetch("depends-on", "") || ""
        layer_val = result.flags.fetch("layer", 0) || 0
        description = result.flags.fetch("description", "") || ""
        dry_run = result.flags.fetch("dry-run", false) || false

        unless KEBAB_RE.match?(pkg_name)
          warn "scaffold-generator: invalid package name #{pkg_name.inspect} (must be kebab-case)"
          exit 1
        end

        if lang_str == "all"
          languages = VALID_LANGUAGES.dup
        else
          languages = []
          lang_str.split(",").each do |lang|
            lang = lang.strip
            unless VALID_LANGUAGES.include?(lang)
              warn "scaffold-generator: unknown language #{lang.inspect}"
              exit 1
            end
            languages << lang
          end
        end

        direct_deps = deps_str.empty? ? [] : deps_str.split(",").map(&:strip).reject(&:empty?)
        direct_deps.each do |dep|
          unless KEBAB_RE.match?(dep)
            warn "scaffold-generator: invalid dependency name #{dep.inspect}"
            exit 1
          end
        end

        repo_root = find_repo_root
        layer = layer_val.to_i

        had_error = false
        languages.each do |lang|
          scaffold_one(pkg_name, pkg_type, lang, direct_deps, layer, description, dry_run, repo_root)
        rescue RuntimeError => e
          warn "scaffold-generator [#{lang}]: #{e.message}"
          had_error = true
        end

        exit 1 if had_error
      end
    end
  end
end
