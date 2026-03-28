# frozen_string_literal: true

# ================================================================
# Heap Objects — Things That Live on the Managed Heap
# ================================================================
#
# In a Lisp VM, some values are simple — integers, booleans. These
# live directly in variables or on the stack. But structured values
# — cons cells, closures, symbols — need to be allocated on a
# managed heap so the garbage collector can track and free them.
#
# A HeapObject is anything that lives on the managed heap. Each
# heap object has an address (an integer), and other values refer
# to it by that address. The GC's job is to find which addresses
# are still reachable and free the rest.
#
# Think of the heap as a Hash:
#
#   heap = {
#     65536 => ConsCell.new(car: 42, cdr: 65537),
#     65537 => ConsCell.new(car: 99, cdr: nil),
#     65538 => Symbol.new(name: "factorial"),
#   }
#
# Object Hierarchy
# ----------------
#   HeapObject      — abstract base with #marked and #references
#     ConsCell      — pair of values (the Lisp building block)
#     LispSymbol    — interned name
#     LispClosure   — function + captured environment
# ================================================================

module CodingAdventures
  module GarbageCollector
    # Abstract base class for anything that lives on the managed heap.
    #
    # Every heap object has a +marked+ flag used by tracing GCs. During
    # the mark phase, reachable objects get marked. During the sweep phase,
    # unmarked objects are freed.
    #
    # Subclasses must implement +#references+ to return the heap addresses
    # they hold. This is how the GC discovers the object graph.
    class HeapObject
      attr_accessor :marked

      def initialize
        @marked = false
      end

      # Return all heap addresses that this object references.
      # The GC calls this during the mark phase to follow pointers.
      #
      # @return [Array<Integer>] heap addresses (may include non-address ints;
      #   the GC validates each one)
      def references
        []
      end
    end

    # A cons cell — the fundamental building block of Lisp lists.
    #
    # A cons cell is a pair: +car+ (the head) and +cdr+ (the tail).
    # Lists are chains of cons cells:
    #
    #   (1 2 3) = ConsCell(car: 1, cdr: ConsCell(car: 2, cdr: ConsCell(car: 3, cdr: nil)))
    #
    # When +car+ or +cdr+ is an Integer that is a valid heap address,
    # the GC treats it as a reference and follows it during marking.
    class ConsCell < HeapObject
      attr_accessor :car, :cdr

      def initialize(car: nil, cdr: nil)
        super()
        @car = car
        @cdr = cdr
      end

      # Both car and cdr might be heap addresses (integers).
      # @return [Array<Integer>]
      def references
        refs = []
        refs << @car if @car.is_a?(Integer)
        refs << @cdr if @cdr.is_a?(Integer)
        refs
      end

      def to_s
        "(#{@car} . #{@cdr})"
      end
    end

    # An interned symbol — a named atom in Lisp.
    #
    # Symbols are interned: every occurrence of the same name maps to the
    # same heap address. This makes identity-based equality (eq?) work.
    # Symbols don't reference other heap objects.
    class LispSymbol < HeapObject
      attr_accessor :name

      def initialize(name: "")
        super()
        @name = name
      end

      def references
        []
      end

      def to_s
        @name
      end
    end

    # A function closure — compiled code + captured environment.
    #
    # When a lambda expression is evaluated, it captures the current
    # variable bindings. The closure stores both the compiled code object
    # and those bindings. When called, the code executes in the captured
    # environment extended with the argument bindings.
    #
    # Integer values in +env+ might be heap addresses. The GC follows them
    # during marking to ensure captured objects stay alive.
    class LispClosure < HeapObject
      attr_accessor :code, :env, :params

      def initialize(code: nil, env: {}, params: [])
        super()
        @code   = code
        @env    = env
        @params = params
      end

      # Captured variable values that are integers might be heap addresses.
      # @return [Array<Integer>]
      def references
        @env.values.select { |v| v.is_a?(Integer) }
      end
    end
  end
end
