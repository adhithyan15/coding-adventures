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
# - **Rust**: Cargo.toml uses path-based local dependencies in the tables
#   that link into or build the crate (`[dependencies]`,
#   `[build-dependencies]` and their `[target.<cfg>.…]` forms). The crate name
#   (the entry key, or its `package` rename) maps to `rust/crate-name`.
#
# - **Swift**: Package.swift uses `.package(path: "../dep-name")` relative
#   path references. The directory name maps to `swift/dep-name`.
#
# External dependencies (those not matching the monorepo prefix) are silently
# skipped.

module BuildTool
  # MetadataEncodingError -- Stable failure for malformed package metadata.
  #
  # The error carries only portable package and repository-relative manifest
  # identities. CLI callers can therefore report malformed UTF-8 without
  # exposing a host-specific checkout root.
  class MetadataEncodingError < StandardError
    attr_reader :code, :package, :manifest, :encoding

    def initialize(package:, manifest:)
      @code = "METADATA_INVALID_UTF8"
      @package = package
      @manifest = manifest
      @encoding = "UTF-8"
      super("#{code}: package=#{package} manifest=#{manifest} encoding=#{encoding}")
    end
  end

  # DependencySelfEdgeError -- A package whose manifest resolves itself.
  #
  # The conformance contract rejects this as DEPENDENCY_SELF_EDGE: a self-edge
  # is neither dropped nor drawn (a drawn self-loop would surface later as a
  # cycle, far from its cause). A Cargo manifest can do it with nothing more
  # than a path entry named after its own directory or [package] name. The
  # manifest is repository-relative, so the message never discloses the
  # checkout root.
  class DependencySelfEdgeError < StandardError
    attr_reader :code, :package, :manifest, :dependency

    def initialize(package:, manifest:, dependency:)
      @code = "DEPENDENCY_SELF_EDGE"
      @package = package
      @manifest = manifest
      @dependency = dependency
      super("#{code}: package=#{package} manifest=#{manifest} dependency=#{dependency}")
    end
  end

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

    # each_predecessor -- Lazily enumerate nodes with edges TO `node`.
    #
    # Callers that enforce their own work ceiling must not materialize the
    # complete predecessor set before that ceiling can fire. Returning an
    # Enumerator when no block is supplied also preserves normal Enumerable
    # composition without copying the adjacency set.
    #
    # @param node [String]
    # @yieldparam predecessor [String]
    # @return [Enumerator, nil]
    def each_predecessor(node)
      return enum_for(__method__, node) unless block_given?
      return unless @reverse.key?(node)

      @reverse.fetch(node).each { |predecessor| yield predecessor }
      nil
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

      text = read_utf8_metadata(package, rockspec_files.first)
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

    # read_utf8_metadata -- Read raw metadata bytes and decode strict UTF-8.
    #
    # Pathname#read inherits Ruby's process-default external encoding. Package
    # manifests are portable repository data, so their decoding must not vary
    # with a runner locale or silently replace malformed byte sequences.
    def read_utf8_metadata(package, path)
      text = File.binread(path).force_encoding(Encoding::UTF_8)
      return text if text.valid_encoding?

      raise MetadataEncodingError.new(
        package: package.name,
        manifest: repository_relative_manifest(path)
      )
    end

    def repository_relative_manifest(path)
      normalized = Pathname(path).cleanpath.to_s.tr("\\", "/")
      # The greedy prefix makes this the LAST code/packages|programs pair, so a
      # checkout under a directory that itself holds one is never printed.
      match = normalized.match(%r{\A(?:.*/)?(code/(?:packages|programs)/.+)\z})
      return match[1] if match

      Pathname(normalized).basename.to_s
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
    # Which Cargo tables are build-graph edges
    # ----------------------------------------
    #
    # A graph edge means "rebuild and re-test the dependent when this
    # changes", so it must cover everything Cargo compiles INTO the crate:
    #
    #   table                                 edge?  why
    #   ------------------------------------  -----  ---------------------------
    #   [dependencies]                        yes    linked into the library
    #   [build-dependencies]                  yes    compiled into build.rs
    #   [build_dependencies]                  yes    legacy spelling of the above
    #   [target.<cfg>.<one of the above>]     yes    union over every cfg: CI
    #                                                builds on several hosts
    #   [dev-dependencies] (every form)       no     tests only; Cargo allows a
    #                                                dev-dependency cycle
    #   [patch], [replace], [workspace], ...  no     not a dependency of the crate
    #
    # Both TOML spellings of a path dependency are read:
    #
    #   [dependencies]
    #   logic-gates = { path = "../logic-gates" }          <- inline table
    #   gates = { package = "logic-gates", path = "..." }  <- renamed
    #
    #   [dependencies.logic-gates]                         <- sub-table
    #   path = "../logic-gates"
    #
    # The line procedure is the Rust paragraph of
    # code/specs/build-tool-conformance.md; every build-tool front implements
    # it and the shared fixture resolution-rust-field-aware pins it. Each
    # candidate name is looked up among the known Cargo names (directory name
    # and declared [package] name); duplicates collapse to one dependency.
    #
    # @param package [Package] The Rust package.
    # @param known_names [Hash<String, String>] Mapping from crate name to package name.
    # @return [Array<String>] Internal dependency package names.
    def parse_rust_deps(package, known_names)
      cargo_toml = package.path / "Cargo.toml"
      return [] unless cargo_toml.exist?

      cargo_link_dependency_names(File.binread(cargo_toml)).filter_map do |crate_name|
        known_names[crate_name]
      end.uniq
    end

    # The table names whose path entries link into, or build, the crate.
    # Compared exactly: TOML keys are case-sensitive, so `[Dependencies]` is
    # some other table.
    CARGO_LINK_TABLE_NAMES = %w[dependencies build-dependencies build_dependencies].freeze

    # TOML's bare-key alphabet: ASCII letters, digits, `_` and `-`.
    TOML_BARE_KEY = /\A[A-Za-z0-9_-]+\z/

    # The UTF-8 byte-order mark, as raw bytes (the manifest is read binary).
    UTF8_BOM = "\xEF\xBB\xBF".b.freeze

    # cargo_link_dependency_names -- Every path dependency's crate name.
    #
    # Returns lower-case names in manifest order, duplicates kept (the
    # caller collapses them). The text is scanned as raw bytes: every
    # character the procedure compares is ASCII, so a byte scan is exact and
    # does not depend on Ruby's default external encoding.
    #
    # It is one pass over the lines with a three-state machine:
    #
    #   :ignored  not a link table; its lines are skipped
    #   :inline   [D] or [target.C.D]: one entry per line
    #   :sub      [D.N] or [target.C.D.N]: the `path`/`package` keys of entry N
    #
    # Every header line -- even a malformed one, or an array of tables --
    # ends the current table, and so does the end of the file; a sub-table's
    # entry is complete only then, since its keys may come in either order.
    #
    # One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside
    # an inline table span lines, and the repository writes exactly that:
    #
    #   coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
    #       "crash-injection",
    #   ] }
    #
    # So while the value still has an unclosed `{` or `[`, the next lines
    # are joined onto it with one space (and are not looked at as headers or
    # entries). If the file ends first, the entry contributes nothing. The
    # brackets are counted over the joined value as one text -- a string left
    # open at the end of one line continues after the joining space -- but
    # the joined value is never rescanned: a TomlBracketScanner carries its
    # state from line to line and the pieces are joined once, so the join is
    # linear in the entry's length.
    def cargo_link_dependency_names(text)
      text = text.b
      text = text.byteslice(UTF8_BOM.bytesize..) if text.start_with?(UTF8_BOM)
      # Split at LF only, keeping trailing empty fields (-1).
      lines = text.split("\n", -1)
      clean = lambda do |raw|
        raw = raw.delete_suffix("\r")
        trim_spaces_and_tabs(strip_toml_line_comment(raw))
      end

      names = []
      kind = :ignored
      sub_key = nil
      sub_path = nil
      sub_package = nil
      finish_table = lambda do
        if kind == :sub && sub_path
          names << ascii_downcase(sub_package || sub_key)
        end
      end

      index = 0
      while index < lines.length
        line = clean.call(lines[index])
        index += 1

        # Step 2: headers. Every header ends the current table.
        if line.start_with?("[") && line.end_with?("]")
          finish_table.call
          kind, sub_key = classify_cargo_link_header(line)
          sub_path = sub_package = nil
          next
        end

        equals = index_toml_outside_strings(line, "=")
        next unless equals

        case kind
        when :inline
          # Step 3: inline entries, continued while a bracket is open.
          # The pieces are collected and joined once, and the scanner carries
          # its state from piece to piece, so the join is linear.
          value = trim_spaces_and_tabs(line.byteslice((equals + 1)..))
          if value.start_with?("{")
            scanner = TomlBracketScanner.new
            scanner.feed(value)
            if scanner.depth > 0
              pieces = [value]
              closed = false
              while !closed && index < lines.length
                piece = clean.call(lines[index])
                index += 1
                scanner.feed(" ")
                scanner.feed(piece)
                pieces << piece
                closed = scanner.depth <= 0
              end
              # If the file ended inside the value, there is no entry.
              value = closed ? pieces.join(" ") : nil
            end
          end
          crate_name = value && cargo_path_dependency_name(line.byteslice(0, equals), value)
          names << crate_name if crate_name
        when :sub
          # Step 4: only the keys `path` and `package` matter, bare or quoted
          # (`"path" = ...` is the same key); first wins. `version`,
          # `features`, `optional` ... are ignored, and so is `Path`: TOML
          # keys are case-sensitive.
          field_value = toml_single_string(line.byteslice((equals + 1)..))
          next unless field_value

          case toml_key_segment(line.byteslice(0, equals))
          when "path" then sub_path ||= field_value
          when "package" then sub_package ||= field_value
          end
        end
      end
      finish_table.call
      names
    end

    # classify_cargo_link_header -- How to read the lines under a header.
    #
    # The line is already comment-stripped and trimmed, and starts with `[`
    # and ends with `]`. Returns [kind, sub_key]. The header is split into
    # dotted-key segments at every `.` OUTSIDE a string and each segment is
    # decoded, so a cfg header is one segment however much punctuation it
    # quotes:
    #
    #   [target.'cfg(any(target_os = "linux", windows))'.dependencies]
    #    target  C (one quoted segment)                   D
    #
    #   [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table pi
    #   [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
    #   ["dependencies"]                                      -> [dependencies]
    #
    # Exactly [D], [D.N], [target.C.D] and [target.C.D.N] are read; arrays of
    # tables, malformed headers and every other segment list are ignored.
    def classify_cargo_link_header(line)
      return [:ignored, nil] if line.start_with?("[[") # e.g. [[bin]]

      segments = []
      split_toml_outside_strings(line.byteslice(1, line.bytesize - 2), ".", false).each do |raw|
        segment = toml_key_segment(raw)
        return [:ignored, nil] unless segment

        segments << segment
      end
      # Drop `target` and the cfg, whatever it is.
      segments = segments.drop(2) if segments.length >= 3 && segments.first == "target"
      return [:ignored, nil] unless CARGO_LINK_TABLE_NAMES.include?(segments.first)

      case segments.length
      when 1 then [:inline, nil]
      when 2 then [:sub, segments[1]]
      else [:ignored, nil]
      end
    end

    # cargo_path_dependency_name -- The crate a complete inline entry names.
    #
    # Only path dependencies can point at a repository package. The name is
    # the `package` rename if there is one, and the entry key otherwise:
    #
    #   logic-gates = { path = "../logic-gates" }                    -> logic-gates
    #   gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
    #   serde = "1"                                -> nil (not a table)
    #   x = { version = "1", features = ["path"] } -> nil (no path field)
    #   foo.path = "../foo"                        -> nil (dotted key)
    #
    # Fields are split at commas outside strings AND outside nested `[...]`
    # or `{...}`, so a `features` array cannot leak a fake field and a nested
    # table's `path` is not the entry's. A field key is decoded like an entry
    # key, so `"path"` and `'path'` are the key `path` (TOML, and so Cargo,
    # read them that way); the decoded key must then be exactly `path` or
    # `package` (Cargo ignores `Package`; escapes are not decoded). The first
    # usable field wins:
    #
    #   a = { "path" = "../a" }                 -> a
    #   x = { 'package' = "a", path = "../a" }  -> a
    #   a = { "Path" = "../a" }                 -> nil (keys are case-sensitive)
    #   a = { path.x = "../a" }                 -> nil (dotted field key)
    def cargo_path_dependency_name(key, value)
      crate_name = toml_key_segment(key)
      return nil unless crate_name

      value = trim_spaces_and_tabs(value)
      return nil unless value.bytesize >= 2 && value.start_with?("{") && value.end_with?("}")

      path = nil
      package_name = nil
      split_toml_outside_strings(value.byteslice(1, value.bytesize - 2), ",", true).each do |field|
        equals = index_toml_outside_strings(field, "=")
        next unless equals

        field_value = toml_single_string(field.byteslice((equals + 1)..))
        next unless field_value

        # A dotted or malformed field key is no key at all (nil).
        case toml_key_segment(field.byteslice(0, equals))
        when "path" then path ||= field_value
        when "package" then package_name ||= field_value
        end
      end
      return nil unless path

      ascii_downcase(package_name || crate_name)
    end

    # toml_string_end -- Index just past the string opening at text[start].
    #
    # text[start] is `"` or `'`. Returns nil when the string is not closed on
    # this line (strings never span lines here). In a basic "..." string a
    # backslash makes the next byte part of the string, so `\"` does not end
    # it; a literal '...' string has no escapes.
    def toml_string_end(text, start)
      quote = text.getbyte(start)
      index = start + 1
      while index < text.bytesize
        byte = text.getbyte(index)
        if quote == 0x22 && byte == 0x5C # a backslash inside "..."
          index += 2
          next
        end
        return index + 1 if byte == quote

        index += 1
      end
      nil
    end

    # Is the byte at text[index] a string opener (`"` or `'`)?
    def toml_quote_at?(text, index)
      byte = text.getbyte(index)
      [0x22, 0x27].include?(byte)
    end

    # strip_toml_line_comment -- Drop everything from the first `#` outside
    # a string. An unclosed string runs to the end of the line.
    def strip_toml_line_comment(line)
      index = 0
      while index < line.bytesize
        if toml_quote_at?(line, index)
          index = toml_string_end(line, index)
          return line unless index

          next
        end
        return line.byteslice(0, index) if line.getbyte(index) == 0x23 # "#"

        index += 1
      end
      line
    end

    # index_toml_outside_strings -- Index of the first `target` byte outside
    # every string, or nil.
    def index_toml_outside_strings(text, target)
      target_byte = target.ord
      index = 0
      while index < text.bytesize
        if toml_quote_at?(text, index)
          index = toml_string_end(text, index)
          return nil unless index

          next
        end
        return index if text.getbyte(index) == target_byte

        index += 1
      end
      nil
    end

    # TomlBracketScanner -- Count the `{`/`[` outside strings left unclosed,
    # over text fed in pieces.
    #
    # Feeding "ab" and then "cd" leaves exactly the state feeding "abcd"
    # would, because the scanner keeps what a rescan of the joined text would
    # have to rediscover: whether it is inside a string (and which delimiter
    # closes it), and whether the previous byte was a backslash escaping the
    # next one in a basic string. So a string still open at the end of one
    # line continues after the joining space, as it does in the joined value:
    #
    #   feed('{ x = "q\\')   depth 1, inside "...", escape pending
    #   feed(" ")             the space is escaped: still inside the string
    #   feed('", p = 1 }')    the string closes, `}` closes the table: 0
    #
    # Zero (or less) means the inline value is complete:
    #
    #   { path = "../a", features = [   -> 2
    #   { path = "../a" }               -> 0 (complete)
    #
    # Carrying this state is what keeps the continuation join linear: the
    # reader used to rescan the whole growing value for every appended line,
    # which is quadratic in a long entry.
    class TomlBracketScanner
      attr_reader :depth

      def initialize
        @depth = 0
        @quote = nil # the delimiter byte of the string being scanned, or nil
        @escape = false # the previous byte was a backslash inside "..."
      end

      def feed(text)
        text.each_byte do |byte|
          if @escape
            @escape = false
          elsif @quote
            if @quote == 0x22 && byte == 0x5C # a backslash inside "..."
              @escape = true
            elsif byte == @quote
              @quote = nil
            end
          elsif byte == 0x22 || byte == 0x27 # " '
            @quote = byte
          elsif byte == 0x7B || byte == 0x5B # { [
            @depth += 1
          elsif byte == 0x7D || byte == 0x5D # } ]
            @depth -= 1
          end
        end
        self
      end
    end

    # split_toml_outside_strings -- Split at every separator outside strings.
    #
    # With `nested`, a separator inside `[...]` or `{...}` does not split
    # either. An unclosed string runs to the end of the last part.
    def split_toml_outside_strings(text, separator, nested)
      separator_byte = separator.ord
      parts = []
      start = 0
      depth = 0
      index = 0
      while index < text.bytesize
        if toml_quote_at?(text, index)
          index = toml_string_end(text, index)
          break unless index

          next
        end
        byte = text.getbyte(index)
        if nested && [0x5B, 0x7B].include?(byte)
          depth += 1
        elsif nested && [0x5D, 0x7D].include?(byte)
          depth -= 1
        elsif byte == separator_byte && depth.zero?
          parts << text.byteslice(start, index - start)
          start = index + 1
        end
        index += 1
      end
      parts << text.byteslice(start..)
    end

    # toml_key_segment -- Decode one key or header segment, or nil.
    #
    # A bare key (ASCII letters, digits, `_`, `-`) or exactly one complete
    # string, whose value is the text between its quotes, verbatim (escapes
    # are not decoded): `dependencies`, `'cfg(unix)'` -> `cfg(unix)`.
    def toml_key_segment(raw)
      segment = trim_spaces_and_tabs(raw)
      return nil if segment.empty?

      if toml_quote_at?(segment, 0)
        return nil unless toml_string_end(segment, 0) == segment.bytesize

        return segment.byteslice(1, segment.bytesize - 2)
      end
      TOML_BARE_KEY.match?(segment) ? segment : nil
    end

    # toml_single_string -- The content of a value that is exactly one
    # non-empty string: `"1"` is one; `"a" "b"`, `1`, `""`, `["x"]` are not.
    def toml_single_string(raw)
      value = trim_spaces_and_tabs(raw)
      return nil if value.empty? || !toml_quote_at?(value, 0)
      return nil unless toml_string_end(value, 0) == value.bytesize

      content = value.byteslice(1, value.bytesize - 2)
      content.empty? ? nil : content
    end

    # Trim only spaces and tabs (String#strip also eats NUL, \v, \f ...).
    #
    # An index scan from each end, not `sub(/[ \t]+\z/, "")`: that regex
    # retries from every byte of an interior run of spaces and fails at
    # each, which is quadratic in a long run between two other characters.
    def trim_spaces_and_tabs(text)
      start = 0
      finish = text.bytesize
      start += 1 while start < finish && [0x20, 0x09].include?(text.getbyte(start))
      finish -= 1 while finish > start && [0x20, 0x09].include?(text.getbyte(finish - 1))
      text.byteslice(start, finish - start)
    end

    # Lower-case only A-Z, then present the name as UTF-8 so it can be
    # compared with the (UTF-8) known-name keys.
    def ascii_downcase(text)
      text.tr("A-Z", "a-z").force_encoding(Encoding::UTF_8)
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

    CARGO_PACKAGE_NAME = /^[ \t]*name[ \t]*=[ \t]*"([^"\n]+)"/

    def read_cargo_package_name(pkg)
      cargo_toml = pkg.path / "Cargo.toml"
      return nil unless cargo_toml.exist?

      # `[ \t]*`, not `\s*`: Ruby's `^` matches at every line start, and a
      # `\s*` there runs across all following blank lines and backtracks,
      # which is quadratic in a run of blank lines. Names never span lines.
      match = cargo_toml.read.match(CARGO_PACKAGE_NAME)
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

    # parse_build_tool_deps -- Read explicit qualified dependencies from the
    # selected legacy BUILD file. This comment contract is used when a runnable
    # program has no ecosystem manifest of its own.
    def parse_build_tool_deps(package, known_package_names)
      return [] if package.build_content.empty?

      dependencies = Set.new
      package.build_content.scan(/^#\s*build-tool:\s*deps\s*=\s*(.+)$/).each do |match|
        match.first.split(/[\s,]+/).each do |dependency|
          next if dependency.empty? || dependency == package.name
          next unless known_package_names.include?(dependency)

          dependencies << dependency
        end
      end
      dependencies.to_a.sort
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
    # @raise [MetadataEncodingError] a rockspec is not valid UTF-8.
    # @raise [DependencySelfEdgeError] a Cargo manifest resolves its own crate.
    def resolve_dependencies(packages)
      graph = DirectedGraph.new

      # Add all packages as nodes first.
      packages.each { |pkg| graph.add_node(pkg.name) }
      known_package_names = packages.map(&:name).to_set

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
        # Step 5 of the Rust contract: a Cargo candidate resolving to the
        # crate itself (by directory alias or [package] name) is rejected,
        # neither dropped nor drawn.
        if %w[rust wasm].include?(pkg.language) && deps.include?(pkg.name)
          raise DependencySelfEdgeError.new(
            package: pkg.name,
            manifest: repository_relative_manifest(pkg.path / "Cargo.toml"),
            dependency: pkg.name
          )
        end
        deps = (deps + parse_build_tool_deps(pkg, known_package_names)).uniq.sort

        deps.each do |dep_name|
          # Edge direction: dep -> pkg means "dep must be built before pkg".
          graph.add_edge(dep_name, pkg.name)
        end
      end

      graph
    end
  end
end
