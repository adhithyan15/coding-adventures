# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Pushdown Automaton (PDA) -- a finite automaton with a stack.
# ---------------------------------------------------------------------------
#
# === What is a PDA? ===
#
# A PDA is a state machine augmented with a **stack** -- an unbounded LIFO
# (last-in, first-out) data structure. The stack gives the PDA the ability
# to "remember" things that a finite automaton cannot, like how many open
# parentheses it has seen.
#
# This extra memory is exactly what is needed to recognize **context-free
# languages** -- the class of languages that includes balanced parentheses,
# nested HTML tags, arithmetic expressions, and most programming language
# syntax.
#
# === The Chomsky Hierarchy Connection ===
#
#     Regular languages    <  Context-free languages  <  Context-sensitive  <  RE
#     (DFA/NFA)              (PDA)                       (LBA)                (TM)
#
# A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
# recognize "does this string have equal numbers of a's and b's?" -- that
# requires counting, and a DFA has no memory beyond its finite state.
#
# A PDA can recognize "a^n b^n" (n a's followed by n b's) because it can
# push an 'a' for each 'a' it reads, then pop an 'a' for each 'b'. If the
# stack is empty at the end, the counts match.
#
# === Formal Definition ===
#
#     PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)
#
#     Q      = finite set of states
#     Sigma  = input alphabet
#     Gamma  = stack alphabet (may differ from Sigma)
#     delta  = transition function: Q x (Sigma union {epsilon}) x Gamma -> P(Q x Gamma*)
#     q0     = initial state
#     Z0     = initial stack symbol (bottom marker)
#     F      = accepting states
#
# Our implementation is deterministic (DPDA): at most one transition
# applies at any time. This is simpler to implement and trace, and
# sufficient for most practical parsing tasks.
#
# === Stack Semantics ===
#
# The stack is represented as an Array where the LAST element is the top.
# Each transition reads (pops) the top of the stack and optionally pushes
# new symbols.
#
#     stack_push = []           -> pop the top (consume it, push nothing)
#     stack_push = ["X"]        -> replace top with X (pop then push X)
#     stack_push = ["X", "Y"]   -> pop top, push X, then push Y (Y is new top)
#     stack_push = [stack_read] -> effectively leave the stack unchanged
#
# === Ruby Implementation Notes ===
#
# - PDATransition and PDATraceEntry are Structs for value semantics.
# - Transitions are indexed in a Hash for O(1) lookup by (state, event, stack_top).
# - The stack is a plain Array with push/pop at the end (Ruby convention).
# - accepts() runs on copied state to avoid mutation (same as DFA/NFA).
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # A single transition rule for a pushdown automaton.
    #
    # A PDA transition says: "If I am in state +source+, and I see input
    # +event+ (or epsilon if nil), and the top of my stack is +stack_read+,
    # then move to state +target+ and replace the stack top with +stack_push+."
    #
    # Example:
    #   PDATransition.new("q0", "(", "$", "q0", ["(", "$"])
    #   # "In q0, reading '(', with '$' on top: stay in q0, push '(' above '$'"
    PDATransition = Struct.new(:source, :event, :stack_read, :target, :stack_push)

    # One step in a PDA's execution trace.
    #
    # Captures the full state of the PDA at each transition: which rule
    # fired, what the stack looked like before and after.
    PDATraceEntry = Struct.new(:source, :event, :stack_read, :target, :stack_push, :stack_after)

    class PushdownAutomaton
      # The finite set of states (frozen Set of strings).
      attr_reader :states

      # The current state (string).
      attr_reader :current_state

      # Create a new Deterministic Pushdown Automaton.
      #
      # @param states [Set<String>] Finite set of states.
      # @param input_alphabet [Set<String>] Finite set of input symbols.
      # @param stack_alphabet [Set<String>] Finite set of stack symbols.
      # @param transitions [Array<PDATransition>] List of transition rules.
      # @param initial [String] Starting state.
      # @param initial_stack_symbol [String] Symbol placed on the stack initially
      #   (typically '$' as a bottom-of-stack marker).
      # @param accepting [Set<String>] Set of accepting/final states.
      # @raise [ArgumentError] If validation fails.
      def initialize(states:, input_alphabet:, stack_alphabet:, transitions:,
        initial:, initial_stack_symbol:, accepting:)
        raise ArgumentError, "States set must be non-empty" if states.empty?

        unless states.include?(initial)
          raise ArgumentError,
            "Initial state '#{initial}' is not in the states set"
        end

        unless stack_alphabet.include?(initial_stack_symbol)
          raise ArgumentError,
            "Initial stack symbol '#{initial_stack_symbol}' is not in " \
            "the stack alphabet"
        end

        invalid_accepting = accepting - states
        unless invalid_accepting.empty?
          raise ArgumentError,
            "Accepting states #{invalid_accepting.to_a.sort} are not in " \
            "the states set"
        end

        @states = states.to_set.freeze
        @input_alphabet = input_alphabet.to_set.freeze
        @stack_alphabet = stack_alphabet.to_set.freeze
        @transitions = transitions.dup.freeze
        @initial = initial.freeze
        @initial_stack_symbol = initial_stack_symbol.freeze
        @accepting = accepting.to_set.freeze

        # Index transitions for fast lookup: [state, event_or_nil, stack_top] -> PDATransition
        #
        # This is the key optimization: instead of searching through all transitions
        # on every step, we build a hash index so lookup is O(1).
        @transition_index = {}
        transitions.each do |t|
          key = [t.source, t.event, t.stack_read]
          if @transition_index.key?(key)
            raise ArgumentError,
              "Duplicate transition for (#{t.source}, #{t.event.inspect}, " \
              "#{t.stack_read.inspect}) -- this PDA must be deterministic"
          end
          @transition_index[key] = t
        end
        @transition_index.freeze

        # Mutable execution state
        @current_state = initial
        @stack = [initial_stack_symbol]
        @trace = []
      end

      # Current stack contents (bottom to top) as a frozen Array.
      #
      # The stack is stored as an Array where index 0 is the bottom
      # and the last index is the top. This matches the natural Ruby
      # convention where Array#push and Array#pop work on the end.
      #
      # @return [Array<String>]
      def stack
        @stack.dup.freeze
      end

      # The top of the stack, or nil if empty.
      #
      # @return [String, nil]
      def stack_top
        @stack.last
      end

      # The execution trace -- a list of PDATraceEntry objects.
      # Returns a copy.
      #
      # @return [Array<PDATraceEntry>]
      def trace
        @trace.dup
      end

      # === Processing ===

      # Process one input symbol.
      #
      # Looks up a transition matching (current_state, event, stack_top).
      # If found, applies it (pops the stack top, pushes new symbols,
      # changes state). If not found, raises an error.
      #
      # @param event [String] An input symbol.
      # @return [String] The new current state.
      # @raise [ArgumentError] If no transition matches.
      def process(event)
        t = find_transition(event)
        if t.nil?
          raise ArgumentError,
            "No transition for (state='#{@current_state}', " \
            "event=#{event.inspect}, stack_top=#{stack_top.inspect})"
        end
        apply_transition(t)
        @current_state
      end

      # Process a sequence of inputs and return the trace.
      #
      # After processing all inputs, tries epsilon transitions until
      # none are available (this handles acceptance transitions that
      # fire at end-of-input).
      #
      # @param events [Array<String>] List of input symbols.
      # @return [Array<PDATraceEntry>] The trace entries generated during processing.
      def process_sequence(events)
        trace_start = @trace.length
        events.each { |event| process(event) }
        # Try epsilon transitions at end of input
        loop { break unless try_epsilon }
        @trace[trace_start..]
      end

      # Check if the PDA accepts the input sequence.
      #
      # Processes all inputs, then tries epsilon transitions until none
      # are available. Returns true if the final state is accepting.
      #
      # Does NOT modify this PDA's state -- runs on a copy.
      #
      # The algorithm:
      #   1. Start with initial state and initial stack.
      #   2. For each input event:
      #      a. If stack is empty, reject (can't read stack top).
      #      b. Look up transition for (state, event, stack_top).
      #      c. If no transition, reject.
      #      d. Pop stack top, push new symbols, change state.
      #   3. After all input, try epsilon transitions (bounded by
      #      number of transitions to prevent infinite loops).
      #   4. Accept if final state is in accepting set.
      #
      # @param events [Array<String>] List of input symbols.
      # @return [Boolean] True if the PDA accepts.
      def accepts(events)
        # Simulate on copies of the mutable state
        state = @initial
        stack = [@initial_stack_symbol]

        events.each do |event|
          return false if stack.empty?
          top = stack.last
          t = @transition_index[[state, event, top]]
          return false if t.nil?
          stack.pop
          stack.concat(t.stack_push)
          state = t.target
        end

        # Try epsilon transitions at end of input.
        # Bounded by number of transitions + 1 to prevent infinite loops
        # (each epsilon transition should make progress).
        max_epsilon = @transitions.length + 1
        max_epsilon.times do
          break if stack.empty?
          top = stack.last
          t = @transition_index[[state, nil, top]]
          break if t.nil?
          stack.pop
          stack.concat(t.stack_push)
          state = t.target
        end

        @accepting.include?(state)
      end

      # Reset to initial state with initial stack.
      def reset
        @current_state = @initial
        @stack = [@initial_stack_symbol]
        @trace = []
      end

      # Return a readable representation.
      #
      # @return [String]
      def inspect
        "PDA(states=#{@states.to_a.sort}, " \
          "current='#{@current_state}', " \
          "stack=#{@stack})"
      end

      alias_method :to_s, :inspect

      private

      # Find a matching transition for the current state and stack top.
      #
      # @param event [String, nil] The input event (nil for epsilon).
      # @return [PDATransition, nil] The matching transition, or nil.
      def find_transition(event)
        return nil if @stack.empty?
        top = @stack.last
        @transition_index[[@current_state, event, top]]
      end

      # Apply a transition: change state and modify the stack.
      #
      # @param transition [PDATransition] The transition to apply.
      def apply_transition(transition)
        # Pop the stack top (it was "read" by the transition)
        @stack.pop

        # Push new symbols (in order: first element goes deepest)
        transition.stack_push.each { |symbol| @stack.push(symbol) }

        # Record the trace
        @trace << PDATraceEntry.new(
          transition.source,
          transition.event,
          transition.stack_read,
          transition.target,
          transition.stack_push.dup,
          @stack.dup.freeze
        )

        # Change state
        @current_state = transition.target
      end

      # Try to take an epsilon transition. Returns true if one was taken.
      #
      # @return [Boolean]
      def try_epsilon
        t = find_transition(nil)
        return false if t.nil?
        apply_transition(t)
        true
      end
    end
  end
end
