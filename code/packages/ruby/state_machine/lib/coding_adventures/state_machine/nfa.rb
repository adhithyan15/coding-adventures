# frozen_string_literal: true

require "set"

# ---------------------------------------------------------------------------
# Non-deterministic Finite Automaton (NFA) with epsilon transitions.
# ---------------------------------------------------------------------------
#
# === What is an NFA? ===
#
# An NFA relaxes the deterministic constraint of a DFA in two ways:
#
# 1. **Multiple transitions:** A single (state, input) pair can lead to
#    multiple target states. The machine explores all possibilities
#    simultaneously -- like spawning parallel universes.
#
# 2. **Epsilon transitions:** The machine can jump to another state
#    without consuming any input. These are "free" moves.
#
# === The "parallel universes" model ===
#
# Think of an NFA as a machine that clones itself at every non-deterministic
# choice point. All clones run in parallel:
#
#     - A clone that reaches a dead end (no transition) simply vanishes.
#     - A clone that reaches an accepting state means the whole NFA accepts.
#     - If ALL clones die without reaching an accepting state, the NFA rejects.
#
# The NFA accepts if there EXISTS at least one path through the machine
# that ends in an accepting state.
#
# === Why NFAs matter ===
#
# NFAs are much easier to construct for certain problems. For example, "does
# this string contain the substring 'abc'?" is trivial as an NFA (just guess
# where 'abc' starts) but requires careful tracking as a DFA.
#
# Every NFA can be converted to an equivalent DFA via subset construction.
# This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
# efficient execution (O(1) per character).
#
# === Formal definition ===
#
#     NFA = (Q, Sigma, delta, q0, F)
#
#     Q      = finite set of states
#     Sigma  = finite alphabet (input symbols)
#     delta  = transition function: Q x (Sigma union {epsilon}) -> P(Q)
#              maps (state, input_or_epsilon) to a SET of states
#     q0     = initial state
#     F      = accepting states
#
# === Ruby Implementation Notes ===
#
# - Transitions map [state, event_or_epsilon] to a Set of target states.
# - EPSILON is the empty string "". No real alphabet symbol should be empty.
# - The NFA tracks a Set of current states (the "active" parallel universes).
# - epsilon_closure uses a worklist algorithm (iterative, not recursive) to
#   avoid stack overflow on deep epsilon chains.
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # Sentinel value for epsilon transitions (transitions that consume no input).
    #
    # We use the empty string "" as the epsilon symbol. This works because
    # no real input alphabet should contain the empty string -- input symbols
    # are always at least one character long.
    EPSILON = ""

    class NFA
      # The finite set of states (frozen Set of strings).
      attr_reader :states

      # The finite set of input symbols (frozen Set of strings).
      attr_reader :alphabet

      # The initial state (string).
      attr_reader :initial

      # The set of accepting/final states (frozen Set of strings).
      attr_reader :accepting

      # The set of states the NFA is currently in (frozen Set of strings).
      attr_reader :current_states

      # Create a new NFA.
      #
      # @param states [Set<String>] The finite set of states. Must be non-empty.
      # @param alphabet [Set<String>] The finite set of input symbols. Must not
      #   contain the empty string (reserved for epsilon).
      # @param transitions [Hash<Array(String, String), Set<String>>] Mapping from
      #   [state, event_or_epsilon] to a Set of target states. Use EPSILON ("")
      #   for epsilon transitions.
      # @param initial [String] The starting state. Must be in +states+.
      # @param accepting [Set<String>] The set of accepting/final states.
      # @raise [ArgumentError] If any validation check fails.
      def initialize(states:, alphabet:, transitions:, initial:, accepting:)
        raise ArgumentError, "States set must be non-empty" if states.empty?

        if alphabet.include?(EPSILON)
          raise ArgumentError,
            "Alphabet must not contain the empty string (reserved for epsilon)"
        end

        unless states.include?(initial)
          raise ArgumentError,
            "Initial state '#{initial}' is not in the states set"
        end

        invalid_accepting = accepting - states
        unless invalid_accepting.empty?
          raise ArgumentError,
            "Accepting states #{invalid_accepting.to_a.sort} are not in " \
            "the states set"
        end

        # Validate transitions
        transitions.each do |(source, event), targets|
          unless states.include?(source)
            raise ArgumentError,
              "Transition source '#{source}' is not in the states set"
          end
          if event != EPSILON && !alphabet.include?(event)
            raise ArgumentError,
              "Transition event '#{event}' is not in the alphabet " \
              "and is not epsilon"
          end
          invalid_targets = targets - states
          unless invalid_targets.empty?
            raise ArgumentError,
              "Transition targets #{invalid_targets.to_a.sort} " \
              "(from (#{source}, #{event.inspect})) are not in the states set"
          end
        end

        @states = states.to_set.freeze
        @alphabet = alphabet.to_set.freeze
        # Store transitions with frozen Set values for immutability
        @transitions = transitions.transform_values { |v| v.to_set.freeze }.freeze
        @initial = initial.freeze
        @accepting = accepting.to_set.freeze

        # The NFA starts in the epsilon closure of the initial state.
        # This is because epsilon transitions are "free" -- the machine can
        # take them before reading any input.
        @current_states = epsilon_closure(Set[initial])
      end

      # === Epsilon Closure ===

      # Compute the epsilon closure of a set of states.
      #
      # Starting from the given states, follow ALL epsilon transitions
      # recursively. Return the full set of states reachable via zero or
      # more epsilon transitions.
      #
      # This is the key operation that makes NFAs work: before and after
      # processing each input, we expand to include all states reachable
      # via "free" epsilon moves.
      #
      # The algorithm is a simple worklist (BFS/DFS over epsilon edges):
      #
      #     1. Start with the input set.
      #     2. For each state, find epsilon transitions.
      #     3. Add all targets to the set.
      #     4. Repeat until no new states are found.
      #
      # We use a worklist rather than recursion to:
      # - Avoid stack overflow on deep epsilon chains
      # - Handle epsilon cycles without infinite loops
      # - Make the algorithm's termination obvious
      #
      # @param state_set [Set<String>] The starting set of states.
      # @return [Set<String>] All states reachable via epsilon transitions.
      #
      # Example:
      #   Given: q0 --epsilon--> q1 --epsilon--> q2
      #   epsilon_closure(Set["q0"]) == Set["q0", "q1", "q2"]
      def epsilon_closure(state_set)
        closure = state_set.to_set.dup
        worklist = state_set.to_a

        until worklist.empty?
          state = worklist.pop
          # Find epsilon transitions from this state
          targets = @transitions[[state, EPSILON]] || Set.new
          targets.each do |target|
            unless closure.include?(target)
              closure.add(target)
              worklist << target
            end
          end
        end

        closure.freeze
      end

      # === Processing ===

      # Process one input event and return the new set of states.
      #
      # For each current state, find all transitions on this event.
      # Take the union of all target states, then compute the epsilon
      # closure of the result.
      #
      # @param event [String] An input symbol from the alphabet.
      # @return [Set<String>] The new set of current states after processing.
      # @raise [ArgumentError] If the event is not in the alphabet.
      def process(event)
        unless @alphabet.include?(event)
          raise ArgumentError,
            "Event '#{event}' is not in the alphabet #{@alphabet.to_a.sort}"
        end

        # Collect all target states from all current states
        next_states = Set.new
        @current_states.each do |state|
          targets = @transitions[[state, event]] || Set.new
          next_states.merge(targets)
        end

        # Expand via epsilon closure
        @current_states = epsilon_closure(next_states)
      end

      # Process a sequence of inputs and return the trace.
      #
      # Each entry in the trace is: [states_before, event, states_after].
      #
      # @param events [Array<String>] A list of input symbols.
      # @return [Array<Array>] List of [before_states, event, after_states] arrays.
      def process_sequence(events)
        trace = []
        events.each do |event|
          before = @current_states
          process(event)
          trace << [before, event, @current_states]
        end
        trace
      end

      # Check if the NFA accepts the input sequence.
      #
      # The NFA accepts if, after processing all inputs, ANY of the
      # current states is an accepting state.
      #
      # Does NOT modify the NFA's current state -- runs on a copy.
      #
      # @param events [Array<String>] A list of input symbols.
      # @return [Boolean] True if the NFA accepts, false otherwise.
      # @raise [ArgumentError] If any event is not in the alphabet.
      def accepts(events)
        # Simulate without modifying this NFA's state
        current = epsilon_closure(Set[@initial])

        events.each do |event|
          unless @alphabet.include?(event)
            raise ArgumentError,
              "Event '#{event}' is not in the alphabet #{@alphabet.to_a.sort}"
          end
          next_states = Set.new
          current.each do |state|
            targets = @transitions[[state, event]] || Set.new
            next_states.merge(targets)
          end
          current = epsilon_closure(next_states)

          # If no states are active, the NFA is dead -- reject early
          return false if current.empty?
        end

        # Accept if any current state is accepting
        !(current & @accepting).empty?
      end

      # Reset to the initial state (with epsilon closure).
      def reset
        @current_states = epsilon_closure(Set[@initial])
      end

      # === Conversion to DFA ===

      # Convert this NFA to an equivalent DFA using subset construction.
      #
      # === The Subset Construction Algorithm ===
      #
      # The key insight: if an NFA can be in states {q0, q1, q3}
      # simultaneously, we create a single DFA state representing that
      # entire set. The DFA's states are sets of NFA states.
      #
      # Algorithm:
      #     1. Start with d0 = epsilon-closure({q0})
      #     2. For each DFA state D and each input symbol a:
      #         - For each NFA state q in D, find delta(q, a)
      #         - Take the union of all targets
      #         - Compute epsilon-closure of the union
      #         - That is the new DFA state D'
      #     3. Repeat until no new DFA states are discovered
      #     4. A DFA state is accepting if it contains ANY NFA accepting state
      #
      # DFA state names are generated from sorted NFA state names:
      #     Set["q0", "q1"] -> "{q0,q1}"
      #
      # @return [DFA] A DFA that recognizes exactly the same language as this NFA.
      def to_dfa
        # Step 1: initial DFA state = epsilon-closure of NFA initial state
        start_closure = epsilon_closure(Set[@initial])
        dfa_start = state_set_name(start_closure)

        # Track DFA states and transitions as we discover them
        dfa_states = Set[dfa_start]
        dfa_transitions = {}
        dfa_accepting = Set.new

        # Map from DFA state name -> Set of NFA states
        state_map = {dfa_start => start_closure}

        # Check if start state is accepting
        if !(start_closure & @accepting).empty?
          dfa_accepting.add(dfa_start)
        end

        # Step 2-3: BFS over DFA states
        worklist = [dfa_start]

        until worklist.empty?
          current_name = worklist.pop
          current_nfa_states = state_map[current_name]

          @alphabet.to_a.sort.each do |event|
            # Collect all NFA states reachable via this event
            next_nfa = Set.new
            current_nfa_states.each do |nfa_state|
              targets = @transitions[[nfa_state, event]] || Set.new
              next_nfa.merge(targets)
            end

            # Epsilon closure of the result
            next_closure = epsilon_closure(next_nfa)

            # Dead state -- no transition (DFA will be incomplete)
            next if next_closure.empty?

            next_name = state_set_name(next_closure)

            # Record this DFA transition
            dfa_transitions[[current_name, event]] = next_name

            # If this is a new DFA state, add it to the worklist
            unless dfa_states.include?(next_name)
              dfa_states.add(next_name)
              state_map[next_name] = next_closure
              worklist << next_name

              # Check if accepting
              if !(next_closure & @accepting).empty?
                dfa_accepting.add(next_name)
              end
            end
          end
        end

        DFA.new(
          states: dfa_states,
          alphabet: @alphabet.dup,
          transitions: dfa_transitions,
          initial: dfa_start,
          accepting: dfa_accepting
        )
      end

      # === Visualization ===

      # Return a Graphviz DOT representation of this NFA.
      #
      # Epsilon transitions are labeled with the epsilon character.
      # Non-deterministic transitions (multiple targets) produce multiple
      # edges from the same source.
      #
      # @return [String] A string in DOT format.
      def to_dot
        lines = []
        lines << "digraph NFA {"
        lines << "    rankdir=LR;"
        lines << ""

        # Start arrow
        lines << "    __start [shape=point, width=0.2];"
        lines << "    __start -> \"#{@initial}\";"
        lines << ""

        # State shapes
        @states.to_a.sort.each do |state|
          shape = @accepting.include?(state) ? "doublecircle" : "circle"
          lines << "    \"#{state}\" [shape=#{shape}];"
        end
        lines << ""

        # Transitions -- group by (source, target) to combine labels
        edge_labels = {}
        @transitions.sort_by { |(s, e), _| [s, e] }.each do |(source, event), targets|
          label = (event == EPSILON) ? "\u03B5" : event
          targets.to_a.sort.each do |target|
            key = [source, target]
            edge_labels[key] ||= []
            edge_labels[key] << label
          end
        end

        edge_labels.sort.each do |(source, target), labels|
          label = labels.join(", ")
          lines << "    \"#{source}\" -> \"#{target}\" [label=\"#{label}\"];"
        end

        lines << "}"
        lines.join("\n")
      end

      # Return a readable representation of the NFA.
      #
      # @return [String]
      def inspect
        "NFA(states=#{@states.to_a.sort}, " \
          "alphabet=#{@alphabet.to_a.sort}, " \
          "initial='#{@initial}', " \
          "accepting=#{@accepting.to_a.sort}, " \
          "current=#{@current_states.to_a.sort})"
      end

      alias_method :to_s, :inspect

      private

      # Convert a Set of state names to a DFA state name.
      #
      # The name is deterministic: sorted state names joined with commas
      # and wrapped in braces.
      #
      # Example:
      #   Set["q0", "q2", "q1"] -> "{q0,q1,q2}"
      #
      # @param state_set [Set<String>] A set of NFA state names.
      # @return [String] A canonical name for this set of states.
      def state_set_name(state_set)
        "{#{state_set.to_a.sort.join(",")}}"
      end
    end
  end
end
