# frozen_string_literal: true

# ==========================================================================
# Table --- WASM Table for Indirect Function Calls
# ==========================================================================
#
# A WASM table is an array of opaque references --- in WASM 1.0, these
# are always function references (funcref). Tables enable indirect
# function calls: instead of calling a function by its index directly,
# code looks up a function reference in a table at runtime.
#
# This is how WASM implements C-style function pointers, virtual method
# dispatch, and dynamic linking.
#
# Tables are *opaque*: code cannot manufacture a function reference from
# an integer. It can only read references placed by the module or host.
# This is capability-based security.
#
#   +----+----+------+----+----+------+
#   | f0 | f3 | nil  | f1 | f5 | nil  |   <- Table elements
#   +----+----+------+----+----+------+
#     0    1     2     3    4     5
# ==========================================================================

module CodingAdventures
  module WasmExecution
    class Table
      # @param initial_size [Integer] number of entries, all initialized to nil
      # @param max_size [Integer, nil] optional upper bound on table size
      def initialize(initial_size, max_size = nil)
        @elements = Array.new(initial_size)
        @max_size = max_size
      end

      # Get the function index at the given table index.
      # Returns nil if the entry is empty (uninitialized).
      # Traps if the index is out of bounds.
      def get(index)
        if index < 0 || index >= @elements.length
          raise TrapError,
                "Out of bounds table access: index=#{index}, table size=#{@elements.length}"
        end
        @elements[index]
      end

      # Set the function index at the given table index.
      def set(index, func_index)
        if index < 0 || index >= @elements.length
          raise TrapError,
                "Out of bounds table access: index=#{index}, table size=#{@elements.length}"
        end
        @elements[index] = func_index
      end

      # Return the current table size (number of entries).
      def size
        @elements.length
      end

      # Grow the table by +delta+ entries (initialized to nil).
      # Returns the old size on success, or -1 on failure.
      def grow(delta)
        old_size = @elements.length
        new_size = old_size + delta

        return -1 if @max_size && new_size > @max_size

        delta.times { @elements.push(nil) }
        old_size
      end
    end
  end
end
