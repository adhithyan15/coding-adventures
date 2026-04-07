# frozen_string_literal: true

# ==========================================================================
# Feedback -- Type Profiling for JIT Readiness
# ==========================================================================
#
# One of V8's key innovations is *inline caches* (ICs) and *feedback vectors*.
# Every function has a feedback vector: an array with one slot per call site,
# property access, or arithmetic operation. Each slot records the types it
# has seen at runtime.
#
# Why does this matter?
#
#   function add(a, b) { return a + b; }
#
# If the JIT observes that `a` and `b` are always integers, it can generate
# machine code that skips the type checks and uses a direct CPU ADD instruction.
# That's 10–100× faster than the generic interpreter path.
#
# The IC state machine (each transition is one-way unless deoptimization occurs):
#
#   ┌──────────────┐  first observation  ┌─────────────────────┐
#   │ uninitialized│ ──────────────────► │ monomorphic         │
#   └──────────────┘                     │ (1 type pair seen)  │
#                                        └──────────┬──────────┘
#                                                   │ new type pair observed
#                                                   ▼
#                                        ┌─────────────────────┐
#                                        │ polymorphic         │
#                                        │ (2–4 type pairs)    │
#                                        └──────────┬──────────┘
#                                                   │ >4 type pairs
#                                                   ▼
#                                        ┌─────────────────────┐
#                                        │ megamorphic         │
#                                        │ (give up, slow path)│
#                                        └─────────────────────┘
#
# References:
#   https://v8.dev/blog/ignition-interpreter
#   https://mrale.ph/blog/2015/01/11/whats-up-with-monomorphism.html
#
module CodingAdventures
  module RegisterVM
    module Feedback
      # Global monotonic counter for hidden class IDs.
      # In a real engine this lives inside the heap, not a module variable.
      # We use a module-level variable here for simplicity.
      @@next_hidden_class_id = 0

      # Allocate a fresh hidden class ID. Thread-safety is not a concern in
      # this educational implementation (Ruby's GIL protects simple increments).
      #
      # @return [Integer] a unique hidden class identifier
      def self.new_hidden_class_id
        id = @@next_hidden_class_id
        @@next_hidden_class_id += 1
        id
      end

      # Reset the hidden class ID counter (useful between test runs).
      def self.reset_hidden_class_counter!
        @@next_hidden_class_id = 0
      end

      # Allocate a new feedback vector for a function with `size` slots.
      # All slots start as :uninitialized — the JIT ignores them until
      # they have been profiled at least once.
      #
      # @param size [Integer] number of feedback slots
      # @return [Array<Symbol|Hash>]
      def self.new_vector(size)
        Array.new(size, :uninitialized)
      end

      # -----------------------------------------------------------------------
      # Type classification
      # -----------------------------------------------------------------------
      # Maps a Ruby runtime value to the JS-style type name string.
      # This string is what ends up in the feedback slot's types array.
      #
      #   value_type(42)         => "number"
      #   value_type("hi")       => "string"
      #   value_type(true)       => "boolean"
      #   value_type(nil)        => "null"
      #   value_type(UNDEFINED)  => "undefined"
      #   value_type(VMObject..) => "object"
      #   value_type([])         => "array"
      #   value_type(VMFunction) => "function"
      #
      def self.value_type(v)
        case v
        when Integer, Float  then "number"
        when String          then "string"
        when TrueClass, FalseClass then "boolean"
        when NilClass        then "null"
        when VMFunction      then "function"
        when VMObject        then "object"
        when Array           then "array"
        else
          # Sentinel UNDEFINED is a plain Object — check identity last
          v.equal?(UNDEFINED) ? "undefined" : "unknown"
        end
      end

      # -----------------------------------------------------------------------
      # Recording helpers — called by interpreter before each operation
      # -----------------------------------------------------------------------

      # Record a binary arithmetic/comparison operation.
      #
      # @param vector    [Array] the frame's feedback vector
      # @param slot_idx  [Integer] which slot to update (nil = skip profiling)
      # @param left      the left operand (accumulator)
      # @param right     the right operand (from register)
      def self.record_binary_op(vector, slot_idx, left, right)
        return if slot_idx.nil?

        pair = [value_type(left), value_type(right)]
        vector[slot_idx] = update_slot(vector[slot_idx], pair)
      end

      # Record a property load: the shape (hidden class ID) of the receiver.
      #
      # A monomorphic property access always loads from an object of the same
      # hidden class. If the hidden class changes (because objects have different
      # property layouts), the slot goes polymorphic.
      #
      # @param vector          [Array] the frame's feedback vector
      # @param slot_idx        [Integer] which slot to update
      # @param hidden_class_id [Integer] the receiver object's hidden class ID
      def self.record_property_load(vector, slot_idx, hidden_class_id)
        return if slot_idx.nil?

        tag = "class:#{hidden_class_id}"
        pair = [tag, tag]
        vector[slot_idx] = update_slot(vector[slot_idx], pair)
      end

      # Record a call site: the type of the function being called.
      #
      # @param vector      [Array] the frame's feedback vector
      # @param slot_idx    [Integer] which slot to update
      # @param callee_type [String] result of value_type(callee)
      def self.record_call_site(vector, slot_idx, callee_type)
        return if slot_idx.nil?

        pair = [callee_type, callee_type]
        vector[slot_idx] = update_slot(vector[slot_idx], pair)
      end

      # -----------------------------------------------------------------------
      # State machine transition
      # -----------------------------------------------------------------------
      # Returns the new slot state after observing one more (left, right) type pair.
      #
      #   uninitialized + pair  => monomorphic { types: [pair] }
      #   monomorphic  + same   => monomorphic (no change)
      #   monomorphic  + new    => polymorphic { types: [pair, old_pair] }
      #   polymorphic  + same   => polymorphic (no change)
      #   polymorphic  + new,  len < 4  => polymorphic with one more pair
      #   polymorphic  + new,  len >= 4 => megamorphic
      #   megamorphic  + any    => megamorphic (terminal state)
      #
      def self.update_slot(slot, pair)
        case slot
        when :uninitialized
          { kind: :monomorphic, types: [pair] }

        when :megamorphic
          :megamorphic

        when Hash
          case slot[:kind]
          when :monomorphic
            # No change if same pair
            return slot if slot[:types].include?(pair)

            # Transition to polymorphic
            { kind: :polymorphic, types: [pair] + slot[:types] }

          when :polymorphic
            return slot if slot[:types].include?(pair)
            # More than 4 unique pairs → give up
            return :megamorphic if slot[:types].length >= 4

            { kind: :polymorphic, types: [pair] + slot[:types] }

          else
            :megamorphic
          end

        else
          # Unrecognized slot state — treat as megamorphic
          :megamorphic
        end
      end
    end
  end
end
