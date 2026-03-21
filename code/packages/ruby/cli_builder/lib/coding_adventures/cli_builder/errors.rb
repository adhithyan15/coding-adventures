# frozen_string_literal: true

# ---------------------------------------------------------------------------
# errors.rb — Error hierarchy for CLI Builder
# ---------------------------------------------------------------------------
#
# CLI Builder distinguishes two kinds of failures:
#
# 1. **Spec errors** — the JSON specification file itself is invalid.
#    These are programmer errors, caught at library load time before any
#    parsing begins. A spec error means the developer made a mistake when
#    writing their CLI spec: duplicate IDs, a cycle in flag dependencies,
#    an enum flag with no values, etc.
#
# 2. **Parse errors** — the user's argv does not conform to the spec.
#    These are user errors, caught during parsing. A parse error means the
#    user typed something wrong: an unknown flag, a missing required argument,
#    two conflicting flags used together, etc.
#
# This two-level design lets callers pattern-match on the exception class to
# determine whether to log a bug report (SpecError) or show usage help
# (ParseErrors).
#
# === The ParseError Struct ===
#
# ParseError is a value object (a Struct) rather than an Exception. The
# parser collects ALL parse errors into an array — so the user sees every
# problem at once — and then raises a single ParseErrors exception wrapping
# that array. This "aggregate errors" pattern is far more usable than
# "fail fast on first error": if the user forgot three required flags, they
# should see all three error messages in one run, not one per run.
#
# Each ParseError carries four fields:
#
#   error_type  — snake_case string, machine-readable (e.g. "missing_required_flag")
#   message     — human-readable sentence (e.g. "--output is required for 'ls'")
#   suggestion  — optional corrective hint (e.g. "Did you mean '--output'?")
#   context     — the command_path at the point of error (e.g. ["git", "commit"])
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Base class for all CLI Builder errors.
    #
    # Catching CliBuilderError catches both spec errors and parse errors.
    # Callers that want finer-grained control should catch SpecError and
    # ParseErrors separately.
    class CliBuilderError < StandardError; end

    # Raised when the JSON specification file is structurally invalid.
    #
    # This is a programmer error — the developer who wrote the spec made a
    # mistake. Examples:
    #
    #   - cli_builder_spec_version is missing or not "1.0"
    #   - A flag has no short, long, or single_dash_long field
    #   - Two flags in the same scope share the same id
    #   - A flag's requires list references a nonexistent flag id
    #   - A cycle exists in the flag dependency graph (A requires B requires A)
    #   - An enum flag has no enum_values
    #
    # SpecErrors are raised by SpecLoader#load before any parsing begins.
    # They should be treated as bugs in the CLI spec, not as user input errors.
    class SpecError < CliBuilderError; end

    # A single parse error encountered during argv processing.
    #
    # ParseError is a Struct (value object), not an Exception. The parser
    # collects all ParseErrors into an array and then raises a single
    # ParseErrors exception. This lets the user see all problems at once.
    #
    # Fields:
    #   error_type  — snake_case identifier (see spec §8.2 for full list)
    #   message     — human-readable description of the problem
    #   suggestion  — optional corrective hint or fuzzy match ("Did you mean X?")
    #   context     — command_path at the point of the error
    #
    # Example:
    #   ParseError.new(
    #     error_type: "missing_required_flag",
    #     message:    "--message is required for 'git commit'",
    #     suggestion: nil,
    #     context:    ["git", "commit"]
    #   )
    ParseError = Struct.new(:error_type, :message, :suggestion, :context, keyword_init: true)

    # Raised when one or more ParseErrors accumulate during parsing.
    #
    # The parser collects all errors rather than stopping at the first one —
    # the "aggregate errors" pattern. When parsing is complete, if any errors
    # were collected, this exception is raised with the full list.
    #
    # Usage:
    #   begin
    #     result = parser.parse
    #   rescue CodingAdventures::CliBuilder::ParseErrors => e
    #     e.errors.each { |err| warn err.message }
    #     exit 1
    #   end
    class ParseErrors < CliBuilderError
      # The list of individual parse errors collected during parsing.
      #
      # @return [Array<ParseError>]
      attr_reader :errors

      # Create a new ParseErrors exception wrapping an array of ParseError objects.
      #
      # @param errors [Array<ParseError>] The collected parse errors.
      def initialize(errors)
        @errors = errors.freeze
        messages = errors.map(&:message).join("; ")
        super(messages)
      end
    end
  end
end
