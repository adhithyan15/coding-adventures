# frozen_string_literal: true

# ---------------------------------------------------------------------------
# flag_validator.rb — Validate parsed flags against the spec's constraints
# ---------------------------------------------------------------------------
#
# After scanning collects the parsed_flags hash, the flag validator checks
# that all constraints from the spec are satisfied:
#
#   1. Required flags are present (or exempted by required_unless)
#   2. Conflicting flags are not both present
#   3. Dependencies are satisfied (transitively via G_flag)
#   4. Mutually exclusive groups have at most one member present
#   5. Non-repeatable flags have not been duplicated
#
# === Using DirectedGraph for transitive requires ===
#
# The "requires" constraint is transitive. If A requires B, and B requires C,
# then using A implies B and C must also be present — even if A does not
# directly list C in its requires array.
#
# We enforce this by building the flag dependency graph G_flag and calling
# transitive_closure on it. The closure maps each flag id to the set of
# all flags it transitively requires. For each flag in parsed_flags, we
# check that every flag in its closure is also present.
#
# === Collecting all errors ===
#
# The validator does NOT stop at the first error. It collects all violations
# and returns them as an array of ParseError objects. The parser then
# decides whether to raise them as a ParseErrors exception.
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Validates a parsed flag set against the spec's flag constraints.
    class FlagValidator
      # Create a validator for the given scope's flags and exclusive groups.
      #
      # @param active_flags [Array<Hash>] All flags in scope (global + command-specific + builtins).
      # @param mutually_exclusive_groups [Array<Hash>] The exclusive groups for this scope.
      def initialize(active_flags, mutually_exclusive_groups)
        @active_flags = active_flags
        @mutually_exclusive_groups = mutually_exclusive_groups || []

        # Build a lookup map from flag id → flag def for fast access
        @flag_by_id = {}
        active_flags.each { |f| @flag_by_id[f["id"]] = f }

        # Build the flag dependency graph once for transitive closure queries.
        # G_flag has one node per flag, one edge A→B for "A requires B".
        @g_flag = build_flag_graph
        @closure = @g_flag.transitive_closure
      end

      # Validate parsed_flags against all constraints.
      #
      # @param parsed_flags [Hash] Map from flag id → value.
      # @param duplicate_flags [Array<String>] Flag ids that appeared more than once.
      # @return [Array<ParseError>] All violations found (empty array = valid).
      def validate(parsed_flags, duplicate_flags = [])
        errors = []

        validate_duplicates(duplicate_flags, errors)
        validate_conflicts(parsed_flags, errors)
        validate_requires(parsed_flags, errors)
        validate_required_flags(parsed_flags, errors)
        validate_exclusive_groups(parsed_flags, errors)

        errors
      end

      private

      # ---------------------------------------------------------------------------
      # Build the flag dependency graph (G_flag)
      # ---------------------------------------------------------------------------

      def build_flag_graph
        g = CodingAdventures::DirectedGraph::Graph.new
        @active_flags.each { |f| g.add_node(f["id"]) }
        @active_flags.each do |f|
          (f["requires"] || []).each do |req_id|
            g.add_edge(f["id"], req_id) if g.has_node?(req_id)
          end
        end
        g
      end

      # ---------------------------------------------------------------------------
      # Rule 1: Duplicate non-repeatable flags
      # ---------------------------------------------------------------------------
      #
      # If the scanner saw a flag more than once and the flag is not repeatable,
      # that is an error. The scanner tracks these separately because it sees each
      # token sequentially — by the time validation runs, the flag's value has
      # already been overwritten.

      def validate_duplicates(duplicate_flags, errors)
        duplicate_flags.each do |fid|
          flag = @flag_by_id[fid]
          next unless flag
          next if flag["repeatable"]

          # Determine the best display name for the error message
          name = flag_display_name(flag)
          errors << ParseError.new(
            error_type: "duplicate_flag",
            message: "#{name} specified more than once",
            suggestion: nil,
            context: []
          )
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 2: conflicts_with violations
      # ---------------------------------------------------------------------------
      #
      # For each flag that is present, check all flags it conflicts with. If any
      # of those are also present, record a conflict error.
      #
      # Note: conflicts_with is defined bilaterally in the spec (if A lists B, the
      # spec should also have B list A), but we only need to detect the pair once.
      # We use a "canonical pair" set to avoid reporting A↔B and B↔A separately.

      def validate_conflicts(parsed_flags, errors)
        reported = Set.new

        parsed_flags.each_key do |fid|
          flag = @flag_by_id[fid]
          next unless flag

          (flag["conflicts_with"] || []).each do |other_id|
            next unless parsed_flags.key?(other_id)

            # Canonical pair: sort the two ids so we only report once
            pair = [fid, other_id].sort
            next if reported.include?(pair)
            reported.add(pair)

            name_a = flag_display_name(flag)
            name_b = flag_display_name(@flag_by_id[other_id])
            errors << ParseError.new(
              error_type: "conflicting_flags",
              message: "#{name_a} and #{name_b} cannot be used together",
              suggestion: nil,
              context: []
            )
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 3: requires violations (transitive)
      # ---------------------------------------------------------------------------
      #
      # For each flag that is present, look up all flags it transitively requires.
      # Any transitively required flag that is absent is an error.
      #
      # Example: If -h/--human-readable requires -l/--long-listing, and the user
      # types `ls -h`, we report: "-h/--human-readable requires -l/--long-listing".

      def validate_requires(parsed_flags, errors)
        parsed_flags.each_key do |fid|
          required_ids = @closure[fid] || Set.new
          required_ids.each do |req_id|
            next if parsed_flags.key?(req_id)

            flag = @flag_by_id[fid]
            req_flag = @flag_by_id[req_id]
            next unless flag && req_flag

            name_src = flag_display_name(flag)
            name_req = flag_display_name(req_flag)
            errors << ParseError.new(
              error_type: "missing_dependency_flag",
              message: "#{name_src} requires #{name_req}",
              suggestion: nil,
              context: []
            )
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 4: required flags
      # ---------------------------------------------------------------------------
      #
      # Check every flag definition that has required: true. If it is absent from
      # parsed_flags, check whether required_unless is satisfied. If not, error.

      def validate_required_flags(parsed_flags, errors)
        @active_flags.each do |flag|
          next unless flag["required"]
          next if parsed_flags.key?(flag["id"])

          # Check required_unless: if any of the listed flag ids IS present, exempt
          exempt_ids = flag["required_unless"] || []
          next if exempt_ids.any? { |eid| parsed_flags.key?(eid) }

          name = flag_display_name(flag)
          errors << ParseError.new(
            error_type: "missing_required_flag",
            message: "#{name} is required",
            suggestion: nil,
            context: []
          )
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 5: mutually exclusive group violations
      # ---------------------------------------------------------------------------
      #
      # For each group, count how many of its flag_ids are present.
      #   - More than one present → exclusive_group_violation
      #   - Zero present and group.required → missing_exclusive_group

      def validate_exclusive_groups(parsed_flags, errors)
        @mutually_exclusive_groups.each do |group|
          flag_ids = group["flag_ids"] || []
          present = flag_ids.select { |fid| parsed_flags.key?(fid) }

          if present.size > 1
            # Build a human-readable list of the conflicting flags
            names = present.map { |fid| flag_display_name(@flag_by_id[fid]) }.compact
            errors << ParseError.new(
              error_type: "exclusive_group_violation",
              message: "Only one of #{names.join(", ")} may be used",
              suggestion: nil,
              context: []
            )
          elsif present.empty? && group["required"]
            # Required group with no member present
            names = flag_ids.map { |fid| flag_display_name(@flag_by_id[fid]) }.compact
            errors << ParseError.new(
              error_type: "missing_exclusive_group",
              message: "One of #{names.join(", ")} is required",
              suggestion: nil,
              context: []
            )
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Display name helpers
      # ---------------------------------------------------------------------------

      # Build a human-readable name for a flag, like "-l/--long-listing" or "--verbose".
      def flag_display_name(flag)
        return "???" unless flag

        parts = []
        parts << "-#{flag["short"]}" if flag["short"]
        parts << "--#{flag["long"]}" if flag["long"]
        parts << "-#{flag["single_dash_long"]}" if flag["single_dash_long"]
        parts.join("/")
      end
    end
  end
end
