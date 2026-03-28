# frozen_string_literal: true

# ================================================================
# Mark-and-Sweep Garbage Collector
# ================================================================
#
# Invented by John McCarthy in 1960 for the original Lisp, mark-and-
# sweep is the simplest tracing garbage collector and the foundation
# for understanding all modern GC algorithms.
#
# The algorithm is three steps:
#
#   1. Mark  — Starting from roots (stack, globals), follow all
#              references and mark each reachable object.
#   2. Sweep — Walk the entire heap. Delete any object that wasn't
#              marked during the mark phase.
#   3. Reset — Clear all marks for the next collection cycle.
#
# Strengths
# ---------
#   + Correctly handles reference cycles (no separate cycle detector needed)
#   + No per-allocation overhead (no reference count updates)
#   + Simple to implement and reason about
#
# Weaknesses
# ----------
#   - Stop-the-world pause (proportional to heap size)
#   - Heap fragmentation (objects are not compacted after collection)
#   - Not incremental (must complete mark+sweep in one go)
#
# How Cycles Are Handled
# ----------------------
# If A references B and B references A, but neither is reachable from
# any root, both will be correctly collected. Mark-and-sweep starts
# from roots — unreachable cycles are simply never marked.
#
# This is the key advantage over reference counting, which requires
# a separate cycle detector to handle the same case.
# ================================================================

require_relative "heap_object"

module CodingAdventures
  module GarbageCollector
    # Mark-and-sweep garbage collector.
    #
    # The heap is a Hash mapping integer addresses to HeapObject instances.
    # Addresses are assigned sequentially starting from 0x10000 (65536) to
    # avoid ambiguity with small integer program values.
    #
    # @example Basic usage
    #   gc = MarkAndSweepGC.new
    #   addr = gc.allocate(ConsCell.new(car: 42, cdr: nil))
    #   cell = gc.deref(addr)
    #   gc.collect(roots: [addr])  # cell is reachable => 0 freed
    class MarkAndSweepGC
      # Starting address for heap objects. 65536 avoids ambiguity with
      # small integer values (0, 1, 2, ...) that programs commonly use.
      # Without this offset, the number "1" would be indistinguishable
      # from a pointer to heap object at address 1.
      HEAP_BASE_ADDRESS = 0x10000

      def initialize
        # The heap: integer address → HeapObject
        @heap = {}
        # Next address to allocate. Monotonically increasing; never reused.
        @next_address = HEAP_BASE_ADDRESS
        # Introspection counters
        @total_allocations = 0
        @total_collections = 0
        @total_freed       = 0
      end

      # Allocate an object on the heap and return its address.
      #
      # Each call assigns a new, never-before-used address.
      #
      # @param obj [HeapObject] the object to store
      # @return [Integer] heap address of the newly allocated object
      def allocate(obj)
        address = @next_address
        @next_address += 1
        @heap[address] = obj
        @total_allocations += 1
        address
      end

      # Look up a heap object by address.
      #
      # @param address [Integer]
      # @return [HeapObject]
      # @raise [KeyError] if address is not valid
      def deref(address)
        @heap.fetch(address)
      end

      # Run a mark-and-sweep collection cycle.
      #
      # Phase 1 — Mark: Recursively mark everything reachable from +roots+.
      # Phase 2 — Sweep: Delete unmarked objects, reset marks on survivors.
      #
      # @param roots [Array] values to scan for heap references. Integers
      #   that are valid heap addresses are followed. Other values are skipped.
      # @return [Integer] number of objects freed
      def collect(roots: [])
        @total_collections += 1

        # Phase 1: Mark all reachable objects
        roots.each { |root| mark_value(root) }

        # Phase 2: Sweep — delete unmarked, reset marks on survivors
        to_delete = []
        @heap.each do |address, obj|
          if obj.marked
            obj.marked = false # Reset for next cycle
          else
            to_delete << address
          end
        end

        to_delete.each { |addr| @heap.delete(addr) }

        freed = to_delete.size
        @total_freed += freed
        freed
      end

      # @return [Integer] number of objects currently on the heap
      def heap_size
        @heap.size
      end

      # Return introspection counters.
      # @return [Hash] keys: total_allocations, total_collections, total_freed, heap_size
      def stats
        {
          total_allocations: @total_allocations,
          total_collections: @total_collections,
          total_freed:       @total_freed,
          heap_size:         heap_size
        }
      end

      # Check whether an address points to a live heap object.
      # @param address [Integer]
      # @return [Boolean]
      def valid_address?(address)
        @heap.key?(address)
      end

      private

      # Recursively mark a value and everything it references.
      #
      # If +value+ is an integer that is a valid heap address, mark the
      # object at that address and recursively mark its references.
      # If +value+ is an Array or Hash, scan its contents.
      #
      # @param value [Object]
      def mark_value(value)
        case value
        when Integer
          # This integer might be a heap address. Follow it if so.
          if @heap.key?(value)
            obj = @heap[value]
            unless obj.marked
              obj.marked = true
              obj.references.each { |ref| mark_value(ref) }
            end
          end
        when Array
          value.each { |item| mark_value(item) }
        when Hash
          value.each_value { |v| mark_value(v) }
        end
        # Other types (String, NilClass, Symbol, etc.) hold no heap refs
      end
    end

    # Convenience factory that creates a symbol table backed by a GC.
    # Interning ensures each unique name maps to exactly one heap address.
    class SymbolTable
      def initialize(gc)
        @gc = gc
        @intern_map = {} # name String → heap address Integer
      end

      # Intern a symbol name — create it if new, return existing address.
      # @param name [String]
      # @return [Integer] heap address of the LispSymbol
      def intern(name)
        @intern_map[name] ||= @gc.allocate(LispSymbol.new(name: name))
      end

      # Return all currently interned addresses (used as GC roots).
      # @return [Array<Integer>]
      def all_addresses
        @intern_map.values
      end
    end
  end
end
