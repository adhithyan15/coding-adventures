# frozen_string_literal: true

require "json"

# ---------------------------------------------------------------------------
# validator.rb — Standalone spec validation without raising exceptions
# ---------------------------------------------------------------------------
#
# The SpecLoader class validates specs by raising SpecError on the first
# problem it encounters. This is perfect for production use — if your spec
# is broken, you want a loud, immediate crash.
#
# But sometimes you need gentler validation:
#
#   - **Linters and editors** want to collect ALL errors, not just the first.
#   - **CI pipelines** want a boolean pass/fail without exception handling.
#   - **Interactive tools** want to show a list of problems to fix.
#
# The `validate_spec` and `validate_spec_string` module methods fill this
# gap. They wrap SpecLoader's validation in a begin/rescue and return a
# simple ValidationResult value object instead of raising.
#
# === ValidationResult ===
#
# A tiny value object with two fields:
#
#   valid?  — true if the spec passed all checks, false otherwise
#   errors  — an array of human-readable error message strings
#
# When valid? is true, errors is always empty.
# When valid? is false, errors contains at least one message.
#
# === Usage ===
#
#   # Validate a file on disk:
#   result = CodingAdventures::CliBuilder.validate_spec("my-tool.json")
#   if result.valid?
#     puts "Spec is valid!"
#   else
#     result.errors.each { |e| puts "ERROR: #{e}" }
#   end
#
#   # Validate a JSON string (useful in tests or editors):
#   json = '{"cli_builder_spec_version": "1.0", ...}'
#   result = CodingAdventures::CliBuilder.validate_spec_string(json)
#   puts result.valid?   # => true or false
#
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # ValidationResult holds the outcome of spec validation.
    #
    # It is a simple value object — immutable after creation. The two fields
    # tell you everything you need to know:
    #
    #   valid?  — did the spec pass all checks?
    #   errors  — what went wrong? (empty array when valid)
    #
    # This is intentionally NOT an exception. It is a data structure that
    # callers can inspect, log, serialize, or display however they like.
    class ValidationResult
      # @return [Boolean] true if the spec passed all validation checks.
      attr_reader :valid

      # @return [Array<String>] human-readable error messages (empty when valid).
      attr_reader :errors

      # Convenience predicate — reads more naturally than `.valid`.
      #
      #   if result.valid?
      #     # proceed
      #   end
      alias_method :valid?, :valid

      # Create a new ValidationResult.
      #
      # @param valid [Boolean] whether the spec is valid.
      # @param errors [Array<String>] error messages (should be empty when valid is true).
      def initialize(valid:, errors: [])
        @valid = valid
        @errors = errors.freeze
      end
    end

    # -------------------------------------------------------------------------
    # Module-level validation methods
    # -------------------------------------------------------------------------

    # Validate a CLI Builder JSON spec file on disk.
    #
    # This is the primary entry point for validation. It reads the file,
    # parses the JSON, and runs all validation checks. Instead of raising
    # SpecError on failure, it returns a ValidationResult.
    #
    # Under the hood, it delegates to SpecLoader — the same validation
    # logic used in production parsing. The only difference is that errors
    # are captured into a result object instead of propagated as exceptions.
    #
    # @param spec_file_path [String] path to the JSON spec file.
    # @return [ValidationResult] the validation outcome.
    def self.validate_spec(spec_file_path)
      SpecLoader.new(spec_file_path).load
      ValidationResult.new(valid: true)
    rescue SpecError => e
      ValidationResult.new(valid: false, errors: [e.message])
    end

    # Validate a CLI Builder spec from a raw JSON string.
    #
    # This is useful when you already have the JSON in memory — for example,
    # in an editor plugin, a test helper, or a web-based spec builder.
    #
    # It writes the string to a temporary file and delegates to validate_spec.
    # The tempfile is cleaned up automatically.
    #
    # @param json_string [String] the JSON spec content.
    # @return [ValidationResult] the validation outcome.
    def self.validate_spec_string(json_string)
      # We use a Tempfile so we can reuse SpecLoader (which expects a file path).
      # This keeps validation logic in one place — no duplication.
      require "tempfile"
      tempfile = Tempfile.new(["cli_spec_validate", ".json"])
      tempfile.write(json_string)
      tempfile.close
      validate_spec(tempfile.path)
    ensure
      tempfile&.unlink
    end
  end
end
