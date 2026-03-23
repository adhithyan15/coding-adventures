# frozen_string_literal: true

# ================================================================
# ScopeChain -- Lexical Scoping for Lattice Variables
# ================================================================
#
# Why Lexical Scoping?
#
# CSS has no concept of scope — everything is global. But Lattice
# adds variables, mixins, and functions, which need scoping rules
# to prevent name collisions and enable local reasoning.
#
# Lattice uses **lexical (static) scoping**, meaning a variable's
# scope is determined by where it appears in the source text, not
# by runtime call order. This is the same model used by JavaScript,
# Python, and most modern languages.
#
# How It Works
#
# Each `{ }` block in the source creates a new child scope.
# Variables declared inside a block are local to that scope and
# its descendants. Looking up a variable walks up the parent chain
# until the name is found:
#
#   $color: red;              <- global scope (depth 0)
#   .parent {                 <- child scope (depth 1)
#     $color: blue;           <- shadows the global $color
#     color: $color;          -> blue  (found at depth 1)
#     .child {                <- grandchild scope (depth 2)
#       color: $color;        -> blue  (inherited from depth 1)
#     }
#   }
#   .sibling {                <- another child scope (depth 1)
#     color: $color;          -> red   (global, not affected by .parent)
#   }
#
# This is implemented as a linked list of scope nodes. Each node
# has a parent pointer and a bindings hash. Looking up a name
# walks the chain upward.
#
# Special Scoping Rules
#
# Mixin expansion creates a child scope of the caller's scope.
# This lets mixins see the caller's variables (like closures).
#
# Function evaluation creates an **isolated** scope whose parent
# is the definition-site global scope, NOT the caller's scope.
# This prevents functions from accidentally depending on where
# they're called from.
#
# Example:
#
#   global = ScopeChain.new
#   global.set("$color", "red")
#
#   block = global.child
#   block.set("$color", "blue")
#
#   block.get("$color")    # => "blue" (local)
#   global.get("$color")   # => "red"  (unchanged)
#
#   nested = block.child
#   nested.get("$color")   # => "blue" (inherited from parent)
# ================================================================

module CodingAdventures
  module LatticeAstToCss
    # A single scope node in the lexical scope chain.
    #
    # Each scope has:
    #   bindings: Hash mapping names to values (AST nodes or LatticeValues)
    #   parent:   The enclosing scope, or nil for the global scope
    class ScopeChain
      attr_reader :parent, :bindings

      # Create a new scope.
      #
      # @param parent [ScopeChain, nil] the enclosing scope
      def initialize(parent = nil)
        @parent = parent
        @bindings = {}
      end

      # Look up a name in this scope or any ancestor scope.
      #
      # Walks up the parent chain until the name is found. If the
      # name isn't found anywhere, returns nil.
      #
      # This is the core of lexical scoping — a variable declared
      # in an outer scope is visible in all inner scopes unless
      # shadowed by a local binding.
      #
      # @param name [String] the variable/mixin/function name
      # @return [Object, nil] the bound value, or nil if not found
      def get(name)
        return @bindings[name] if @bindings.key?(name)

        @parent&.get(name)
      end

      # Bind a name to a value in THIS scope (not the parent's).
      #
      # A child scope can shadow a parent's binding without modifying
      # the parent. This is intentional: nested scopes are isolated.
      #
      # @param name [String] the name to bind
      # @param value [Object] the value to associate with the name
      def set(name, value)
        @bindings[name] = value
      end

      # Check whether a name exists in this scope or any ancestor.
      #
      # Like `get`, walks up the parent chain. Returns true if the
      # name is bound anywhere, false otherwise.
      #
      # @param name [String] the name to check
      # @return [Boolean]
      def has?(name)
        return true if @bindings.key?(name)

        @parent ? @parent.has?(name) : false
      end

      # Check whether a name exists in THIS scope only (not parents).
      #
      # Useful for detecting re-declarations and shadowing.
      #
      # @param name [String] the name to check
      # @return [Boolean]
      def has_local?(name)
        @bindings.key?(name)
      end

      # Create a new child scope with self as parent.
      #
      # The child inherits all bindings from the parent chain via
      # `get`, but any `set` calls on the child only affect the child.
      #
      # @return [ScopeChain] a new child scope
      def child
        ScopeChain.new(self)
      end

      # How many levels deep this scope is (0 = global).
      #
      # The global scope has depth 0. Each child adds 1.
      #
      # @return [Integer]
      def depth
        @parent ? 1 + @parent.depth : 0
      end

      def inspect
        names = @bindings.keys
        "ScopeChain(depth=#{depth}, bindings=#{names.inspect})"
      end
    end
  end
end
