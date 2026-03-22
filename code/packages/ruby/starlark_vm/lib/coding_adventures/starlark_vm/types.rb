# frozen_string_literal: true

# ==========================================================================
# Starlark VM Types -- Runtime Data Structures
# ==========================================================================
#
# These types represent the runtime objects that live on the VM's stack
# and in its variable stores while Starlark code is executing.
#
# StarlarkFunction -- A compiled function object (code + metadata).
#   When the compiler encounters `def foo(a, b=10):`, it emits a
#   MAKE_FUNCTION instruction. The VM handler creates a StarlarkFunction
#   with the function's CodeObject, parameter names, and default values.
#
# StarlarkIterator -- A stateful cursor over a sequence.
#   Created by GET_ITER, advanced by FOR_ITER. Tracks the current
#   position so `for x in items:` can step through one element at a time.
#
# StarlarkResult -- The output of executing a Starlark program.
#   Contains the final variable bindings, any printed output, and
#   the full execution trace (useful for debugging and visualization).
# ==========================================================================

module CodingAdventures
  module StarlarkVM
    # A compiled Starlark function, created by MAKE_FUNCTION.
    #
    # == Fields
    #
    # - code: the CodeObject containing the function's bytecode
    # - defaults: array of default argument values (for trailing params)
    # - name: the function's name (or "<lambda>" for anonymous functions)
    # - param_count: total number of parameters
    # - param_names: array of parameter name strings
    #
    # == Example
    #
    # For `def greet(name, greeting="Hello"):`, the compiler produces:
    #   code: CodeObject(...)  -- the function body bytecode
    #   defaults: ["Hello"]    -- one default value
    #   name: "greet"
    #   param_count: 2         -- two parameters total
    #   param_names: ["name", "greeting"]
    #
    class StarlarkFunction
      attr_reader :code, :defaults, :name, :param_count, :param_names

      def initialize(code:, defaults: [], name: "<lambda>", param_count: 0, param_names: [])
        @code = code
        @defaults = defaults
        @name = name
        @param_count = param_count
        @param_names = param_names
      end

      def to_s
        "<function #{@name}>"
      end
    end

    # A stateful iterator over a sequence of values.
    #
    # The VM creates iterators via GET_ITER and advances them via FOR_ITER.
    # This mirrors Python's iterator protocol:
    #
    #   iter = StarlarkIterator.new([10, 20, 30])
    #   iter.next_value  # => 10
    #   iter.next_value  # => 20
    #   iter.next_value  # => 30
    #   iter.next_value  # => nil (exhausted)
    #   iter.done?       # => true
    #
    class StarlarkIterator
      attr_reader :items
      attr_accessor :index

      def initialize(items)
        @items = items
        @index = 0
      end

      # Return the next value and advance the cursor.
      # Returns nil when the iterator is exhausted.
      def next_value
        return nil if @index >= @items.length
        val = @items[@index]
        @index += 1
        val
      end

      # True when all items have been consumed.
      def done?
        @index >= @items.length
      end
    end

    # The result of executing a Starlark program.
    #
    # == Fields
    #
    # - variables: Hash of variable name => value (the module-level namespace)
    # - output: Array of strings produced by print() calls
    # - traces: Array of VMTrace entries (one per instruction executed)
    #
    StarlarkResult = Data.define(:variables, :output, :traces)
  end
end
