# frozen_string_literal: true

# ================================================================
# Lattice Compiler Error Types
# ================================================================
#
# Every error in the Lattice compiler carries:
#   - A human-readable message explaining what went wrong.
#   - The line and column where the error occurred.
#
# Error hierarchy mirrors the three-pass compiler:
#
# Pass 1 (Module Resolution):
#   LatticeModuleNotFoundError  -- @use references nonexistent file
#
# Pass 2 (Symbol Collection):
#   LatticeReturnOutsideFunctionError -- @return outside @function
#
# Pass 3 (Expansion):
#   LatticeUndefinedVariableError  -- $var never declared
#   LatticeUndefinedMixinError     -- @include unknown mixin
#   LatticeUndefinedFunctionError  -- function call to unknown fn
#   LatticeWrongArityError         -- wrong number of args
#   LatticeCircularReferenceError  -- mixin/fn calls itself
#   LatticeTypeErrorInExpression   -- 10px + red (incompatible types)
#   LatticeUnitMismatchError       -- 10px + 5s (incompatible units)
#   LatticeMissingReturnError      -- function has no @return
#
# All inherit from LatticeError so callers can rescue the whole
# family with a single `rescue LatticeError`.
#
# Example:
#
#   begin
#     css = CodingAdventures::LatticeTranspiler.transpile(source)
#   rescue CodingAdventures::LatticeAstToCss::LatticeError => e
#     puts "Error at line #{e.line}, col #{e.column}: #{e.message}"
#   end
# ================================================================

module CodingAdventures
  module LatticeAstToCss
    # ============================================================
    # Base error class for all Lattice compiler errors.
    # ============================================================
    class LatticeError < StandardError
      # @return [String] human-readable error description
      attr_reader :message

      # @return [Integer] 1-based line number (0 if unknown)
      attr_reader :line

      # @return [Integer] 1-based column number (0 if unknown)
      attr_reader :column

      # @param message [String] error description
      # @param line [Integer] line number (0 if unknown)
      # @param column [Integer] column number (0 if unknown)
      def initialize(message, line = 0, column = 0)
        @message = message
        @line = line
        @column = column
        location = line.positive? ? " at line #{line}, column #{column}" : ""
        super("#{message}#{location}")
      end
    end

    # ============================================================
    # Pass 1: Module Resolution Errors
    # ============================================================

    # Raised when @use references a module that cannot be found.
    #
    # Example: @use "nonexistent";
    class LatticeModuleNotFoundError < LatticeError
      attr_reader :module_name

      def initialize(module_name, line = 0, column = 0)
        @module_name = module_name
        super("Module '#{module_name}' not found", line, column)
      end
    end

    # ============================================================
    # Pass 2: Symbol Collection Errors
    # ============================================================

    # Raised when @return appears outside a @function body.
    #
    # Example: @return 42;  (at top level or inside a mixin)
    class LatticeReturnOutsideFunctionError < LatticeError
      def initialize(line = 0, column = 0)
        super("@return outside @function", line, column)
      end
    end

    # ============================================================
    # Pass 3: Expansion Errors
    # ============================================================

    # Raised when a $variable is referenced but never declared.
    #
    # Example: color: $nonexistent;
    class LatticeUndefinedVariableError < LatticeError
      attr_reader :name

      def initialize(name, line = 0, column = 0)
        @name = name
        super("Undefined variable '#{name}'", line, column)
      end
    end

    # Raised when @include references a mixin that was never defined.
    #
    # Example: @include nonexistent;
    class LatticeUndefinedMixinError < LatticeError
      attr_reader :name

      def initialize(name, line = 0, column = 0)
        @name = name
        super("Undefined mixin '#{name}'", line, column)
      end
    end

    # Raised when a function call references an unknown function.
    #
    # Note: CSS built-ins (rgb, calc, var) are NOT affected —
    # they are passed through unchanged.
    #
    # Example: padding: spacing(2);  (if spacing was never defined)
    class LatticeUndefinedFunctionError < LatticeError
      attr_reader :name

      def initialize(name, line = 0, column = 0)
        @name = name
        super("Undefined function '#{name}'", line, column)
      end
    end

    # Raised when a mixin or function is called with the wrong arg count.
    #
    # The expected count accounts for parameters with defaults —
    # only parameters WITHOUT defaults are required.
    #
    # Example: @mixin button($bg, $fg) called as @include button(red, blue, green)
    class LatticeWrongArityError < LatticeError
      attr_reader :name, :expected, :got

      def initialize(kind, name, expected, got, line = 0, column = 0)
        @name = name
        @expected = expected
        @got = got
        super("#{kind} '#{name}' expects #{expected} args, got #{got}", line, column)
      end
    end

    # Raised when a mixin or function calls itself, forming a cycle.
    #
    # The chain shows the full call path: a -> b -> a.
    #
    # Example: @mixin a { @include b; }  @mixin b { @include a; }
    class LatticeCircularReferenceError < LatticeError
      attr_reader :chain

      def initialize(kind, chain, line = 0, column = 0)
        @chain = chain
        chain_str = chain.join(" -> ")
        super("Circular #{kind}: #{chain_str}", line, column)
      end
    end

    # Raised when arithmetic is attempted on incompatible types.
    #
    # Example: 10px + red  (dimension + color/ident)
    class LatticeTypeErrorInExpression < LatticeError
      attr_reader :op, :left_type, :right_type

      def initialize(op, left, right, line = 0, column = 0)
        @op = op
        @left_type = left
        @right_type = right
        super("Cannot #{op} '#{left}' and '#{right}'", line, column)
      end
    end

    # Raised when arithmetic combines dimensions with incompatible units.
    #
    # Compatible units: 10px + 5px -> 15px
    # Incompatible units: 10px + 5s (length + time) -> error
    #
    # Example: 10px + 5s
    class LatticeUnitMismatchError < LatticeError
      attr_reader :left_unit, :right_unit

      def initialize(left_unit, right_unit, line = 0, column = 0)
        @left_unit = left_unit
        @right_unit = right_unit
        super("Cannot add '#{left_unit}' and '#{right_unit}' units", line, column)
      end
    end

    # Raised when a function body has no @return statement.
    #
    # Every @function must return a value via @return. A function
    # body with no @return in any reachable branch is an error.
    #
    # Example: @function noop($x) { $y: $x; }
    class LatticeMissingReturnError < LatticeError
      attr_reader :name

      def initialize(name, line = 0, column = 0)
        @name = name
        super("Function '#{name}' has no @return", line, column)
      end
    end
  end
end
