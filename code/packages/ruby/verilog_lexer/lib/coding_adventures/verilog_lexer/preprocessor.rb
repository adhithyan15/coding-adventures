# frozen_string_literal: true

# ================================================================
# Verilog Preprocessor -- Handles Compiler Directives Before Lexing
# ================================================================
#
# Verilog uses backtick-prefixed compiler directives that must be
# resolved before tokenization. This preprocessor handles:
#
#   `define NAME value       -- define a text-substitution macro
#   `define NAME(a,b) expr   -- define a parameterized macro
#   `undef NAME              -- remove a macro definition
#   `ifdef NAME              -- conditional: include if macro defined
#   `ifndef NAME             -- conditional: include if macro NOT defined
#   `else                    -- else branch of conditional
#   `endif                   -- end conditional block
#   `include "file.v"        -- file inclusion (stubbed -- returns comment)
#   `timescale 1ns/1ps       -- time unit spec (stripped entirely)
#
# How it works:
# ----------------------------------------------------------------
# The preprocessor makes a single pass through the source, line by
# line. It maintains two pieces of state:
#
# 1. A macro table (Hash) mapping macro names to their definitions.
#    Simple macros map to a string value. Parameterized macros map
#    to a struct holding parameter names and a body template.
#
# 2. A conditional stack (Array) tracking nested `ifdef/`ifndef
#    blocks. Each entry records whether the current branch is active
#    (should emit output) or suppressed (should skip lines).
#
# After directive processing, remaining lines have macro references
# (`NAME or `NAME(args)) expanded by string substitution.
#
# Example:
#   `define WIDTH 8
#   `define MAX(a,b) ((a) > (b) ? (a) : (b))
#   wire [`WIDTH-1:0] data;           => wire [8-1:0] data;
#   assign out = `MAX(x, y);          => assign out = ((x) > (y) ? (x) : (y));
# ================================================================

