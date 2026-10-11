# frozen_string_literal: true

# Pure graph and diff-selection contract for the Ruby build tool.
#
# Git, checkout discovery, JSON decoding, and fixture loading belong to the
# caller. This module accepts materialized values, validates the whole request
# before evaluating it, and returns only canonical value objects. In
# particular, a cycle or a selection error cannot leak a partial build plan.

require "digest"
require "json"
require_relative "glob_match"
require_relative "tracked_artifact_unicode17"

module BuildTool
  module GraphDiff
    MAX_PACKAGES = 4_096
    MAX_EDGES = 16_384
    MAX_SOURCE_GLOBS = 256
    MAX_PATH_SCALARS = 512
    MAX_NAME_SCALARS = 240
    MAX_MATCH_WORK = 50_000_000
    BUILD_FRONTS = %w[BUILD BUILD_windows BUILD_mac BUILD_linux BUILD_mac_and_linux].freeze
    BOUNDARY_DOMAIN = "coding-adventures/build-tool-repository-source-input-boundary/v1\0"
    PACKAGE_NAME = /\A[a-z0-9][a-z0-9._-]*(\/[a-z0-9][a-z0-9._-]*)+\z/
    DIGEST = /\A[0-9a-f]{64}\z/
    DRIVE_PREFIX = /\A[A-Za-z]:/
    RESERVED = (%w[CON PRN AUX NUL CONIN$ CONOUT$ CLOCK$] +
      (1..9).flat_map { |n| ["COM#{n}", "LPT#{n}"] } +
      %w[COM¹ COM² COM³ LPT¹ LPT² LPT³]).freeze

    GraphInput = Data.define(:packages, :edges)
    DiffInput = Data.define(:packages, :edges, :forced_packages,
      :unknown_path_policy, :changed_paths,
      :boundary_sha256, :boundary)
    Result = Data.define(:result, :error_code)

    class ContractError < ArgumentError
      attr_reader :code

      def initialize(code)
        @code = code
        super
      end
    end

    module_function

    # An edge [A, B] says B depends on A. Kahn traversal processes every
    # currently ready package in one sorted level; a disconnected cycle still
    # makes the *whole* result empty.
    def evaluate_graph(input)
      require_contract(input.is_a?(GraphInput), "GRAPH_INPUT_INVALID")
      graph = validate_graph(input.packages, input.edges)
      levels = levels_for(graph)
      return failure("GRAPH_CYCLE") if levels.nil?

      Result.new({"edges" => graph[:edges], "levels" => levels}, nil)
    end

    def evaluate_diff_selection(input)
      require_contract(input.is_a?(DiffInput), "DIFF_INPUT_INVALID")
      require_contract(input.packages.is_a?(Array), "DIFF_PACKAGE_INVALID")
      require_contract(input.packages.all? { |package| package.is_a?(Hash) && package.key?("name") },
        "DIFF_PACKAGE_INVALID")
      names = input.packages.map { |package| package.fetch("name") }
      graph = validate_graph(names, input.edges)
      require_contract(!levels_for(graph).nil?, "DIFF_EDGE_CYCLE")
      packages = validate_diff_input(input, graph)
      boundary_consumers = boundary_reverse_index(input, packages)

      # The fixed ceiling is charged *before* a single glob match. Every
      # applicable pattern is charged even if an earlier one would match.
      remaining = MAX_MATCH_WORK
      input.packages.each do |package|
        next unless package.fetch("source_mode") == "strict_globs"

        factor = package.fetch("source_globs", []).sum { |pattern| pattern.codepoints.length + 1 }
        input.changed_paths.each do |path|
          next unless inside?(path, package.fetch("rel_path"))

          relative = relative_path(path, package.fetch("rel_path"))
          next if BUILD_FRONTS.include?(basename(relative))

          path_factor = relative.codepoints.length + 1
          return failure("DIFF_MATCH_LIMIT_EXCEEDED") if factor.positive? && factor > remaining / path_factor

          remaining -= factor * path_factor
        end
      end

      changed = input.forced_packages.to_set
      unknown = false
      input.changed_paths.each do |path|
        consumers = boundary_consumers.fetch(path, [])
        changed.merge(consumers)
        known = !consumers.empty?
        input.packages.each do |package|
          root = package.fetch("rel_path")
          next unless inside?(path, root)

          known = true
          relative = relative_path(path, root)
          if package.fetch("source_mode") == "package_prefix" ||
              BUILD_FRONTS.include?(basename(relative)) ||
              package.fetch("source_globs", []).any? { |pattern| GlobMatch.match_path?(pattern, relative) }
            changed.add(package.fetch("name"))
          end
        end
        unknown ||= !known
      end
      return failure("DIFF_UNKNOWN_PATH") if unknown && input.unknown_path_policy == "error"

      changed = packages.keys.to_set if unknown
      affected = closure(changed, graph[:dependents])
      prerequisites = closure(affected, graph[:prerequisites]) - affected
      Result.new({
        "changed_packages" => changed.to_a.sort,
        "affected_packages" => affected.to_a.sort,
        "prerequisite_packages" => prerequisites.to_a.sort
      }, nil)
    end

    def validate_graph(packages, edges)
      require_contract(packages.is_a?(Array) && packages.length <= MAX_PACKAGES,
        "GRAPH_PACKAGE_LIMIT_EXCEEDED")
      require_contract(edges.is_a?(Array) && edges.length <= MAX_EDGES,
        "GRAPH_EDGE_LIMIT_EXCEEDED")
      names = {}
      packages.each do |name|
        require_contract(valid_package_name?(name), "GRAPH_PACKAGE_INVALID")
        require_contract(!names.key?(name), "GRAPH_PACKAGE_DUPLICATE")
        names[name] = true
      end
      seen_edges = {}
      dependents = names.to_h { |name, _| [name, []] }
      prerequisites = names.to_h { |name, _| [name, []] }
      edges.each do |edge|
        require_contract(edge.is_a?(Array) && edge.length == 2 &&
                         names.key?(edge[0]) && names.key?(edge[1]), "GRAPH_EDGE_UNKNOWN")
        require_contract(edge[0] != edge[1], "GRAPH_EDGE_SELF")
        require_contract(!seen_edges.key?(edge), "GRAPH_EDGE_DUPLICATE")
        seen_edges[edge] = true
        dependents[edge[0]] << edge[1]
        prerequisites[edge[1]] << edge[0]
      end
      {
        names: names.keys,
        edges: edges.sort,
        dependents: dependents.transform_values(&:sort),
        prerequisites: prerequisites.transform_values(&:sort)
      }
    end

    def levels_for(graph)
      indegree = graph[:names].to_h { |name| [name, 0] }
      graph[:edges].each { |_, dependent| indegree[dependent] += 1 }
      ready = indegree.select { |_, degree| degree.zero? }.keys.sort
      levels = []
      visited = 0
      until ready.empty?
        levels << ready
        following = []
        ready.each do |name|
          visited += 1
          graph[:dependents].fetch(name).each do |dependent|
            indegree[dependent] -= 1
            following << dependent if indegree[dependent].zero?
          end
        end
        ready = following.sort
      end
      (visited == graph[:names].length) ? levels : nil
    end

    def closure(seeds, adjacency)
      result = seeds.dup
      pending = seeds.to_a
      until pending.empty?
        adjacency.fetch(pending.shift).each do |name|
          next if result.include?(name)

          result.add(name)
          pending << name
        end
      end
      result
    end

    def validate_diff_input(input, graph)
      packages = {}
      root_identities = []
      input.packages.each do |package|
        require_contract(package.is_a?(Hash), "DIFF_PACKAGE_INVALID")
        name = package["name"]
        root = package["rel_path"]
        mode = package["source_mode"]
        globs = package.fetch("source_globs", [])
        require_contract(valid_package_name?(name), "DIFF_PACKAGE_INVALID")
        require_contract(valid_portable_path?(root), "DIFF_PATH_INVALID")
        identity = unicode.casefold(unicode.nfc(root))
        root_identities.each do |prior|
          require_contract(identity != prior && !identity.start_with?("#{prior}/") &&
                           !prior.start_with?("#{identity}/"), "DIFF_PATH_INVALID")
        end
        root_identities << identity
        require_contract(%w[package_prefix strict_globs].include?(mode), "DIFF_SOURCE_MODE_INVALID")
        require_contract(globs.is_a?(Array) && globs.length <= MAX_SOURCE_GLOBS &&
                         globs.uniq.length == globs.length, "DIFF_GLOB_INVALID")
        globs.each do |glob|
          require_contract(valid_portable_glob?(glob), "DIFF_GLOB_INVALID")
          begin
            GlobMatch.validate_pattern(glob)
          rescue GlobMatch::InvalidPatternError
            raise ContractError, "DIFF_GLOB_INVALID"
          end
        end
        require_contract(mode == "strict_globs" || globs.empty?, "DIFF_GLOB_INVALID")
        require_contract(!packages.key?(name), "DIFF_PACKAGE_DUPLICATE")
        packages[name] = package
      end
      require_contract(packages.keys.to_set == graph[:names].to_set, "DIFF_PACKAGE_INVALID")
      require_contract(%w[all error].include?(input.unknown_path_policy), "DIFF_POLICY_INVALID")
      require_contract(input.forced_packages.is_a?(Array) &&
                       input.forced_packages.length <= MAX_PACKAGES &&
                       input.forced_packages.uniq.length == input.forced_packages.length,
        "DIFF_FORCED_PACKAGE_INVALID")
      require_contract(input.changed_paths.is_a?(Array) &&
                       input.changed_paths.length <= MAX_PACKAGES &&
                       input.changed_paths.uniq.length == input.changed_paths.length,
        "DIFF_PATH_INVALID")
      input.changed_paths.each do |path|
        require_contract(valid_portable_path?(path), "DIFF_PATH_INVALID")
      end
      input.forced_packages.each do |name|
        require_contract(packages.key?(name), "DIFF_FORCED_PACKAGE_UNKNOWN")
      end
      packages
    end

    def boundary_reverse_index(input, packages)
      require_contract(input.boundary_sha256.is_a?(String) &&
                       input.boundary_sha256.encoding == Encoding::UTF_8 &&
                       input.boundary_sha256.valid_encoding?, "DIFF_BOUNDARY_DIGEST_MISMATCH")
      if input.boundary_sha256.empty?
        require_contract(input.boundary.nil?, "DIFF_BOUNDARY_DIGEST_MISMATCH")
        return {}
      end
      require_contract(input.boundary_sha256.match?(DIGEST) && input.boundary.is_a?(Hash),
        "DIFF_BOUNDARY_DIGEST_MISMATCH")
      boundary = input.boundary
      begin
        encoded = JSON.generate(canonical(boundary)).encode("UTF-8")
      rescue JSON::GeneratorError, ArgumentError, TypeError
        raise ContractError, "DIFF_BOUNDARY_DIGEST_MISMATCH"
      end
      framed = BOUNDARY_DOMAIN.b + [encoded.bytesize].pack("Q>") + encoded.b
      require_contract(Digest::SHA256.hexdigest(framed) == input.boundary_sha256,
        "DIFF_BOUNDARY_DIGEST_MISMATCH")
      require_contract(boundary["schema_version"] == 1 && boundary["boundaries"].is_a?(Array),
        "DIFF_BOUNDARY_INVALID")
      consumers = Hash.new { |hash, key| hash[key] = Set.new }
      boundary.fetch("boundaries").each do |rule|
        require_contract(rule.is_a?(Hash) && rule["applies_to"].is_a?(Hash) &&
                         rule["inputs"].is_a?(Array), "DIFF_BOUNDARY_INVALID")
        applies = rule.fetch("applies_to")
        require_contract(%w[exact_roots excluded_roots descendant_roots].all? do |key|
          applies[key].is_a?(Array) && applies[key].all? { |root| valid_portable_path?(root) }
        end, "DIFF_BOUNDARY_INVALID")
        require_contract(rule.fetch("inputs").all? do |source|
          source.is_a?(Hash) && valid_portable_path?(source["path"])
        end, "DIFF_BOUNDARY_INVALID")
      end
      packages.each_value do |package|
        boundary.fetch("boundaries").each do |rule|
          next unless applies_to?(rule.fetch("applies_to"), package.fetch("rel_path"))

          rule.fetch("inputs").each do |source|
            consumers[source.fetch("path")].add(package.fetch("name"))
          end
        end
      end
      consumers.transform_values(&:to_a)
    end

    def canonical(value)
      case value
      when Hash
        value.keys.sort.to_h { |key| [key, canonical(value.fetch(key))] }
      when Array
        value.map { |item| canonical(item) }
      else
        value
      end
    end

    def applies_to?(value, root)
      return true if value.fetch("exact_roots").include?(root)
      return false if value.fetch("excluded_roots").include?(root)

      value.fetch("descendant_roots").any? { |parent| root.start_with?("#{parent}/") }
    end

    def inside?(path, root)
      path == root || path.start_with?("#{root}/")
    end

    def relative_path(path, root)
      (path == root) ? "" : path.delete_prefix("#{root}/")
    end

    def basename(path)
      path.split("/").last || ""
    end

    def unicode
      BuildTool::TrackedArtifactUnicode17
    end

    def valid_package_name?(value)
      value.is_a?(String) && value.encoding == Encoding::UTF_8 && value.valid_encoding? &&
        value.codepoints.length <= MAX_NAME_SCALARS && value.match?(PACKAGE_NAME)
    end

    def valid_portable_path?(value)
      return false unless portable_prefix?(value, /[<>:"|?*]/)

      value.split("/", -1).all? { |segment| valid_segment?(segment) }
    end

    def valid_portable_glob?(value)
      return false unless portable_prefix?(value, /[<>:"|?]/)

      value.split("/", -1).all? do |segment|
        !segment.empty? && segment != "." && segment != ".." &&
          !segment.end_with?(" ", ".") &&
          (segment.match?(/[*\[\]{}]/) || !reserved?(segment))
      end
    end

    def portable_prefix?(value, forbidden)
      value.is_a?(String) && value.encoding == Encoding::UTF_8 && value.valid_encoding? && !value.empty? &&
        value.codepoints.length <= MAX_PATH_SCALARS &&
        unicode.nfc(value) == value && !value.start_with?("/") &&
        !value.match?(DRIVE_PREFIX) && !value.include?("\\") &&
        !value.include?("//") && !value.match?(forbidden) &&
        value.each_codepoint.none? { |scalar| scalar < 32 }
    end

    def valid_segment?(segment)
      !segment.empty? && segment != "." && segment != ".." &&
        !segment.end_with?(" ", ".") && !reserved?(segment)
    end

    def reserved?(segment)
      base = segment.split(".", 2).first
      RESERVED.include?(unicode.full_uppercase(base))
    end

    def failure(code)
      Result.new({}, code)
    end

    def require_contract(condition, code)
      raise ContractError, code unless condition
    end
  end
end
