# frozen_string_literal: true

# ---------------------------------------------------------------------------
# positional_resolver.rb — Assign positional tokens to argument slots
# ---------------------------------------------------------------------------
#
# After the scanner collects all positional tokens (those that are not flags),
# we need to assign them to the argument definitions declared in the spec.
# This is the "positional resolver".
#
# === The simple case: no variadic argument ===
#
# If none of the argument definitions is variadic, the assignment is
# one-to-one in order:
#
#   arg_defs: [source, dest]
#   tokens:   ["a.txt", "b.txt"]
#   result:   { "source" => "a.txt", "dest" => "b.txt" }
#
# If there are more tokens than arg_defs, it's a "too many arguments" error.
# If a required arg_def has no corresponding token, it's "missing argument".
#
# === The variadic case: the last-wins algorithm ===
#
# When one argument definition is variadic, we need a smarter algorithm.
# Consider the `cp` command:
#
#   cp a.txt b.txt c.txt /dest/
#
# The spec defines:
#   source  — variadic, required, variadic_min: 1  (the files to copy)
#   dest    — non-variadic, required               (the destination)
#
# With four tokens and a variadic+trailing-required layout, how do we know
# which tokens go to "source" and which to "dest"? The answer is the
# **last-wins algorithm**:
#
#   1. Assign tokens to any leading (before variadic) non-variadic args first.
#   2. Assign tokens from the END to any trailing (after variadic) non-variadic args.
#   3. Give all remaining tokens to the variadic argument.
#
# Worked example for `cp a.txt b.txt c.txt /dest/`:
#
#   arg_defs:      [source(variadic), dest]
#   tokens:        ["a.txt", "b.txt", "c.txt", "/dest/"]
#   variadic_idx:  0  (source is first)
#   leading_defs:  []           (nothing before source)
#   trailing_defs: [dest]       (dest comes after source)
#
#   trailing_start = 4 - 1 = 3
#   dest   = tokens[3] = "/dest/"
#   source = tokens[0..2] = ["a.txt", "b.txt", "c.txt"]
#
# This "consume from the right" approach generalizes to any layout:
#
#   mv <source>... <dest>   → same logic, source gets all but last
#   tar c <archive> <files...> → leading fixed + variadic, no trailing
#
# === Type coercion ===
#
# Each value is coerced to the type declared in the argument definition:
#   integer   → Integer()
#   float     → Float()
#   path      → String (syntactic validity only, no filesystem check)
#   file      → String (must be an existing readable file)
#   directory → String (must be an existing directory)
#   string    → String (non-empty)
#   enum      → String (must be in enum_values)
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Resolves positional tokens to named argument slots per the spec §6.4.1.
    class PositionalResolver
      # Create a resolver for the given argument definitions.
      #
      # @param arg_defs [Array<Hash>] Argument definition hashes from the current scope.
      def initialize(arg_defs)
        @arg_defs = arg_defs
      end

      # Assign tokens to argument slots.
      #
      # @param tokens [Array<String>] The positional tokens collected during scanning.
      # @param parsed_flags [Hash] The flags parsed so far (used for required_unless_flag).
      # @return [Hash] Map from argument id to coerced value (variadic → array).
      # @raise [ParseErrors] If any required argument is missing or other constraint violated.
      def resolve(tokens, parsed_flags = {})
        errors = []
        result = {}

        variadic_idx = @arg_defs.index { |a| a["variadic"] }

        if variadic_idx.nil?
          resolve_no_variadic(tokens, parsed_flags, result, errors)
        else
          resolve_with_variadic(tokens, parsed_flags, variadic_idx, result, errors)
        end

        unless errors.empty?
          raise ParseErrors.new(errors)
        end

        result
      end

      private

      # ---------------------------------------------------------------------------
      # No-variadic resolution: one-to-one assignment in spec order
      # ---------------------------------------------------------------------------

      def resolve_no_variadic(tokens, parsed_flags, result, errors)
        # Check for too many tokens
        if tokens.size > @arg_defs.size
          _extra = tokens.size - @arg_defs.size
          errors << ParseError.new(
            error_type: "too_many_arguments",
            message: "Expected at most #{@arg_defs.size} argument(s), got #{tokens.size}",
            suggestion: nil,
            context: []
          )
          return
        end

        @arg_defs.each_with_index do |arg_def, i|
          if i < tokens.size
            coerced = coerce_value(tokens[i], arg_def, errors)
            result[arg_def["id"]] = coerced
          else
            # Token absent — check if required
            unless arg_def["required"] == false || required_unless_satisfied?(arg_def, parsed_flags)
              errors << ParseError.new(
                error_type: "missing_required_argument",
                message: "Missing required argument: <#{arg_def["name"]}>",
                suggestion: nil,
                context: []
              )
            end
            # Use default or nil
            result[arg_def["id"]] = arg_def["default"]
          end
        end
      end

      # ---------------------------------------------------------------------------
      # Variadic resolution: the last-wins algorithm
      # ---------------------------------------------------------------------------
      #
      # Split arg_defs into three sections:
      #   leading_defs  — arg_defs before the variadic
      #   variadic_def  — the variadic arg itself
      #   trailing_defs — arg_defs after the variadic
      #
      # Then:
      #   1. Assign leading tokens to leading_defs (left-to-right)
      #   2. Assign trailing tokens from the END to trailing_defs (right-to-left assignment)
      #   3. Give the middle slice to the variadic_def

      def resolve_with_variadic(tokens, parsed_flags, variadic_idx, result, errors)
        leading_defs = @arg_defs[0, variadic_idx]
        variadic_def = @arg_defs[variadic_idx]
        trailing_defs = @arg_defs[variadic_idx + 1..]

        n = tokens.size
        n_leading = leading_defs.size
        n_trailing = trailing_defs.size
        _min_needed = n_leading + n_trailing + (variadic_def["variadic_min"] || 1)

        # Check if we have enough tokens for leading + trailing minimums
        if n < n_leading + n_trailing
          # Not even enough for the non-variadic required args
          trailing_defs.each do |td|
            unless td["required"] == false
              errors << ParseError.new(
                error_type: "missing_required_argument",
                message: "Missing required argument: <#{td["name"]}>",
                suggestion: nil,
                context: []
              )
            end
            result[td["id"]] = td["default"]
          end
          if n < n_leading
            leading_defs[n..].each do |ld|
              unless ld["required"] == false
                errors << ParseError.new(
                  error_type: "missing_required_argument",
                  message: "Missing required argument: <#{ld["name"]}>",
                  suggestion: nil,
                  context: []
                )
              end
              result[ld["id"]] = ld["default"]
            end
          end
          # Variadic gets nothing when we don't have enough tokens for fixed args
          result[variadic_def["id"]] = []
          return
        end

        # When all tokens are consumed by leading/trailing defs (none left for the
        # variadic), check whether any required leading def has a required_unless_flag
        # that is NOT satisfied. This catches the case where the user provided a FILE
        # argument but forgot the required PATTERN argument — without this check the
        # resolver would silently consume the FILE token into the PATTERN slot.
        #
        # Example: `grep file.txt` with spec [PATTERN(required_unless=-e), FILE(variadic)]
        #   n=1, n_leading=1, n_trailing=0  → n == n_leading + n_trailing
        #   -e flag not present             → required_unless_flag not satisfied
        #   → report missing_required_argument for PATTERN and bail out
        #
        # We only apply this stricter check when the arg has required_unless_flag,
        # because that flag signals the arg is "conditionally optional" — the user might
        # legitimately skip it by providing the flag. Without required_unless_flag, a
        # token present for that slot is always correct to consume.
        if n == n_leading + n_trailing
          leading_defs.each do |ld|
            next if ld["required"] == false
            next if (ld["required_unless_flag"] || []).empty?
            next if required_unless_satisfied?(ld, parsed_flags)

            errors << ParseError.new(
              error_type: "missing_required_argument",
              message: "Missing required argument: <#{ld["name"]}>",
              suggestion: nil,
              context: []
            )
          end
          return if errors.any?
        end

        # Assign leading tokens
        leading_defs.each_with_index do |ld, i|
          if i < n
            result[ld["id"]] = coerce_value(tokens[i], ld, errors)
          else
            result[ld["id"]] = ld["default"]
          end
        end

        # Assign trailing tokens from the end
        trailing_start = n - n_trailing
        trailing_defs.each_with_index do |td, i|
          tok_idx = trailing_start + i
          if tok_idx < n
            result[td["id"]] = coerce_value(tokens[tok_idx], td, errors)
          else
            unless td["required"] == false
              errors << ParseError.new(
                error_type: "missing_required_argument",
                message: "Missing required argument: <#{td["name"]}>",
                suggestion: nil,
                context: []
              )
            end
            result[td["id"]] = td["default"]
          end
        end

        # Variadic gets everything between leading and trailing
        variadic_tokens = tokens[n_leading, trailing_start - n_leading]
        variadic_tokens ||= []
        count = variadic_tokens.size

        vmin = variadic_def["variadic_min"] || (variadic_def["required"] ? 1 : 0)
        vmax = variadic_def["variadic_max"]

        if count < vmin
          errors << ParseError.new(
            error_type: "too_few_arguments",
            message: "Expected at least #{vmin} <#{variadic_def["name"]}>, got #{count}",
            suggestion: nil,
            context: []
          )
        elsif vmax && count > vmax
          errors << ParseError.new(
            error_type: "too_many_arguments",
            message: "Expected at most #{vmax} <#{variadic_def["name"]}>, got #{count}",
            suggestion: nil,
            context: []
          )
        end

        result[variadic_def["id"]] = variadic_tokens.map { |t| coerce_value(t, variadic_def, errors) }
      end

      # ---------------------------------------------------------------------------
      # Type coercion
      # ---------------------------------------------------------------------------
      #
      # Coerce a string value to the Ruby type declared in the arg_def.
      # On failure, appends a ParseError and returns nil.

      def coerce_value(str, item_def, errors)
        type = item_def["type"]
        case type
        when "boolean"
          # Boolean args are unusual but supported
          str == "true"
        when "integer"
          begin
            Integer(str)
          rescue ArgumentError
            errors << ParseError.new(
              error_type: "invalid_value",
              message: "Invalid integer for <#{item_def["name"]}>: #{str.inspect}",
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
              message: "Invalid float for <#{item_def["name"]}>: #{str.inspect}",
              suggestion: nil,
              context: []
            )
            nil
          end
        when "path"
          # Path: syntactically any string, no filesystem check
          str
        when "file"
          # File: must exist and be readable
          unless File.file?(str) && File.readable?(str)
            errors << ParseError.new(
              error_type: "invalid_value",
              message: "Not a readable file for <#{item_def["name"]}>: #{str.inspect}",
              suggestion: nil,
              context: []
            )
            return nil
          end
          str
        when "directory"
          # Directory: must exist and be a directory
          unless File.directory?(str)
            errors << ParseError.new(
              error_type: "invalid_value",
              message: "Not a directory for <#{item_def["name"]}>: #{str.inspect}",
              suggestion: nil,
              context: []
            )
            return nil
          end
          str
        when "enum"
          valid = item_def["enum_values"] || []
          unless valid.include?(str)
            errors << ParseError.new(
              error_type: "invalid_enum_value",
              message: "Invalid value #{str.inspect} for <#{item_def["name"]}>. " \
                       "Must be one of: #{valid.join(", ")}",
              suggestion: nil,
              context: []
            )
            return nil
          end
          str
        else
          # Default: treat as string
          str
        end
      end

      # Check if required_unless_flag conditions exempt this arg from being required.
      def required_unless_satisfied?(arg_def, parsed_flags)
        exempt_flags = arg_def["required_unless_flag"] || []
        exempt_flags.any? { |fid| parsed_flags.key?(fid) }
      end
    end
  end
end
