# frozen_string_literal: true

require "json"

# ---------------------------------------------------------------------------
# spec_loader.rb — Load and validate a CLI Builder JSON specification
# ---------------------------------------------------------------------------
#
# The SpecLoader is responsible for one thing: reading a JSON file from disk,
# checking that it is a valid CLI spec, and returning a normalized Ruby hash
# ready for the Parser to consume.
#
# === Why validate before parsing? ===
#
# Spec errors are programmer bugs, not user input errors. They should be
# caught at startup — before any argv is processed — so that the developer
# gets immediate feedback, not a cryptic runtime error when a user happens
# to type a particular sequence of flags.
#
# The eight validation rules (see §13 / spec §6.4.3) cover:
#
#   1. Version check          — spec version must be "1.0"
#   2. Required top-level     — name and description must be present
#   3. Duplicate IDs          — no two flags, commands, or args share an id
#   4. Flag name presence     — every flag has at least one of short/long/single_dash_long
#   5. Cross-references       — conflicts_with/requires reference valid ids
#   6. Exclusive groups       — mutually_exclusive_groups reference valid ids
#   7. Enum values            — enum-type flags/args have non-empty enum_values
#   8. Variadic uniqueness    — at most one variadic arg per scope
#   9. Cycle detection        — flag dependency graph has no cycles
#
# === Using DirectedGraph for cycle detection ===
#
# The flag dependency graph G_flag has one node per flag and one edge A→B
# for every "A requires B" relationship. A cycle in this graph means the
# spec is self-contradictory (e.g. -v requires -q AND -q requires -v).
#
# We build G_flag using CodingAdventures::DirectedGraph::Graph and call
# has_cycle? on it. If it returns true, we raise a SpecError naming the
# problematic scope.
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Loads and validates a CLI Builder JSON specification file.
    #
    # Usage:
    #   loader = SpecLoader.new("/path/to/my-tool.json")
    #   spec   = loader.load   # returns normalized Hash or raises SpecError
    class SpecLoader
      # Create a SpecLoader for the given spec file path.
      #
      # @param spec_file_path [String] Absolute or relative path to the JSON spec file.
      def initialize(spec_file_path)
        @spec_file_path = spec_file_path
      end

      # Load and validate the spec file.
      #
      # Reads the file, parses JSON, runs all validation checks, and returns
      # a normalized Hash. Raises SpecError on any validation failure.
      #
      # @return [Hash] The validated spec hash with normalized defaults.
      # @raise [SpecError] If the file is missing, not valid JSON, or fails any check.
      def load
        raw = read_file
        spec = parse_json(raw)
        validate(spec)
        normalize(spec)
      end

      private

      # ---------------------------------------------------------------------------
      # File I/O
      # ---------------------------------------------------------------------------

      def read_file
        File.read(@spec_file_path)
      rescue Errno::ENOENT
        raise SpecError, "Spec file not found: #{@spec_file_path}"
      rescue Errno::EACCES
        raise SpecError, "Cannot read spec file (permission denied): #{@spec_file_path}"
      end

      def parse_json(raw)
        JSON.parse(raw)
      rescue JSON::ParserError => e
        raise SpecError, "Invalid JSON in spec file: #{e.message}"
      end

      # ---------------------------------------------------------------------------
      # Normalization — fill in defaults so downstream code never has to nil-check
      # ---------------------------------------------------------------------------

      def normalize(spec)
        spec = spec.dup
        spec["parsing_mode"] ||= "gnu"
        spec["builtin_flags"] ||= {"help" => true, "version" => true}
        spec["builtin_flags"]["help"] = true unless spec["builtin_flags"].key?("help")
        spec["builtin_flags"]["version"] = true unless spec["builtin_flags"].key?("version")
        spec["global_flags"] ||= []
        spec["flags"] ||= []
        spec["arguments"] ||= []
        spec["commands"] ||= []
        spec["mutually_exclusive_groups"] ||= []
        spec["global_flags"] = normalize_flags(spec["global_flags"])
        spec["flags"] = normalize_flags(spec["flags"])
        spec["arguments"] = normalize_arguments(spec["arguments"])
        spec["commands"] = normalize_commands(spec["commands"])
        spec
      end

      def normalize_flags(flags)
        flags.map do |f|
          f = f.dup
          f["required"] ||= false
          f["repeatable"] ||= false
          f["conflicts_with"] ||= []
          f["requires"] ||= []
          f["required_unless"] ||= []
          f["enum_values"] ||= []
          f
        end
      end

      def normalize_arguments(args)
        args.map do |a|
          a = a.dup
          # Accept display_name (preferred) or name (backward compatibility).
          a["display_name"] ||= a["name"]
          a["required"] = a.key?("required") ? a["required"] : true
          a["variadic"] ||= false
          a["variadic_min"] = a.key?("variadic_min") ? a["variadic_min"] : (a["required"] ? 1 : 0)
          a["variadic_max"] ||= nil
          a["enum_values"] ||= []
          a["required_unless_flag"] ||= []
          a
        end
      end

      def normalize_commands(commands)
        commands.map do |c|
          c = c.dup
          c["aliases"] ||= []
          c["inherit_global_flags"] = c.key?("inherit_global_flags") ? c["inherit_global_flags"] : true
          c["flags"] = normalize_flags(c["flags"] || [])
          c["arguments"] = normalize_arguments(c["arguments"] || [])
          c["commands"] = normalize_commands(c["commands"] || [])
          c["mutually_exclusive_groups"] ||= []
          c
        end
      end

      # ---------------------------------------------------------------------------
      # Validation — all 8 rules from spec §6.4.3
      # ---------------------------------------------------------------------------

      def validate(spec)
        # Rule 1: Spec version must be "1.0"
        validate_version(spec)

        # Rule 2: Required top-level fields
        validate_required_fields(spec)

        # Rules 3-9: Recursive validation of each scope
        validate_scope(spec, spec["global_flags"] || [], "root", [])

        commands = spec["commands"] || []
        validate_commands_recursive(commands, spec["global_flags"] || [], "root")
      end

      # ---------------------------------------------------------------------------
      # Rule 1: Version check
      # ---------------------------------------------------------------------------
      #
      # The spec must declare cli_builder_spec_version = "1.0". This allows
      # future versions of the library to support multiple spec formats while
      # rejecting specs written for incompatible future versions.

      def validate_version(spec)
        version = spec["cli_builder_spec_version"]
        if version.nil?
          raise SpecError, "Missing required field: cli_builder_spec_version"
        end
        unless version == "1.0"
          raise SpecError, "Unsupported spec version: #{version.inspect}. Expected \"1.0\"."
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 2: Required top-level fields
      # ---------------------------------------------------------------------------

      def validate_required_fields(spec)
        %w[name description].each do |field|
          if spec[field].nil? || spec[field].to_s.strip.empty?
            raise SpecError, "Missing required top-level field: #{field}"
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Scope validation — validates flags, arguments, commands, and groups
      # in a single command scope. Called for root and every command.
      # ---------------------------------------------------------------------------

      def validate_scope(spec_or_cmd, inherited_global_flags, scope_name, parent_global_flags)
        local_flags = (spec_or_cmd["flags"] || [])
        global_flags = spec_or_cmd["global_flags"] || parent_global_flags
        all_flags = local_flags + global_flags
        arguments = spec_or_cmd["arguments"] || []
        groups = spec_or_cmd["mutually_exclusive_groups"] || []

        # Rule 3: No duplicate ids in this scope
        validate_no_duplicate_ids(local_flags, arguments, spec_or_cmd["commands"] || [], scope_name)

        # Rule 4: Every flag has at least one of short/long/single_dash_long
        validate_flag_names(all_flags, scope_name)

        # Rule 5: Cross-references in conflicts_with and requires
        all_flag_ids = all_flags.map { |f| f["id"] }
        validate_cross_references(all_flags, all_flag_ids, scope_name)

        # Rule 6: Mutually exclusive groups reference valid flag ids
        validate_exclusive_groups(groups, all_flag_ids, scope_name)

        # Rule 7: Enum flags have non-empty enum_values
        validate_enum_values(all_flags + arguments, scope_name)

        # Rule 8: At most one variadic argument per scope
        validate_variadic_uniqueness(arguments, scope_name)

        # Rule 9: Flag dependency graph has no cycles
        validate_no_flag_cycles(all_flags, scope_name)

        # Rule 10 (v1.1): default_when_present is valid for enum flags only
        validate_default_when_present(all_flags, scope_name)
      end

      # ---------------------------------------------------------------------------
      # Rule 3: Duplicate IDs
      # ---------------------------------------------------------------------------
      #
      # Within any scope (root or a specific command), every flag id, argument id,
      # and command id must be unique. We check each category separately so the
      # error message can pinpoint what kind of id was duplicated.

      def validate_no_duplicate_ids(flags, arguments, commands, scope_name)
        check_duplicates(flags.map { |f| f["id"] }, "flag id", scope_name)
        check_duplicates(arguments.map { |a| a["id"] }, "argument id", scope_name)
        check_duplicates(commands.map { |c| c["id"] }, "command id", scope_name)

        # Also check that command names and aliases are unique among siblings
        all_names = commands.flat_map { |c| [c["name"]] + (c["aliases"] || []) }
        check_duplicates(all_names, "command name/alias", scope_name)
      end

      def check_duplicates(ids, kind, scope_name)
        seen = {}
        ids.each do |id|
          next if id.nil?
          if seen[id]
            raise SpecError, "Duplicate #{kind} #{id.inspect} in scope #{scope_name.inspect}"
          end
          seen[id] = true
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 4: Flag name presence
      # ---------------------------------------------------------------------------
      #
      # A flag with no short, long, or single_dash_long field cannot be typed by
      # the user — it would be unreachable. This is always a spec bug.

      def validate_flag_names(flags, scope_name)
        flags.each do |f|
          has_name = f["short"] || f["long"] || f["single_dash_long"]
          unless has_name
            raise SpecError,
              "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has no " \
              "short, long, or single_dash_long field"
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 5: Cross-references
      # ---------------------------------------------------------------------------
      #
      # Every id listed in conflicts_with and requires must exist in the same scope
      # (including global_flags). Forward references are not allowed.

      def validate_cross_references(flags, all_flag_ids, scope_name)
        flags.each do |f|
          (f["conflicts_with"] || []).each do |ref_id|
            unless all_flag_ids.include?(ref_id)
              raise SpecError,
                "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
                "conflicts_with reference to unknown id #{ref_id.inspect}"
            end
          end
          (f["requires"] || []).each do |ref_id|
            unless all_flag_ids.include?(ref_id)
              raise SpecError,
                "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
                "requires reference to unknown id #{ref_id.inspect}"
            end
          end
          (f["required_unless"] || []).each do |ref_id|
            unless all_flag_ids.include?(ref_id)
              raise SpecError,
                "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
                "required_unless reference to unknown id #{ref_id.inspect}"
            end
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 6: Mutually exclusive groups
      # ---------------------------------------------------------------------------
      #
      # Every flag_id listed in a mutually_exclusive_groups entry must exist in the
      # same scope. Groups with fewer than 2 flags are degenerate (useless) and
      # likely a mistake.

      def validate_exclusive_groups(groups, all_flag_ids, scope_name)
        groups.each do |group|
          (group["flag_ids"] || []).each do |fid|
            unless all_flag_ids.include?(fid)
              raise SpecError,
                "Mutually exclusive group #{group["id"].inspect} in scope " \
                "#{scope_name.inspect} references unknown flag id #{fid.inspect}"
            end
          end
          if (group["flag_ids"] || []).size < 2
            raise SpecError,
              "Mutually exclusive group #{group["id"].inspect} in scope " \
              "#{scope_name.inspect} must contain at least 2 flag ids"
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 7: Enum values
      # ---------------------------------------------------------------------------
      #
      # When type is "enum", the enum_values array must be present and non-empty.
      # An enum flag with no valid values would always reject every input — another
      # guaranteed spec bug.

      def validate_enum_values(items, scope_name)
        items.each do |item|
          next unless item["type"] == "enum"
          if (item["enum_values"] || []).empty?
            raise SpecError,
              "Item #{item["id"].inspect} in scope #{scope_name.inspect} has " \
              "type 'enum' but no enum_values"
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 10: default_when_present validation (v1.1)
      # ---------------------------------------------------------------------------
      #
      # When a flag specifies default_when_present, three constraints must hold:
      #
      #   1. The flag's type must be "enum". Using default_when_present on a
      #      non-enum flag makes no sense — it's designed for the "flag present
      #      without value" pattern, which only applies to enums.
      #
      #   2. The value must be one of enum_values. Otherwise, the parser would
      #      produce a value that fails its own validation.
      #
      #   3. enum_values must not be empty. (This is already caught by Rule 7,
      #      but we check it here too for a better error message.)

      def validate_default_when_present(flags, scope_name)
        flags.each do |f|
          next unless f.key?("default_when_present")

          unless f["type"] == "enum"
            raise SpecError,
              "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
              "default_when_present but type is #{f["type"].inspect} (must be \"enum\")"
          end

          enum_values = f["enum_values"] || []
          if enum_values.empty?
            raise SpecError,
              "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
              "default_when_present but enum_values is empty"
          end

          unless enum_values.include?(f["default_when_present"])
            raise SpecError,
              "Flag #{f["id"].inspect} in scope #{scope_name.inspect} has " \
              "default_when_present value #{f["default_when_present"].inspect} " \
              "which is not in enum_values: #{enum_values.inspect}"
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 8: At most one variadic argument
      # ---------------------------------------------------------------------------
      #
      # If two arguments were both variadic, there would be no way to decide where
      # one ends and the other begins. The spec forbids this.

      def validate_variadic_uniqueness(arguments, scope_name)
        variadic_count = arguments.count { |a| a["variadic"] }
        if variadic_count > 1
          raise SpecError,
            "Scope #{scope_name.inspect} has #{variadic_count} variadic arguments; " \
            "at most one is allowed"
        end
      end

      # ---------------------------------------------------------------------------
      # Rule 9: Cycle detection in flag dependency graph
      # ---------------------------------------------------------------------------
      #
      # Build G_flag: one node per flag, one edge A→B for each "A requires B".
      # Call has_cycle? on the graph. A cycle means the spec is self-contradictory.
      #
      # Example of a cycle:
      #   --verbose requires --debug
      #   --debug requires --verbose
      #
      # This is impossible to satisfy: using either flag would require the other,
      # and the user would need to supply both, but then each would still "require"
      # the other in an infinite regression. The spec validator catches this before
      # any user interaction occurs.

      def validate_no_flag_cycles(flags, scope_name)
        g = CodingAdventures::DirectedGraph::Graph.new
        flags.each { |f| g.add_node(f["id"]) }
        flags.each do |f|
          (f["requires"] || []).each do |req_id|
            next unless g.has_node?(req_id)
            # add_edge raises CycleError for self-loops (A requires A)
            begin
              g.add_edge(f["id"], req_id)
            rescue CodingAdventures::DirectedGraph::CycleError
              raise SpecError,
                "Circular requires dependency (self-loop) for flag #{f["id"].inspect} " \
                "in scope #{scope_name.inspect}"
            end
          end
        end
        if g.has_cycle?
          raise SpecError,
            "Circular requires dependency detected in flag graph for scope #{scope_name.inspect}"
        end
      end

      # ---------------------------------------------------------------------------
      # Recursive command validation
      # ---------------------------------------------------------------------------

      def validate_commands_recursive(commands, global_flags, parent_scope)
        commands.each do |cmd|
          scope_name = "#{parent_scope}/#{cmd["name"]}"
          cmd_global = cmd.key?("inherit_global_flags") && !cmd["inherit_global_flags"] ? [] : global_flags
          # Build a synthetic scope object
          scope = {
            "flags" => cmd["flags"] || [],
            "global_flags" => cmd_global,
            "arguments" => cmd["arguments"] || [],
            "commands" => cmd["commands"] || [],
            "mutually_exclusive_groups" => cmd["mutually_exclusive_groups"] || []
          }
          validate_scope(scope, cmd_global, scope_name, global_flags)
          validate_commands_recursive(cmd["commands"] || [], global_flags, scope_name)
        end
      end
    end
  end
end
