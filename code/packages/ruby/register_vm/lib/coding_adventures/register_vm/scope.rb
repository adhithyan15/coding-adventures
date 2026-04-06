# frozen_string_literal: true

# ==========================================================================
# Scope -- Lexical Scope Chain Management
# ==========================================================================
#
# Closures require the VM to capture the lexical environment at the point
# where a function is *defined*, not where it is *called*. Consider:
#
#   function counter() {
#     let count = 0;           // slot 0 in counter's context
#     return function() {
#       count += 1;            // captures counter's context
#       return count;
#     };
#   }
#   const inc = counter();
#   inc();  // => 1
#   inc();  // => 2  — count is still alive!
#
# When `counter` returns, its call frame is popped. But the `Context` object
# it created (with `count` in slot 0) is still referenced by the returned
# function. The garbage collector keeps it alive.
#
# The scope chain:
#
#   global_ctx ← counter_ctx ← inner_fn_ctx
#
# To read `count` from the innermost function, you walk 1 parent link
# (depth=1) and read slot 0 (idx=0).
#
# This matches V8's context chain layout, where every function that captures
# free variables gets a heap-allocated Context object. Functions that close
# over nothing (pure functions) can skip the context allocation entirely.
#
module CodingAdventures
  module RegisterVM
    module Scope
      # Create a new Context with `slot_count` slots, all initialized to UNDEFINED.
      #
      # @param parent     [Context, nil] the enclosing scope (nil = global scope)
      # @param slot_count [Integer] number of variable slots in this scope
      # @return [Context]
      def self.new_context(parent, slot_count)
        Context.new(
          slots:  Array.new(slot_count, UNDEFINED),
          parent: parent
        )
      end

      # Read a variable from the scope chain.
      #
      # @param ctx   [Context] the innermost context (head of chain)
      # @param depth [Integer] how many parent links to follow (0 = local)
      # @param idx   [Integer] slot index within the target context
      # @return the value stored at that slot
      #
      # Raises VMError if depth exceeds the chain length.
      def self.get_slot(ctx, depth, idx)
        node = walk(ctx, depth)
        node.slots[idx]
      end

      # Write a variable into the scope chain.
      #
      # @param ctx   [Context] the innermost context
      # @param depth [Integer] parent links to follow
      # @param idx   [Integer] slot index to write
      # @param value the new value
      def self.set_slot(ctx, depth, idx, value)
        node = walk(ctx, depth)
        node.slots[idx] = value
      end

      private

      # Walk `depth` parent links from `ctx`.
      # Raises VMError if the chain is shorter than `depth`.
      def self.walk(ctx, depth)
        node = ctx
        depth.times do |step|
          raise VMError, "Scope chain too short: tried to walk #{depth} levels but hit nil at step #{step}" if node.parent.nil?

          node = node.parent
        end
        node
      end
    end
  end
end
