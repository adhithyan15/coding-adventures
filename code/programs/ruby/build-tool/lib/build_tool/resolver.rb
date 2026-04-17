# frozen_string_literal: true

# resolver.rb -- Dependency Resolution from Package Metadata
# ==========================================================
#
# This module reads package metadata files (pyproject.toml for Python, .gemspec
# for Ruby, go.mod for Go, package.json for TypeScript, Cargo.toml for Rust,
# Package.swift for Swift) and extracts internal dependencies. It builds a
# directed graph where edges represent "A depends on B".
#
# Dependency mapping conventions
# ------------------------------
#
# Each language ecosystem uses a different naming convention for packages in
# this monorepo:
#
# - **Python**: Package names in pyproject.toml use the `coding-adventures-`
#   prefix with hyphens. For example, `coding-adventures-logic-gates` maps to
#   the package `python/logic-gates`.
#
# - **Ruby**: Gem names in .gemspec use the `coding_adventures_` prefix with
#   underscores. For example, `coding_adventures_logic_gates` maps to
#   `ruby/logic_gates`.
#
# - **Go**: Module paths in go.mod include the repo path. We map module paths
#   to `go/X` based on the last path component.
#
# - **TypeScript**: package.json uses `@coding-adventures/` scoped npm names.
#   `@coding-adventures/logic-gates` maps to `typescript/logic-gates`.
#
# - **Rust**: Cargo.toml uses path-based local dependencies.
#   The crate name (key before `=`) maps to `rust/crate-name`.
#
# - **Swift**: Package.swift uses `.package(path: "../dep-name")` relative
#   path references. The directory name maps to `swift/dep-name`.
#
# External dependencies (those not matching the monorepo prefix) are silently
# skipped.