module CodingAdventures
  module VerilogLexer
    module Preprocessor
      # ----------------------------------------------------------
      # ParameterizedMacro -- A macro with formal parameters
      # ----------------------------------------------------------
      #
      # When we encounter `define ADD(a,b) ((a) + (b)), we store:
      #   params = ["a", "b"]
      #   body   = "((a) + (b))"
      #
      # On expansion of `ADD(x, y), we substitute each param with
      # the corresponding argument: "((x) + (y))".

      ParameterizedMacro = Struct.new(:params, :body)

      # ----------------------------------------------------------
      # ConditionalFrame -- Tracks one level of `ifdef/`ifndef nesting
      # ----------------------------------------------------------
      #
      # Fields:
      #   active       -- is the CURRENT branch emitting output?
      #   parent_active -- was the PARENT scope active?
      #   seen_true    -- has ANY branch in this if/else chain been true?
      #
      # A branch emits output only when both active AND parent_active are true.
      # The seen_true flag prevents `else from activating when `ifdef was true.

      ConditionalFrame = Struct.new(:active, :parent_active, :seen_true)

      # ----------------------------------------------------------
      # process -- Main entry point
      # ----------------------------------------------------------
      #
      # Takes a string of Verilog source code and returns a new string
      # with all preprocessor directives resolved.
      #
      # @param source [String] raw Verilog source with directives
      # @return [String] preprocessed source ready for tokenization

      def self.process(source)
        macros = {}
        cond_stack = []
        output_lines = []

        source.each_line do |raw_line|
          line = raw_line.chomp

          # Determine if we are currently in an active (emitting) scope.
          # If the conditional stack is empty, we are at top level -- always active.
          # Otherwise, the current scope is active only if both the frame's own
          # `active` flag AND its parent's `parent_active` flag are true.
          currently_active = currently_active?(cond_stack)

          # ----- Directive dispatch -----
          # We check for directives even in inactive scopes because `ifdef/`else/`endif
          # must still be tracked to maintain correct nesting.

          if line =~ /^\s*`define\s+/
            handle_define(line, macros) if currently_active
          elsif line =~ /^\s*`undef\s+(\w+)/
            macros.delete(Regexp.last_match(1)) if currently_active
          elsif line =~ /^\s*`ifdef\s+(\w+)/
            handle_ifdef(Regexp.last_match(1), macros, cond_stack, currently_active)
          elsif line =~ /^\s*`ifndef\s+(\w+)/
            handle_ifndef(Regexp.last_match(1), macros, cond_stack, currently_active)
          elsif line.strip == "`else"
            handle_else(cond_stack)
          elsif line.strip == "`endif"
            cond_stack.pop
          elsif line =~ /^\s*`include\s+/
            # Stubbed: replace `include with a comment indicating inclusion point.
            output_lines << "// [preprocessor] include stubbed: #{line.strip}" if currently_active
          elsif line =~ /^\s*`timescale\s+/
            # `timescale directives are stripped entirely -- they have no effect
            # on the token stream and are only meaningful for simulation timing.
            next
          elsif currently_active
            # Regular line (not a directive) in an active scope.
            # Expand any macro references before emitting.
            output_lines << expand_macros(line, macros)
          end
          # Lines in inactive scopes (inside a false `ifdef branch) are silently dropped.
        end

        output_lines.join("\n")
      end

      # ----------------------------------------------------------
      # Private helpers
      # ----------------------------------------------------------

      # Is the current scope active (should we emit output)?
      # Top-level (empty stack) is always active. Otherwise, both the
      # frame's own flag and its parent scope must be active.
      def self.currently_active?(cond_stack)
        return true if cond_stack.empty?

        frame = cond_stack.last
        frame.active && frame.parent_active
      end

      # Parse a `define directive and add the macro to the table.
      #
      # Two forms:
      #   `define WIDTH 8                    -- simple macro
      #   `define MAX(a, b) ((a) > (b) ...)  -- parameterized macro
      #
      # The regex captures:
      #   name   -- the macro name (e.g., "WIDTH" or "MAX")
      #   rest   -- everything after the name (e.g., "(a, b) ((a)...)" or " 8")
      def self.handle_define(line, macros)
        if line =~ /^\s*`define\s+(\w+)\(([^)]*)\)\s+(.*)/
          # Parameterized macro: `define NAME(params) body
          name = Regexp.last_match(1)
          params = Regexp.last_match(2).split(",").map(&:strip)
          body = Regexp.last_match(3)
          macros[name] = ParameterizedMacro.new(params, body)
        elsif line =~ /^\s*`define\s+(\w+)\s+(.*)/
          # Simple macro: `define NAME value
          macros[Regexp.last_match(1)] = Regexp.last_match(2).strip
        elsif line =~ /^\s*`define\s+(\w+)\s*$/
          # Flag macro (no value): `define NAME
          # Used for `ifdef checks. Expands to empty string.
          macros[Regexp.last_match(1)] = ""
        end
      end

      # Push a new conditional frame for `ifdef.
      # The macro is "defined" if it exists in the macro table.
      def self.handle_ifdef(name, macros, cond_stack, parent_active)
        is_defined = macros.key?(name)
        cond_stack.push(ConditionalFrame.new(is_defined, parent_active, is_defined))
      end

      # Push a new conditional frame for `ifndef.
      # Opposite of `ifdef -- active when the macro is NOT defined.
      def self.handle_ifndef(name, macros, cond_stack, parent_active)
        is_not_defined = !macros.key?(name)
        cond_stack.push(ConditionalFrame.new(is_not_defined, parent_active, is_not_defined))
      end

      # Flip the active flag for `else.
      # Only activates if no previous branch was true (seen_true is false).
      def self.handle_else(cond_stack)
        return if cond_stack.empty?

        frame = cond_stack.last
        if frame.seen_true
          # A previous branch was true, so `else is inactive.
          frame.active = false
        else
          # No branch was true yet, so `else activates.
          frame.active = true
          frame.seen_true = true
        end
      end

      # Expand all macro references in a line.
      #
      # Scans for backtick-prefixed identifiers and replaces them:
      #   `WIDTH         => value from macros["WIDTH"]
      #   `MAX(x, y)     => parameterized expansion
      #
      # Parameterized macros use a simple argument parser that handles
      # nested parentheses (so `MAX((a+b), c) works correctly).
      def self.expand_macros(line, macros)
        # First expand parameterized macros (`NAME(args) patterns).
        # This must happen before simple macro expansion so that
        # `MAX(a, b) is expanded as a whole unit rather than treating
        # `MAX as a simple macro reference.
        result = expand_parameterized_macros(line, macros)

        # Simple macros: `NAME (not followed by open paren)
        result = result.gsub(/`(\w+)/) do
          name = Regexp.last_match(1)
          if macros.key?(name) && !macros[name].is_a?(ParameterizedMacro)
            macros[name]
          else
            "`#{name}"
          end
        end

        result
      end

      # Expand parameterized macros in a string.
      # Uses a loop to find `NAME( patterns and replace them with their expansion.
      def self.expand_parameterized_macros(line, macros)
        result = line
        # Keep expanding until no more parameterized macros are found.
        # (Handles nested macro calls, though that's rare in practice.)
        changed = true
        iterations = 0
        max_iterations = 50 # Safety limit to prevent infinite loops

        while changed && iterations < max_iterations
          changed = false
          iterations += 1

          macros.each do |name, macro_def|
            next unless macro_def.is_a?(ParameterizedMacro)

            pattern = "`#{name}("
            idx = result.index(pattern)
            next unless idx

            # Extract the arguments between the parens.
            args_start = idx + pattern.length
            args_str = extract_balanced_args(result, args_start)
            next unless args_str

            # The full match spans from idx to args_start + args_str.length + 1 (closing paren).
            full_end = args_start + args_str.length + 1 # +1 for the closing ')'

            args = split_args(args_str)
            expanded = macro_def.body.dup
            macro_def.params.each_with_index do |param, i|
              expanded = expanded.gsub(/\b#{Regexp.escape(param)}\b/, args[i] || "")
            end

            result = result[0...idx] + expanded + result[full_end..]
            changed = true
          end
        end

        result
      end

      # Extract the content between balanced parentheses.
      # Starts scanning at `start_idx` (just after the opening paren).
      # Returns the string between parens, or nil if unbalanced.
      #
      # Example: for "MAX(a, (b+c))", starting after "MAX(",
      #   returns "a, (b+c)" and the closing paren is at start_idx + 8.
      def self.extract_balanced_args(text, start_idx)
        depth = 1
        pos = start_idx

        while pos < text.length && depth > 0
          case text[pos]
          when "(" then depth += 1
          when ")" then depth -= 1
          end
          pos += 1 unless depth == 0
        end

        return nil unless depth == 0

        text[start_idx...pos]
      end

      # Split a comma-separated argument string, respecting nested parens.
      # "a, (b+c), d" => ["a", "(b+c)", "d"]
      def self.split_args(args_str)
        args = []
        current = +""
        depth = 0

        args_str.each_char do |ch|
          case ch
          when "("
            depth += 1
            current << ch
          when ")"
            depth -= 1
            current << ch
          when ","
            if depth == 0
              args << current.strip
              current = +""
            else
              current << ch
            end
          else
            current << ch
          end
        end

        args << current.strip unless current.strip.empty?
        args
      end

      private_class_method :currently_active?, :handle_define, :handle_ifdef,
        :handle_ifndef, :handle_else, :expand_macros,
        :expand_parameterized_macros, :extract_balanced_args, :split_args
    end
  end
end
