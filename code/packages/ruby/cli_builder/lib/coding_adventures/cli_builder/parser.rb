# frozen_string_literal: true

# ---------------------------------------------------------------------------
# parser.rb — The main CLI Builder parser
# ---------------------------------------------------------------------------
#
# The Parser is the orchestrator: it ties together SpecLoader, DirectedGraph
# routing, the ModalStateMachine scanning loop, TokenClassifier, FlagValidator,
# PositionalResolver, and HelpGenerator into a single #parse method.
#
# === Three-Phase Architecture ===
#
# Parsing proceeds in three sequential phases:
#
#   Phase 1 — Routing (Directed Graph)
#   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
#   Walk argv looking for subcommand tokens. Use G_cmd (a directed graph of
#   the command tree) to transition between command nodes. Stop when a token
#   doesn't match any outgoing edge.
#
#   This phase determines the "command path" (e.g. ["git", "remote", "add"])
#   and the "active context" — which flags and arguments are in scope.
#
#   Phase 2 — Scanning (Modal State Machine)
#   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
#   Re-walk argv, skipping tokens consumed in Phase 1. Drive the scan with
#   a Modal State Machine that has three modes:
#
#     SCANNING    — normal mode: classify each token, dispatch on type
#     FLAG_VALUE  — entered after a value-taking flag; next token is the value
#     END_OF_FLAGS — entered after "--"; all remaining tokens are positional
#
#   The state machine transitions are:
#
#     SCANNING --[non-boolean flag seen]--> FLAG_VALUE
#     SCANNING --["--" token seen]-------> END_OF_FLAGS
#     SCANNING --[posix mode + positional]-> END_OF_FLAGS
#     FLAG_VALUE --[any token]-----------> SCANNING
#     END_OF_FLAGS (terminal — no outgoing transitions)
#
#   Phase 3 — Validation
#   ~~~~~~~~~~~~~~~~~~~~~~
#   Run FlagValidator on the collected parsed_flags, then PositionalResolver
#   on the collected positional_tokens. Accumulate all errors. Raise ParseErrors
#   if any errors were found.
#
# === Help and Version Short-Circuits ===
#
# If --help or -h is seen at any point during Phase 2, parsing stops
# immediately and returns a HelpResult. If --version is seen, returns
# a VersionResult. These bypass Phase 3 entirely.
#
# === Traditional Mode ===
#
# When parsing_mode is "traditional" (tar-style), the first non-flag token
# that doesn't match a subcommand is treated as a stack of short flag
# characters WITHOUT a leading dash. "tar xvf" → ["tar", "-x", "-v", "-f"].
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # The main CLI Builder parser.
    #
    # Usage:
    #   parser = Parser.new("/path/to/spec.json", ARGV)
    #   result = parser.parse
    #   case result
    #   when ParseResult   then run_program(result)
    #   when HelpResult    then puts result.text; exit 0
    #   when VersionResult then puts result.version; exit 0
    #   end
    class Parser
      # Create a parser for the given spec file and argv.
      #
      # @param spec_file_path [String] Path to the JSON spec file (or nil if passing spec_hash).
      # @param argv [Array<String>] The argument vector as passed to the program.
      # @param spec_hash [Hash, nil] Pre-parsed spec hash (skips file loading). Used for testing.
      def initialize(spec_file_path, argv, spec_hash: nil)
        if spec_hash
          @spec = spec_hash
        else
          loader = SpecLoader.new(spec_file_path)
          @spec = loader.load
        end
        @argv = argv.dup

        # Build the command routing graph G_cmd once at construction time.
        # This graph is shared across all parse() calls (Parser instances are
        # typically single-use, but building the graph is idempotent).
        @g_cmd = build_routing_graph
      end

      # Parse the argv and return a result.
      #
      # @return [ParseResult, HelpResult, VersionResult]
      # @raise [ParseErrors] If parsing fails due to user input errors.
      def parse
        # Strip argv[0] (the program name itself)
        program = @argv[0] || @spec["name"]
        tokens = @argv[1..] || []

        # Phase 1: Route to the correct command
        command_path, _current_node_id, _remaining_tokens, consumed_indices =
          phase1_routing(tokens, program)

        # Build the active flag set for the resolved context
        active_flags = build_active_flags(command_path)
        current_node_def = resolve_node_def(command_path)
        active_args = current_node_def["arguments"] || []
        active_groups = current_node_def["mutually_exclusive_groups"] || []

        # Phase 2: Scan tokens with Modal State Machine
        scan_result = phase2_scanning(tokens, consumed_indices, active_flags, command_path)

        # Handle help/version short-circuits
        return scan_result if scan_result.is_a?(HelpResult) || scan_result.is_a?(VersionResult)

        parsed_flags, positional_tokens, scan_errors, duplicate_flags, explicit_flags = scan_result

        # Phase 3: Validate
        all_errors = scan_errors.dup

        # Flag validation
        validator = FlagValidator.new(active_flags, active_groups)
        all_errors.concat(validator.validate(parsed_flags, duplicate_flags))

        # Positional resolution
        resolver = PositionalResolver.new(active_args)
        begin
          parsed_args = resolver.resolve(positional_tokens, parsed_flags)
        rescue ParseErrors => e
          all_errors.concat(e.errors)
          parsed_args = {}
        end

        # Apply context to all errors
        all_errors = all_errors.map do |e|
          ParseError.new(
            error_type: e.error_type,
            message: e.message,
            suggestion: e.suggestion,
            context: e.context.empty? ? command_path : e.context
          )
        end

        raise ParseErrors.new(all_errors) unless all_errors.empty?

        # Fill in defaults for absent flags
        finalized_flags = finalize_flags(parsed_flags, active_flags)

        ParseResult.new(
          program: program,
          command_path: command_path,
          flags: finalized_flags,
          arguments: parsed_args,
          explicit_flags: explicit_flags.to_a
        )
      end

      private

      # ---------------------------------------------------------------------------
      # Phase 1: Routing (Directed Graph traversal)
      # ---------------------------------------------------------------------------
      #
      # Walk the tokens from left to right. For each token:
      #   - If it starts with "-", skip it (and possibly the next token if it
      #     takes a value). Flags belong to Phase 2.
      #   - If it matches a subcommand name/alias at the current graph node,
      #     follow the edge and append to command_path.
      #   - Otherwise, stop routing — this token begins the flag/arg section.
      #
      # Returns:
      #   [command_path, current_node_id, tokens, consumed_indices]
      # Where consumed_indices is the set of token indices that were "routed"
      # (i.e., consumed as subcommand names, not to be re-scanned in Phase 2).

      def phase1_routing(tokens, program)
        command_path = [program]
        current_id = "__root__"
        consumed_indices = Set.new
        parsing_mode = @spec["parsing_mode"] || "gnu"

        # For traditional mode: track whether we've seen argv[1]
        first_token_done = false

        i = 0
        while i < tokens.length
          token = tokens[i]

          # "--" stops routing
          break if token == "--"

          if token.start_with?("-") || token == "-"
            # Skip this flag token. Determine if it takes a value.
            # We do a quick check: if it's a long flag without "=", and
            # the corresponding flag is non-boolean, skip one more token.
            flag = find_flag_for_routing_skip(token, command_path)
            i += 1
            i += 1 if flag && !%w[boolean count].include?(flag["type"]) && !flag["default_when_present"] && !token.include?("=") && !token.match?(/\A-[^-]/) && i < tokens.length
            next
          end

          # Traditional mode: first non-flag token that's not a subcommand
          # is treated as stacked flags without a dash prefix
          if !first_token_done && parsing_mode == "traditional"
            first_token_done = true
            # Check if it matches a subcommand first
            successors = @g_cmd.successors(current_id)
            matched_cmd = find_command_match(token, successors, command_path)
            if matched_cmd
              command_path << matched_cmd["name"]
              current_id = matched_cmd["id"]
              consumed_indices.add(i)
              i += 1
            else
              # Treat as stacked flags — Phase 2 will handle the rewriting
              # We just stop routing here
              break
            end
            next
          end

          first_token_done = true

          # Try to match as a subcommand
          successors = @g_cmd.successors(current_id)
          matched_cmd = find_command_match(token, successors, command_path)

          if matched_cmd
            command_path << matched_cmd["name"]
            current_id = matched_cmd["id"]
            consumed_indices.add(i)
            i += 1
          else
            # No matching subcommand — this is the first positional token
            # Check if this looks like an unknown command (subcommand_first mode)
            if parsing_mode == "subcommand_first" && !successors.empty?
              # In subcommand_first mode, we expect a command here — it's an error
              # but we don't raise it here; Phase 2 will produce the error
            end
            break
          end
        end

        [command_path, current_id, tokens, consumed_indices]
      end

      # ---------------------------------------------------------------------------
      # Phase 2: Scanning (Modal State Machine)
      # ---------------------------------------------------------------------------
      #
      # The ModalStateMachine has three modes:
      #
      #   "scanning"      — normal operation; classify each token
      #   "flag_value"    — previous token was a non-boolean flag; this token is its value
      #   "end_of_flags"  — after "--"; all remaining tokens are positional
      #
      # Mode transitions:
      #   scanning    + non-boolean flag seen  → flag_value
      #   scanning    + "--" token seen        → end_of_flags
      #   scanning    + posix positional seen  → end_of_flags
      #   flag_value  + any token              → scanning
      #   end_of_flags (no outgoing transitions — terminal mode)

      def phase2_scanning(tokens, consumed_indices, active_flags, command_path)
        # Build the Modal State Machine
        msm = build_modal_state_machine

        # Build the token classifier for the active flag set
        classifier = TokenClassifier.new(active_flags)

        parsed_flags = {}
        positional_tokens = []
        scan_errors = []
        duplicate_flags = []
        explicit_flags = Set.new
        pending_flag = nil
        pending_flag_optional = false
        parsing_mode = @spec["parsing_mode"] || "gnu"

        # Build quick lookup maps
        flag_by_short = {}
        flag_by_long = {}
        flag_by_sdl = {}
        active_flags.each do |f|
          flag_by_short[f["short"]] = f if f["short"]
          flag_by_long[f["long"]] = f if f["long"]
          flag_by_sdl[f["single_dash_long"]] = f if f["single_dash_long"]
        end

        # Traditional mode: track whether we need to rewrite argv[1]
        traditional_first = (parsing_mode == "traditional")
        first_non_cmd_seen = false

        tokens.each_with_index do |token, i|
          # Skip tokens consumed during routing (subcommand names)
          next if consumed_indices.include?(i)

          # Traditional mode: first non-consumed token that isn't a flag
          if traditional_first && !first_non_cmd_seen && !token.start_with?("-") && token != "--"
            first_non_cmd_seen = true
            # Treat as stacked short flags without a dash prefix
            fake_token = "-#{token}"
            classified = classifier.classify(fake_token)
            if classified[:type] == :stacked_flags || classified[:type] == :short_flag
              result = process_classified(classified, parsed_flags, positional_tokens, scan_errors,
                duplicate_flags, explicit_flags, msm, parsing_mode, command_path, nil)
              return result if result.is_a?(HelpResult) || result.is_a?(VersionResult)
              if result.is_a?(Hash) && result[:pending_flag]
                pending_flag = result[:pending_flag]
                pending_flag_optional = result[:optional_value]
              end
              next
            end
            # If that didn't work, fall through to normal classification
          end
          first_non_cmd_seen = true

          case msm.current_mode
          when "flag_value"
            # ---------------------------------------------------------------------------
            # default_when_present disambiguation (v1.1)
            # ---------------------------------------------------------------------------
            #
            # When we're waiting for a value for an enum flag that has
            # default_when_present, we first check whether the next token
            # is a valid enum value:
            #
            #   --color auto      → "auto" is in enum_values, consume it
            #   --color somefile   → "somefile" is NOT in enum_values,
            #                        use default_when_present, leave token unconsumed
            #   --color --verbose  → starts with "-", use default_when_present
            #
            # This matches GNU coreutils behavior for --color[=WHEN].
            if pending_flag_optional && pending_flag["default_when_present"]
              enum_values = pending_flag["enum_values"] || []
              if token.start_with?("-") || !enum_values.include?(token)
                # Token is not a valid enum value — use default_when_present
                record_flag(pending_flag, pending_flag["default_when_present"], parsed_flags, duplicate_flags)
                explicit_flags.add(pending_flag["id"])
                pending_flag = nil
                pending_flag_optional = false
                msm.switch_mode("got_value")
                # Re-process this token in scanning mode — it's not consumed
                classified = classifier.classify(token)
                result = process_classified(classified, parsed_flags, positional_tokens, scan_errors,
                  duplicate_flags, explicit_flags, msm, parsing_mode, command_path, pending_flag)
                return result if result.is_a?(HelpResult) || result.is_a?(VersionResult)
                if result.is_a?(Hash) && result[:pending_flag]
                  pending_flag = result[:pending_flag]
                  pending_flag_optional = result[:optional_value]
                end
                next
              end
            end

            # This entire token is the value for the pending flag
            value = coerce_flag_value(token, pending_flag, scan_errors)
            if pending_flag["repeatable"]
              parsed_flags[pending_flag["id"]] ||= []
              parsed_flags[pending_flag["id"]] << value
            else
              if parsed_flags.key?(pending_flag["id"])
                duplicate_flags << pending_flag["id"]
              end
              parsed_flags[pending_flag["id"]] = value
            end
            explicit_flags.add(pending_flag["id"])
            pending_flag = nil
            pending_flag_optional = false
            msm.switch_mode("got_value")

          when "end_of_flags"
            # After "--": everything is positional, no classification needed
            positional_tokens << token

          when "scanning"
            classified = classifier.classify(token)
            result = process_classified(classified, parsed_flags, positional_tokens, scan_errors,
              duplicate_flags, explicit_flags, msm, parsing_mode, command_path, pending_flag)

            # Help/version short-circuits: return immediately
            return result if result.is_a?(HelpResult) || result.is_a?(VersionResult)

            # Check if we need to enter flag_value mode
            if result.is_a?(Hash) && result[:pending_flag]
              pending_flag = result[:pending_flag]
              pending_flag_optional = result[:optional_value]
            end
          end
        end

        # If we're still in flag_value mode when tokens run out, either:
        #   - The flag has default_when_present → use that value (no error)
        #   - The flag requires a value → error
        if msm.current_mode == "flag_value" && pending_flag
          if pending_flag_optional && pending_flag["default_when_present"]
            record_flag(pending_flag, pending_flag["default_when_present"], parsed_flags, duplicate_flags)
            explicit_flags.add(pending_flag["id"])
          else
            scan_errors << ParseError.new(
              error_type: "missing_required_argument",
              message: "#{flag_display_name(pending_flag)} requires a value but none was given",
              suggestion: nil,
              context: command_path
            )
          end
        end

        [parsed_flags, positional_tokens, scan_errors, duplicate_flags, explicit_flags]
      end

      # Process a single classified token, updating state in-place.
      # Returns { pending_flag: flag } if we need to enter flag_value mode,
      # or a HelpResult/VersionResult for short-circuits.
      # ---------------------------------------------------------------------------
      # Process a single classified token
      # ---------------------------------------------------------------------------
      #
      # This method dispatches on the token type and updates the parsing state.
      # It handles all flag types (long, short, stacked, single-dash-long),
      # positional arguments, end-of-flags, and unknown flags.
      #
      # v1.1 additions:
      #   - Count type: like boolean, consumes no value. Each occurrence increments
      #     the counter. In stacked flags, each character increments independently.
      #   - default_when_present: enum flags with this field can appear without a
      #     value. When they do, the default_when_present value is used instead of
      #     entering flag_value mode.
      #   - explicit_flags: every flag consumed from argv gets its ID added to the
      #     explicit_flags set, so callers can distinguish user-provided values from
      #     parser-filled defaults.

      def process_classified(classified, parsed_flags, positional_tokens, scan_errors, # rubocop:disable Metrics/ParameterLists
        duplicate_flags, explicit_flags, msm, parsing_mode, command_path, _pending_flag)
        case classified[:type]
        when :end_of_flags
          msm.switch_mode("see_double_dash")
          nil

        when :long_flag
          flag = classified[:flag]
          return handle_help_version(flag, command_path) if builtin_flag?(flag)
          if flag["type"] == "boolean"
            record_flag(flag, true, parsed_flags, duplicate_flags)
            explicit_flags.add(flag["id"])
          elsif flag["type"] == "count"
            # Count flags consume no value — just increment the counter.
            # Unlike boolean flags which set true, count flags accumulate.
            record_count_flag(flag, parsed_flags)
            explicit_flags.add(flag["id"])
          elsif flag["default_when_present"]
            # Enum flag with default_when_present: use the default value
            # when no value token follows. The caller (phase2_scanning) will
            # handle the disambiguation of whether the next token is a valid
            # enum value — but for bare --flag at end-of-argv or followed by
            # another flag, we use default_when_present.
            # We enter a special "need_value_optional" mode handled in phase2.
            msm.switch_mode("need_value")
            return {pending_flag: flag, optional_value: true}
          else
            msm.switch_mode("need_value")
            return {pending_flag: flag}
          end
          nil

        when :long_flag_with_value
          flag = classified[:flag]
          return handle_help_version(flag, command_path) if builtin_flag?(flag)
          value = coerce_flag_value(classified[:value], flag, scan_errors)
          record_flag(flag, value, parsed_flags, duplicate_flags)
          explicit_flags.add(flag["id"])
          nil

        when :single_dash_long
          flag = classified[:flag]
          return handle_help_version(flag, command_path) if builtin_flag?(flag)
          if flag["type"] == "boolean"
            record_flag(flag, true, parsed_flags, duplicate_flags)
            explicit_flags.add(flag["id"])
          elsif flag["type"] == "count"
            record_count_flag(flag, parsed_flags)
            explicit_flags.add(flag["id"])
          elsif flag["default_when_present"]
            msm.switch_mode("need_value")
            return {pending_flag: flag, optional_value: true}
          else
            msm.switch_mode("need_value")
            return {pending_flag: flag}
          end
          nil

        when :short_flag
          flag = classified[:flag]
          return handle_help_version(flag, command_path) if builtin_flag?(flag)
          if flag["type"] == "boolean"
            record_flag(flag, true, parsed_flags, duplicate_flags)
            explicit_flags.add(flag["id"])
          elsif flag["type"] == "count"
            record_count_flag(flag, parsed_flags)
            explicit_flags.add(flag["id"])
          elsif flag["default_when_present"]
            msm.switch_mode("need_value")
            return {pending_flag: flag, optional_value: true}
          else
            msm.switch_mode("need_value")
            return {pending_flag: flag}
          end
          nil

        when :short_flag_with_value
          flag = classified[:flag]
          return handle_help_version(flag, command_path) if builtin_flag?(flag)
          value = coerce_flag_value(classified[:value], flag, scan_errors)
          record_flag(flag, value, parsed_flags, duplicate_flags)
          explicit_flags.add(flag["id"])
          nil

        when :stacked_flags
          flags = classified[:flags]
          last_value = classified[:last_value]

          flags.each_with_index do |flag, idx|
            return handle_help_version(flag, command_path) if builtin_flag?(flag)
            is_last = (idx == flags.length - 1)
            if is_last && last_value
              value = coerce_flag_value(last_value, flag, scan_errors)
              record_flag(flag, value, parsed_flags, duplicate_flags)
              explicit_flags.add(flag["id"])
            elsif flag["type"] == "count"
              # Count flags in a stack: each character increments the counter.
              # "-vvv" means v appears 3 times in the stack, so count = 3.
              record_count_flag(flag, parsed_flags)
              explicit_flags.add(flag["id"])
            elsif is_last && !%w[boolean count].include?(flag["type"])
              if flag["default_when_present"]
                # Last flag in stack is an enum with default_when_present:
                # use default_when_present since there's no inline value
                record_flag(flag, flag["default_when_present"], parsed_flags, duplicate_flags)
                explicit_flags.add(flag["id"])
              else
                # Last flag in stack is non-boolean with no inline value
                # → next token is the value
                msm.switch_mode("need_value")
                return {pending_flag: flag}
              end
            else
              record_flag(flag, true, parsed_flags, duplicate_flags)
              explicit_flags.add(flag["id"])
            end
          end
          nil

        when :positional
          if parsing_mode == "posix" && msm.current_mode == "scanning"
            # POSIX mode: first positional ends flag scanning
            msm.switch_mode("see_double_dash")
          end
          positional_tokens << classified[:value]
          nil

        when :unknown_flag
          token = classified[:token]
          suggestion = fuzzy_suggest_flag(token)
          scan_errors << ParseError.new(
            error_type: "unknown_flag",
            message: "Unknown flag #{token.inspect}",
            suggestion: suggestion ? "Did you mean #{suggestion}?" : nil,
            context: command_path
          )
          nil
        end
      end

      # ---------------------------------------------------------------------------
      # Flag recording helpers
      # ---------------------------------------------------------------------------

      def record_flag(flag, value, parsed_flags, duplicate_flags)
        fid = flag["id"]
        if flag["repeatable"]
          parsed_flags[fid] ||= []
          parsed_flags[fid] << value
        else
          duplicate_flags << fid if parsed_flags.key?(fid)
          parsed_flags[fid] = value
        end
      end

      # ---------------------------------------------------------------------------
      # Count flag recording (v1.1)
      # ---------------------------------------------------------------------------
      #
      # Count flags are a special case: they don't take a value token, and each
      # occurrence increments a counter by 1. In stacked short flags like "-vvv",
      # each 'v' character calls this method once, producing a count of 3.
      #
      # Unlike regular flags, count flags are inherently repeatable — duplicates
      # are expected and desired. We never add them to duplicate_flags.

      def record_count_flag(flag, parsed_flags)
        fid = flag["id"]
        parsed_flags[fid] = (parsed_flags[fid] || 0) + 1
      end

      # ---------------------------------------------------------------------------
      # int64 range constants (v1.1)
      # ---------------------------------------------------------------------------
      #
      # Ruby's Integer is arbitrary-precision, so it can represent values far
      # beyond what a 64-bit signed integer can hold. For cross-language
      # consistency, we reject values outside the int64 range.
      #
      # The range is: −2^63 to 2^63 − 1
      #   min = -9,223,372,036,854,775,808
      #   max =  9,223,372,036,854,775,807
      INT64_MIN = -(2**63)
      INT64_MAX = (2**63) - 1

      def coerce_flag_value(str, flag, errors)
        type = flag["type"]
        case type
        when "boolean"
          str == "true"
        when "integer"
          begin
            val = Integer(str)
            # v1.1: Range check — reject values outside 64-bit signed integer range.
            # Ruby happily handles arbitrary-precision integers, but for cross-language
            # consistency (Go uses int64, Rust uses i64, etc.) we enforce the same limit.
            if val < INT64_MIN || val > INT64_MAX
              errors << ParseError.new(
                error_type: "invalid_value",
                message: "Integer value #{str.inspect} is out of range (must fit in 64 bits)",
                suggestion: nil,
                context: []
              )
              return nil
            end
            val
          rescue ArgumentError
            errors << ParseError.new(
              error_type: "invalid_value",
              message: "Invalid integer for #{flag_display_name(flag)}: #{str.inspect}",
              suggestion: nil,
              context: []
            )
            nil
          end
        when "float"
          begin
            Float(str)
          rescue ArgumentError
            errors << ParseError.new(
              error_type: "invalid_value",
              message: "Invalid float for #{flag_display_name(flag)}: #{str.inspect}",
              suggestion: nil,
              context: []
            )
            nil
          end
        when "enum"
          valid = flag["enum_values"] || []
          unless valid.include?(str)
            errors << ParseError.new(
              error_type: "invalid_enum_value",
              message: "Invalid value #{str.inspect} for #{flag_display_name(flag)}. " \
                       "Must be one of: #{valid.join(", ")}",
              suggestion: nil,
              context: []
            )
            return nil
          end
          str
        else
          # string, path, file, directory — leave as string at flag level
          str
        end
      end

      # ---------------------------------------------------------------------------
      # Help / Version detection
      # ---------------------------------------------------------------------------

      def handle_help_version(flag, command_path)
        if flag["id"] == "__help__" || (flag["short"] == "h" && builtin_flag?(flag)) ||
            (flag["long"] == "help" && builtin_flag?(flag))
          generator = HelpGenerator.new(@spec, command_path)
          return HelpResult.new(text: generator.generate, command_path: command_path)
        end

        if flag["id"] == "__version__" || (flag["long"] == "version" && builtin_flag?(flag))
          return VersionResult.new(version: @spec["version"] || "0.0.0")
        end

        nil
      end

      def builtin_flag?(flag)
        flag["id"] == "__help__" || flag["id"] == "__version__"
      end

      # ---------------------------------------------------------------------------
      # Active flag set construction
      # ---------------------------------------------------------------------------
      #
      # The active flags for a given command path are:
      #   1. global_flags (if inherit_global_flags is true for this command)
      #   2. flags defined at each level of the command path
      #   3. builtin flags (--help, --version) if enabled

      def build_active_flags(command_path)
        # We build user flags first, then add builtins only when they don't
        # clash with user-defined flags. This ensures user-defined flags always
        # win over the injected --help/-h and --version flags.
        user_flags = []

        # Global flags
        user_flags.concat(@spec["global_flags"] || [])

        # Root-level flags
        user_flags.concat(@spec["flags"] || [])

        # Command-level flags (traverse the path)
        current = @spec
        command_path[1..].each do |cmd_name|
          cmds = current["commands"] || []
          cmd = cmds.find { |c| c["name"] == cmd_name || (c["aliases"] || []).include?(cmd_name) }
          if cmd
            user_flags.concat(cmd["flags"] || [])
            current = cmd
          end
        end

        # Collect which short/long names are already claimed by user flags.
        # Builtin flags are only added when their names are not already in use.
        used_short = user_flags.map { |f| f["short"] }.compact.to_set
        used_long = user_flags.map { |f| f["long"] }.compact.to_set

        builtin_flags = []
        bf = @spec["builtin_flags"] || {}

        if bf["help"] != false
          help_flag = {
            "id" => "__help__",
            "short" => "h",
            "long" => "help",
            "description" => "Show this help message and exit.",
            "type" => "boolean",
            "required" => false,
            "repeatable" => false,
            "conflicts_with" => [],
            "requires" => [],
            "required_unless" => [],
            "enum_values" => []
          }
          # Only claim -h if not already used; always claim --help unless used.
          help_flag.delete("short") if used_short.include?("h")
          builtin_flags << help_flag unless used_long.include?("help")
        end

        if bf["version"] != false && @spec["version"]
          unless used_long.include?("version")
            builtin_flags << {
              "id" => "__version__",
              "long" => "version",
              "description" => "Show version and exit.",
              "type" => "boolean",
              "required" => false,
              "repeatable" => false,
              "conflicts_with" => [],
              "requires" => [],
              "required_unless" => [],
              "enum_values" => []
            }
          end
        end

        user_flags + builtin_flags
      end

      # ---------------------------------------------------------------------------
      # Default finalization
      # ---------------------------------------------------------------------------
      #
      # After validation, fill in defaults for flags that were not provided.
      # Boolean flags absent → false. Other flags absent → nil (or default value).

      # ---------------------------------------------------------------------------
      # Default finalization (v1.1 updated)
      # ---------------------------------------------------------------------------
      #
      # After validation, fill in defaults for flags that were not provided:
      #   - boolean absent → false (or specified default)
      #   - count absent → 0 (or specified default)
      #   - other absent → nil (or specified default)

      def finalize_flags(parsed_flags, active_flags)
        result = {}
        active_flags.each do |f|
          next if f["id"].start_with?("__") # Skip builtins in output

          result[f["id"]] = if parsed_flags.key?(f["id"])
            parsed_flags[f["id"]]
          elsif f["type"] == "boolean"
            f["default"].nil? ? false : f["default"]
          elsif f["type"] == "count"
            # Count flags default to 0 when absent (like boolean defaults to false).
            f["default"].nil? ? 0 : f["default"]
          else
            f["default"]
          end
        end
        result
      end

      # ---------------------------------------------------------------------------
      # Command node definition resolution
      # ---------------------------------------------------------------------------

      def resolve_node_def(command_path)
        current = @spec
        command_path[1..].each do |cmd_name|
          cmds = current["commands"] || []
          cmd = cmds.find { |c| c["name"] == cmd_name || (c["aliases"] || []).include?(cmd_name) }
          current = cmd if cmd
        end
        current
      end

      # ---------------------------------------------------------------------------
      # Routing graph construction
      # ---------------------------------------------------------------------------
      #
      # Build G_cmd: nodes are "__root__" plus each command's id.
      # Edges: parent_id → child_id for each subcommand relationship.
      # We also store a lookup from node_id → command_def for routing.

      def build_routing_graph
        g = CodingAdventures::DirectedGraph::Graph.new
        g.add_node("__root__")

        @command_lookup = {"__root__" => @spec}

        add_commands_to_graph(g, @spec["commands"] || [], "__root__")
        g
      end

      def add_commands_to_graph(g, commands, parent_id)
        commands.each do |cmd|
          g.add_node(cmd["id"])
          g.add_edge(parent_id, cmd["id"])
          @command_lookup[cmd["id"]] = cmd
          add_commands_to_graph(g, cmd["commands"] || [], cmd["id"])
        end
      end

      # Find the matching command definition among the successors
      def find_command_match(token, successor_ids, command_path)
        successor_ids.each do |sid|
          cmd = @command_lookup[sid]
          next unless cmd
          if cmd["name"] == token || (cmd["aliases"] || []).include?(token)
            return cmd
          end
        end
        nil
      end

      # Quick flag lookup for routing phase skip logic
      def find_flag_for_routing_skip(token, command_path)
        active = build_active_flags(command_path)
        classifier = TokenClassifier.new(active)
        result = classifier.classify(token)
        case result[:type]
        when :long_flag, :single_dash_long, :short_flag then result[:flag]
        when :stacked_flags then result[:flags].last
        end
      end

      # ---------------------------------------------------------------------------
      # Modal State Machine construction
      # ---------------------------------------------------------------------------
      #
      # States: scanning, flag_value, end_of_flags
      # Modes (DFAs): each mode is a trivial DFA with one state (the mode just
      # acts as a label; the real logic is in the scanner loop above)
      #
      # Mode transitions:
      #   [scanning, need_value]      → flag_value
      #   [scanning, see_double_dash] → end_of_flags
      #   [flag_value, got_value]     → scanning
      #
      # We use a trivial DFA for each mode (one state, one self-loop event)
      # because the actual processing logic lives in the scanner loop, not
      # inside the DFAs. The Modal State Machine here is used purely for
      # mode tracking and transition management.

      def build_modal_state_machine
        trivial_dfa = ->(name) {
          CodingAdventures::StateMachine::DFA.new(
            states: Set["active"],
            alphabet: Set["tick"],
            transitions: {["active", "tick"] => "active"},
            initial: "active",
            accepting: Set["active"]
          )
        }

        CodingAdventures::StateMachine::ModalStateMachine.new(
          modes: {
            "scanning" => trivial_dfa.call("scanning"),
            "flag_value" => trivial_dfa.call("flag_value"),
            "end_of_flags" => trivial_dfa.call("end_of_flags")
          },
          mode_transitions: {
            ["scanning", "need_value"] => "flag_value",
            ["scanning", "see_double_dash"] => "end_of_flags",
            ["flag_value", "got_value"] => "scanning"
          },
          initial_mode: "scanning"
        )
      end

      # ---------------------------------------------------------------------------
      # Fuzzy matching for suggestions
      # ---------------------------------------------------------------------------
      #
      # When the user types an unknown flag, compute Levenshtein distance against
      # all known flags in scope and suggest the closest one (if dist ≤ 2).

      def fuzzy_suggest_flag(token)
        # Strip leading dashes for comparison
        stripped = token.sub(/\A--?/, "")
        best = nil
        best_dist = 3 # only suggest if dist ≤ 2

        active = @spec["flags"] || []
        active.concat(@spec["global_flags"] || [])

        active.each do |f|
          [f["long"], f["short"], f["single_dash_long"]].compact.each do |name|
            d = levenshtein(stripped, name)
            if d < best_dist
              best_dist = d
              best = if f["long"]
                "--#{f["long"]}"
              elsif f["short"]
                "-#{f["short"]}"
              else
                "-#{f["single_dash_long"]}"
              end
            end
          end
        end

        best
      end

      def levenshtein(a, b)
        m, n = a.length, b.length
        return n if m.zero?
        return m if n.zero?

        prev = (0..n).to_a
        (1..m).each do |i|
          curr = [i]
          (1..n).each do |j|
            cost = (a[i - 1] == b[j - 1]) ? 0 : 1
            curr << [curr[j - 1] + 1, prev[j] + 1, prev[j - 1] + cost].min
          end
          prev = curr
        end
        prev[n]
      end

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