module BuildTool
  # --------------------------------------------------------------------------
  # DirectedGraph -- A minimal directed graph for dependency resolution.
  #
  # This is a self-contained graph implementation, so the build tool has zero
  # gem dependencies beyond the Ruby standard library. We store adjacency
  # lists in both directions (forward for successors, reverse for
  # predecessors) so that traversals in either direction are O(1) per edge.
  #
  # The key operations are:
  #   - add_node / add_edge   -- Build the graph incrementally.
  #   - independent_groups    -- Kahn's algorithm for topological levels.
  #   - transitive_closure    -- All reachable nodes from a starting node.
  #   - transitive_dependents -- All nodes that transitively depend on a node.
  # --------------------------------------------------------------------------
  class DirectedGraph
    def initialize
      # @forward maps each node to the set of nodes it has edges TO.
      # @reverse maps each node to the set of nodes that have edges TO it.
      @forward = Hash.new { |h, k| h[k] = Set.new }
      @reverse = Hash.new { |h, k| h[k] = Set.new }
    end

    # add_node -- Ensure a node exists in the graph.
    #
    # If the node already exists, this is a no-op. We touch both the forward
    # and reverse adjacency hashes so that the node appears in `nodes` even
    # if it has no edges.
    #
    # @param node [String] The node identifier.
    def add_node(node)
      @forward[node] # triggers default block, creating the Set
      @reverse[node]
    end

    # add_edge -- Add a directed edge from `from_node` to `to_node`.
    #
    # Both nodes are implicitly added if they don't exist yet.
    #
    # @param from_node [String] The source node.
    # @param to_node [String] The target node.
    def add_edge(from_node, to_node)
      add_node(from_node)
      add_node(to_node)
      @forward[from_node].add(to_node)
      @reverse[to_node].add(from_node)
    end

    # has_node? -- Check whether a node exists in the graph.
    #
    # @param node [String] The node to check.
    # @return [Boolean]
    def has_node?(node)
      @forward.key?(node)
    end

    # nodes -- Return all node identifiers.
    #
    # @return [Array<String>]
    def nodes
      @forward.keys
    end

    # successors -- Return nodes that this node has edges TO.
    #
    # @param node [String]
    # @return [Array<String>]
    def successors(node)
      @forward.fetch(node, Set.new).to_a
    end

    # predecessors -- Return nodes that have edges TO this node.
    #
    # @param node [String]
    # @return [Array<String>]
    def predecessors(node)
      @reverse.fetch(node, Set.new).to_a
    end

    # transitive_closure -- All nodes reachable from `node` (excluding itself).
    #
    # We do a simple iterative depth-first traversal following forward edges.
    # The result does NOT include the starting node itself, matching the
    # Python implementation's behavior.
    #
    # @param node [String]
    # @return [Set<String>]
    def transitive_closure(node)
      return Set.new unless @forward.key?(node)

      visited = Set.new
      stack = @forward[node].to_a
      visited.merge(stack)

      until stack.empty?
        current = stack.pop
        @forward.fetch(current, Set.new).each do |successor|
          unless visited.include?(successor)
            visited.add(successor)
            stack.push(successor)
          end
        end
      end

      visited
    end

    # transitive_dependents -- All nodes that transitively depend on `node`.
    #
    # This walks REVERSE edges: it finds every node that (directly or
    # indirectly) has `node` as a dependency. Used to propagate build
    # failures upward through the dependency chain.
    #
    # @param node [String]
    # @return [Set<String>]
    def transitive_dependents(node)
      return Set.new unless @reverse.key?(node)

      visited = Set.new
      stack = @reverse[node].to_a
      visited.merge(stack)

      until stack.empty?
        current = stack.pop
        @reverse.fetch(current, Set.new).each do |predecessor|
          unless visited.include?(predecessor)
            visited.add(predecessor)
            stack.push(predecessor)
          end
        end
      end

      visited
    end

    # independent_groups -- Topological sort via Kahn's algorithm.
    #
    # Returns an array of arrays (levels). Each level contains nodes whose
    # in-degree is zero after removing all nodes in previous levels. Nodes
    # within a level can be built in parallel because they have no
    # dependencies on each other.
    #
    # Raises RuntimeError if the graph contains a cycle (not all nodes
    # can be processed).
    #
    # @return [Array<Array<String>>]
    def independent_groups
      in_degree = {}
      @reverse.each { |node, preds| in_degree[node] = preds.size }

      # Start with all zero-in-degree nodes, sorted for determinism.
      current_level = in_degree.select { |_node, deg| deg.zero? }.keys.sort
      groups = []
      processed = 0

      until current_level.empty?
        groups << current_level
        processed += current_level.size

        next_level_set = Set.new
        current_level.each do |node|
          @forward[node].each do |successor|
            in_degree[successor] -= 1
            next_level_set.add(successor) if in_degree[successor].zero?
          end
        end

        current_level = next_level_set.to_a.sort
      end

      if processed != @forward.size
        raise "Dependency graph contains a cycle"
      end

      groups
    end
  end

  # --------------------------------------------------------------------------
  # Dependency parsing -- one method per language ecosystem.
  #
  # Each parser reads the ecosystem's metadata file and extracts references to
  # other monorepo packages. External dependencies (not matching the monorepo
  # naming convention) are silently ignored.
  # --------------------------------------------------------------------------
  module Resolver
    module_function

    # parse_python_deps -- Extract internal deps from pyproject.toml.
    #
    # We use a dead-simple TOML parser (just regex) because we only need the
    # `dependencies = [...]` array from `[project]`. A full TOML parser would
    # be overkill and would add a gem dependency we don't want.
    #
    # The format we look for is:
    #   dependencies = ["coding-adventures-pkg-a>=0.1", "coding-adventures-pkg-b"]
    #
    # @param package [Package] The Python package.
    # @param known_names [Hash<String, String>] Mapping from pypi name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_python_deps(package, known_names)
      pyproject = package.path / "pyproject.toml"
      return [] unless pyproject.exist?

      text = pyproject.read

      # Extract the dependencies array. We look for the pattern:
      #   dependencies = ["...", "..."]
      # This is a simplified parser that handles the single-line case.
      match = text.match(/dependencies\s*=\s*\[([^\]]*)\]/)
      return [] unless match

      deps_str = match[1]
      internal_deps = []

      # Split on commas, strip quotes and version specifiers.
      deps_str.scan(/"([^"]*)"/).flatten.each do |dep_str|
        dep_name = dep_str.split(/[>=<!;\s]/).first&.strip&.downcase
        next unless dep_name

        if known_names.key?(dep_name)
          internal_deps << known_names[dep_name]
        end
      end

      internal_deps
    end

    # parse_ruby_deps -- Extract internal deps from .gemspec.
    #
    # We look for lines like:
    #   spec.add_dependency "coding_adventures_something"
    #
    # @param package [Package] The Ruby package.
    # @param known_names [Hash<String, String>] Mapping from gem name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_ruby_deps(package, known_names)
      gemspec_files = package.path.glob("*.gemspec").to_a
      return [] if gemspec_files.empty?

      text = gemspec_files.first.read
      internal_deps = []

      text.scan(/spec\.add_dependency\s+"([^"]+)"/).flatten.each do |gem_name|
        gem_name = gem_name.strip.downcase
        if known_names.key?(gem_name)
          internal_deps << known_names[gem_name]
        end
      end

      internal_deps
    end

    # parse_go_deps -- Extract internal deps from go.mod.
    #
    # We look for `require` lines (both single-line and block form) and match
    # module paths against the known names table.
    #
    # @param package [Package] The Go package.
    # @param known_names [Hash<String, String>] Mapping from module path to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_go_deps(package, known_names)
      go_mod = package.path / "go.mod"
      return [] unless go_mod.exist?

      text = go_mod.read
      internal_deps = []
      in_require_block = false

      text.lines.each do |line|
        stripped = line.strip

        if stripped == "require ("
          in_require_block = true
          next
        end
        if stripped == ")"
          in_require_block = false
          next
        end

        if in_require_block || stripped.start_with?("require ")
          # Extract module path (first token after "require").
          parts = stripped.sub("require ", "").strip.split
          if parts.any?
            module_path = parts.first.downcase
            if known_names.key?(module_path)
              internal_deps << known_names[module_path]
            end
          end
        end
      end

      internal_deps
    end

    # parse_elixir_deps -- Extract internal deps from mix.exs.
    #
    # @param package [Package] The Elixir package.
    # @param known_names [Hash<String, String>] Mapping from elixir app name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_elixir_deps(package, known_names)
      mix_exs = package.path / "mix.exs"
      return [] unless mix_exs.exist?

      internal_deps = []
      mix_exs.read.lines.each do |line|
        line.scan(/\{:(coding_adventures_\w+)/).flatten.each do |app_name|
          app_name = app_name.downcase
          if known_names.key?(app_name)
            internal_deps << known_names[app_name]
          end
        end
      end

      internal_deps
    end

    # parse_lua_deps -- Extract internal deps from .rockspec.
    #
    # LuaRocks rockspec files declare dependencies in a Lua table:
    #
    #   dependencies = {
    #       "lua >= 5.4",
    #       "coding-adventures-logic-gates >= 0.1.0",
    #   }
    #
    # We scan for quoted strings inside the dependencies block and map them
    # to internal package names, stripping version specifiers.
    #
    # @param package [Package] The Lua package.
    # @param known_names [Hash<String, String>] Mapping from rockspec name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_lua_deps(package, known_names)
      rockspec_files = package.path.glob("*.rockspec").to_a
      return [] if rockspec_files.empty?

      text = rockspec_files.first.read
      internal_deps = []
      in_deps = false

      text.lines.each do |line|
        stripped = line.strip

        unless in_deps
          if stripped.include?("dependencies") && stripped.include?("=") && stripped.include?("{")
            in_deps = true
            if stripped.include?("}")
              extract_lua_deps(stripped, known_names, internal_deps)
              break
            end
            extract_lua_deps(stripped, known_names, internal_deps)
          end
          next
        end

        # Inside the dependencies block.
        if stripped.include?("}")
          extract_lua_deps(stripped, known_names, internal_deps)
          break
        end
        extract_lua_deps(stripped, known_names, internal_deps)
      end

      internal_deps
    end

    # extract_lua_deps -- Extract dependency names from a line of a rockspec.
    #
    # @param line [String] A line from the rockspec file.
    # @param known_names [Hash<String, String>] Name mapping.
    # @param deps [Array<String>] Accumulator for found dependencies.
    def extract_lua_deps(line, known_names, deps)
      line.scan(/"([^"]+)"/).flatten.each do |dep_str|
        dep_name = dep_str.split(/[>=<!~\s]/).first&.strip&.downcase
        next unless dep_name

        deps << known_names[dep_name] if known_names.key?(dep_name)
      end
    end

    # parse_typescript_deps -- Extract internal deps from package.json.
    #
    # TypeScript packages declare dependencies in package.json:
    #
    #   "dependencies": {
    #       "@coding-adventures/logic-gates": "file:../logic-gates"
    #   }
    #
    # We scan both `dependencies` and `devDependencies` blocks for keys
    # matching `@coding-adventures/` prefix and map them to package names.
    #
    # @param package [Package] The TypeScript package.
    # @param known_names [Hash<String, String>] Mapping from npm name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_typescript_deps(package, known_names)
      package_json = package.path / "package.json"
      return [] unless package_json.exist?

      text = package_json.read
      internal_deps = []
      in_deps = false
      key_re = /"([^"]+)"\s*:/

      text.lines.each do |line|
        stripped = line.strip

        unless in_deps
          if (stripped.include?('"dependencies"') || stripped.include?('"devDependencies"')) &&
             stripped.include?("{")
            in_deps = true
          end
          next
        end

        if stripped.include?("}")
          in_deps = false
          next
        end

        stripped.scan(key_re).flatten.each do |dep_name|
          dep_name = dep_name.strip.downcase
          internal_deps << known_names[dep_name] if known_names.key?(dep_name)
        end
      end

      internal_deps
    end

    # parse_rust_deps -- Extract internal deps from Cargo.toml.
    #
    # Rust Cargo.toml declares workspace-local dependencies with path references:
    #
    #   [dependencies]
    #   logic-gates = { path = "../logic-gates" }
    #
    # We look for lines in the [dependencies] section that contain `path =`
    # and extract the crate name (the key before `=`).
    #
    # @param package [Package] The Rust package.
    # @param known_names [Hash<String, String>] Mapping from crate name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_rust_deps(package, known_names)
      cargo_toml = package.path / "Cargo.toml"
      return [] unless cargo_toml.exist?

      text = cargo_toml.read
      internal_deps = []
      in_deps = false

      text.lines.each do |line|
        stripped = line.strip

        if stripped.start_with?("[")
          in_deps = stripped == "[dependencies]"
          next
        end

        next unless in_deps

        # Look for lines like: logic-gates = { path = "../logic-gates" }
        if stripped.include?("path") && stripped.include?("=")
          parts = stripped.split("=", 2)
          if parts.size >= 2
            crate_name = parts.first.strip.downcase
            internal_deps << known_names[crate_name] if known_names.key?(crate_name)
          end
        end
      end

      internal_deps
    end

    # parse_dotnet_deps -- Extract internal deps from .csproj/.fsproj.
    #
    # .NET packages declare sibling package dependencies via ProjectReference:
    #
    #   <ProjectReference Include="../logic-gates/logic-gates.csproj" />
    #
    # We extract the sibling directory name after "../" and map it back to
    # the internal package name using the known names table.
    def parse_dotnet_deps(package, known_names)
      project_files = package.path.glob("*.csproj").to_a + package.path.glob("*.fsproj").to_a
      return [] if project_files.empty?

      internal_deps = []
      pattern = /<ProjectReference\s+Include\s*=\s*"\.\.[\\\/]([^\/\\"]+)[\\\/][^"]*"/

      project_files.each do |project_file|
        project_file.read.lines.each do |line|
          match = line.match(pattern)
          next unless match

          dep_dir = match[1].strip.downcase
          next if dep_dir.include?("/") || dep_dir.include?("\\") || dep_dir == ".."

          internal_deps << known_names[dep_dir] if known_names.key?(dep_dir)
        end
      end

      internal_deps
    end

    # SWIFT_DEP_RE -- Matches .package(path: "../dep-name") in Package.swift.
    SWIFT_DEP_RE = /\.package\s*\(\s*path\s*:\s*"\.\.\/(.*?)"/

    # parse_swift_deps -- Extract internal deps from Package.swift.
    #
    # Swift Package Manager uses relative path references for local (monorepo)
    # dependencies. The declaration always appears on a single line:
    #
    #   .package(path: "../logic-gates"),
    #
    # We scan for this pattern and map the directory name back to our internal
    # package name. External dependencies (declared with `url:`) are silently
    # skipped because they don't match the `path: "../"` prefix.
    #
    # @param package [Package] The Swift package.
    # @param known_names [Hash<String, String>] Mapping from dir name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_swift_deps(package, known_names)
      manifest = package.path / "Package.swift"
      return [] unless manifest.exist?

      internal_deps = []

      manifest.read.lines.each do |line|
        stripped = line.strip
        next if stripped.empty? || stripped.start_with?("//")

        match = SWIFT_DEP_RE.match(stripped)
        next unless match

        dep_dir = match[1].downcase
        # Guard against path traversal: reject segments with separators or "..".
        next if dep_dir.include?("/") || dep_dir.include?("\\") || dep_dir == ".."

        internal_deps << known_names[dep_dir] if known_names.key?(dep_dir)
      end

      internal_deps
    end

    # parse_perl_deps -- Extract internal dependencies from a Perl cpanfile.
    #
    # A cpanfile declares dependencies with one `requires` per line:
    #
    #   requires 'coding-adventures-logic-gates';
    #   requires 'coding-adventures-bitset', '>= 0.01';
    #
    # We scan for lines matching `requires 'coding-adventures-...'` and map
    # them to internal package names. External deps are silently skipped.
    #
    # @param package [Package]
    # @param known_names [Hash<String, String>]
    # @return [Array<String>]
    def parse_perl_deps(package, known_names)
      cpanfile = package.path / "cpanfile"
      return [] unless cpanfile.exist?

      internal_deps = []
      pattern = /requires\s+['"](coding-adventures-[^'"]+)['"]/

      cpanfile.read.lines.each do |line|
        stripped = line.strip
        next if stripped.empty? || stripped.start_with?("#")

        match = stripped.match(pattern)
        if match
          dep_name = match[1].downcase
          internal_deps << known_names[dep_name] if known_names.key?(dep_name)
        end
      end

      internal_deps
    end

    # parse_haskell_deps -- Extract internal dependencies from a Haskell package.
    #
    # @param package [Package]
    # @param known_names [Hash<String, String>]
    # @return [Array<String>]
    def parse_haskell_deps(package, known_names)
      cabal_files = package.path.glob("*.cabal").to_a
      return [] if cabal_files.empty?

      internal_deps = []
      pattern = /coding-adventures-([a-z0-9-]+)/

      cabal_files.first.read.lines.each do |line|
        line.scan(pattern).flatten.each do |dep_base|
          dep_name = "coding-adventures-#{dep_base.downcase}"
          next unless known_names.key?(dep_name)
          next if known_names[dep_name] == package.name
          internal_deps << known_names[dep_name]
        end
      end

      internal_deps
    end

    # parse_gradle_deps -- Extract internal dependencies from settings.gradle.kts.
    #
    # Both Java and Kotlin packages use Gradle as their build system. In this
    # monorepo, sibling package dependencies are declared as composite builds
    # in settings.gradle.kts:
    #
    #   includeBuild("../logic-gates")
    #   includeBuild("../transistors")
    #
    # We scan for `includeBuild("../...")` entries and map the directory name
    # back to our internal package name.
    #
    # @param package [Package]
    # @param known_names [Hash<String, String>]
    # @return [Array<String>]
    def parse_gradle_deps(package, known_names)
      settings_file = package.path / "settings.gradle.kts"
      return [] unless settings_file.exist?

      internal_deps = []
      pattern = /includeBuild\s*\(\s*"\.\.\/([^"]+)"\s*\)/

      settings_file.read.lines.each do |line|
        stripped = line.strip
        next if stripped.empty? || stripped.start_with?("//")

        match = stripped.match(pattern)
        if match
          dep_dir = match[1].downcase
          # Guard against path traversal.
          next if dep_dir.include?("/") || dep_dir.include?("\\") || dep_dir == ".."
          internal_deps << known_names[dep_dir] if known_names.key?(dep_dir)
        end
      end

      internal_deps
    end

    # build_known_names -- Build ecosystem-specific name -> package name mapping.
    #
    # For Python:     "coding-adventures-logic-gates" -> "python/logic-gates"
    # For Ruby:       "coding_adventures_logic_gates" -> "ruby/logic_gates"
    # For Go:         module paths -> "go/module-name"
    # For TypeScript: "@coding-adventures/logic-gates" -> "typescript/logic-gates"
    # For Rust:       "logic-gates" (crate name) -> "rust/logic-gates"
    # For Swift:      "logic-gates" (dir name) -> "swift/logic-gates"
    #
    # Library packages take priority over programs when the same ecosystem name
    # maps to both. This prevents a program that depends on its own library from
    # resolving the dep to itself and creating a self-loop.
    #
    # @param packages [Array<Package>]
    # @return [Hash<String, String>]
    def dependency_scope(language)
      return "dotnet" if %w[csharp fsharp dotnet].include?(language)
      return "wasm" if language == "wasm"

      language
    end

    def in_dependency_scope?(package_language, scope)
      case scope
      when "dotnet"
        %w[csharp fsharp dotnet].include?(package_language)
      when "wasm"
        %w[wasm rust].include?(package_language)
      else
        package_language == scope
      end
    end

    def read_cargo_package_name(pkg)
      cargo_toml = pkg.path / "Cargo.toml"
      return nil unless cargo_toml.exist?

      match = cargo_toml.read.match(/^\s*name\s*=\s*"([^"]+)"/)
      match && match[1].strip.downcase
    end

    def build_known_names(packages, language = nil)
      known = {}

      # set_known inserts key->value, letting library packages overwrite programs
      # but never letting programs overwrite library packages.
      set_known = lambda do |key, value, pkg_path|
        unless known.key?(key)
          known[key] = value
          next
        end
        # Allow overwrite only if the current pkg is a library (not a program).
        known[key] = value unless pkg_path.to_s.tr("\\", "/").include?("/programs/")
      end

      packages.each do |pkg|
        next if language && !in_dependency_scope?(pkg.language, language)

        case pkg.language
        when "python"
          pypi_name = "coding-adventures-#{pkg.path.basename}".downcase
          set_known.call(pypi_name, pkg.name, pkg.path)
        when "ruby"
          gem_name = "coding_adventures_#{pkg.path.basename}".downcase
          set_known.call(gem_name, pkg.name, pkg.path)
        when "go"
          # For Go, read the module path from go.mod. Module paths are unique
          # across packages and programs so no priority logic needed.
          go_mod = pkg.path / "go.mod"
          if go_mod.exist?
            go_mod.read.lines.each do |line|
              if line.start_with?("module ")
                module_path = line.split(nil, 2)[1]&.strip&.downcase
                known[module_path] = pkg.name if module_path
                break
              end
            end
          end
        when "typescript"
          # Convert dir name to npm scoped name: "logic-gates" -> "@coding-adventures/logic-gates"
          npm_name = "@coding-adventures/#{pkg.path.basename}".downcase
          set_known.call(npm_name, pkg.name, pkg.path)
          set_known.call(pkg.path.basename.to_s.downcase, pkg.name, pkg.path)

          # Also read the actual "name" field from package.json for accuracy.
          package_json = pkg.path / "package.json"
          if package_json.exist?
            match = package_json.read.match(/"name"\s*:\s*"([^"]+)"/)
            set_known.call(match[1].strip.downcase, pkg.name, pkg.path) if match
          end
        when "rust", "wasm"
          # Rust crate names use the directory name directly (kebab-case).
          crate_name = pkg.path.basename.to_s.downcase
          set_known.call(crate_name, pkg.name, pkg.path)
          cargo_name = read_cargo_package_name(pkg)
          set_known.call(cargo_name, pkg.name, pkg.path) if cargo_name
        when "elixir"
          # Elixir mix names replace hyphens with underscores.
          base_name = pkg.path.basename.to_s.gsub("-", "_").downcase
          app_name = "coding_adventures_#{base_name}"
          set_known.call(app_name, pkg.name, pkg.path)
          set_known.call(base_name, pkg.name, pkg.path)

          # Also read the actual app name from mix.exs for accuracy.
          mix_exs = pkg.path / "mix.exs"
          if mix_exs.exist?
            match = mix_exs.read.match(/app:\s*:([a-z0-9_]+)/)
            set_known.call(match[1].strip.downcase, pkg.name, pkg.path) if match
          end
        when "lua"
          # Lua rockspec names use hyphens: "logic_gates" dir -> "coding-adventures-logic-gates"
          rockspec_name = "coding-adventures-#{pkg.path.basename.to_s.gsub('_', '-')}".downcase
          set_known.call(rockspec_name, pkg.name, pkg.path)
        when "perl"
          # Perl CPAN dist names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
          cpan_name = "coding-adventures-#{pkg.path.basename}".downcase
          set_known.call(cpan_name, pkg.name, pkg.path)
        when "swift"
          # Swift SPM package names are the kebab-case directory name.
          dir_base = pkg.path.basename.to_s.downcase
          set_known.call(dir_base, pkg.name, pkg.path)
        when "haskell"
          # Haskell Cabal package names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
          cabal_name = "coding-adventures-#{pkg.path.basename}".downcase
          set_known.call(cabal_name, pkg.name, pkg.path)
        when "java", "kotlin", "csharp", "fsharp", "dotnet"
          # Java and Kotlin use Gradle composite builds. Dependencies are
          # referenced by directory name in settings.gradle.kts. .NET uses the
          # sibling directory name from ProjectReference paths.
          dir_base = pkg.path.basename.to_s.downcase
          set_known.call(dir_base, pkg.name, pkg.path)
        end
      end

      known
    end

    # resolve_dependencies -- Parse metadata, build a dependency graph.
    #
    # The graph contains all discovered packages as nodes. Edges represent
    # "A depends on B" -- specifically, we add edge(dep, pkg) meaning "dep
    # must be built before pkg". This way, Kahn's algorithm in
    # `independent_groups` produces levels in correct build order: packages
    # with zero in-degree (no dependencies) come first.
    #
    # @param packages [Array<Package>]
    # @return [DirectedGraph]
    def resolve_dependencies(packages)
      graph = DirectedGraph.new

      # Add all packages as nodes first.
      packages.each { |pkg| graph.add_node(pkg.name) }

      known_names_by_scope = {}
      packages.each do |pkg|
        scope = dependency_scope(pkg.language)
        known_names_by_scope[scope] ||= build_known_names(packages, scope)
      end

      # Parse dependencies for each package and add edges.
      packages.each do |pkg|
        known_names = known_names_by_scope.fetch(dependency_scope(pkg.language))
        deps = case pkg.language
               when "python"     then parse_python_deps(pkg, known_names)
               when "ruby"       then parse_ruby_deps(pkg, known_names)
               when "go"         then parse_go_deps(pkg, known_names)
               when "typescript" then parse_typescript_deps(pkg, known_names)
               when "rust", "wasm" then parse_rust_deps(pkg, known_names)
               when "elixir"     then parse_elixir_deps(pkg, known_names)
               when "lua"        then parse_lua_deps(pkg, known_names)
               when "perl"       then parse_perl_deps(pkg, known_names)
               when "swift"      then parse_swift_deps(pkg, known_names)
               when "haskell"    then parse_haskell_deps(pkg, known_names)
               when "java", "kotlin" then parse_gradle_deps(pkg, known_names)
               when "csharp", "fsharp", "dotnet" then parse_dotnet_deps(pkg, known_names)
               else []
               end

        deps.each do |dep_name|
          # Edge direction: dep -> pkg means "dep must be built before pkg".
          graph.add_edge(dep_name, pkg.name)
        end
      end

      graph
    end
  end
end
