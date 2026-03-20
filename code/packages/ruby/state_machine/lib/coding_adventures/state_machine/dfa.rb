# frozen_string_literal: true

require "set"

# ---------------------------------------------------------------------------
# Deterministic Finite Automaton (DFA) -- the workhorse of state machines.
# ---------------------------------------------------------------------------
#
# === What is a DFA? ===
#
# A DFA is the simplest kind of state machine. It has a fixed set of states,
# reads input symbols one at a time, and follows exactly one transition for
# each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
#
# Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
#
#     Q      = a finite set of states
#     Sigma  = a finite set of input symbols (the "alphabet")
#     delta  = a transition function: Q x Sigma -> Q
#     q0     = the initial state (q0 in Q)
#     F      = a set of accepting/final states (F is a subset of Q)
#
# === Why "deterministic"? ===
#
# "Deterministic" means there is exactly ONE next state for every (state, input)
# combination. Given the same starting state and the same input sequence, a DFA
# always follows the same path and reaches the same final state. This makes DFAs
# predictable, efficient, and easy to implement in hardware -- which is why they
# appear everywhere from CPU branch predictors to network protocol handlers.
#
# === Example: a turnstile ===
#
# A turnstile at a subway station has two states: locked and unlocked.
# Insert a coin -> it unlocks. Push the arm -> it locks.
#
#     States:      {locked, unlocked}
#     Alphabet:    {coin, push}
#     Transitions: (locked, coin)    -> unlocked
#                  (locked, push)    -> locked
#                  (unlocked, coin)  -> unlocked
#                  (unlocked, push)  -> locked
#     Initial:     locked
#     Accepting:   {unlocked}
#
# This DFA answers the question: "after this sequence of coin/push events,
# is the turnstile unlocked?"
#
# === Connection to existing code ===
#
# The 2-bit branch predictor in the branch-predictor package (D02) is a DFA:
#
#     States:      {SNT, WNT, WT, ST}  (strongly/weakly not-taken/taken)
#     Alphabet:    {taken, not_taken}
#     Transitions: defined by the saturating counter logic
#     Initial:     WNT
#     Accepting:   {WT, ST}  (states that predict "taken")
#
# The CPU pipeline (D04) is a linear DFA: FETCH -> DECODE -> EXECUTE -> repeat.
# The lexer's character dispatch is an implicit DFA where character classes
# determine transitions.
#
# === Ruby Implementation Notes ===
#
# - States and events are strings (just like the Python version).
# - Transitions are stored as a Hash with [state, event] array keys.
# - We use Ruby's Set class for states, alphabet, and accepting sets.
# - Actions are callable objects (Procs, lambdas, or anything responding to #call).
# - All validation happens eagerly in initialize ("fail fast" principle).
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    class DFA
      # The finite set of states (frozen Set of strings).
      attr_reader :states

      # The finite set of input symbols (frozen Set of strings).
      attr_reader :alphabet

      # The initial state (string).
      attr_reader :initial

      # The set of accepting/final states (frozen Set of strings).
      attr_reader :accepting

      # The state the machine is currently in (string).
      attr_reader :current_state

      # === Construction ===
      #
      # We validate all inputs eagerly in initialize so that errors are caught
      # at definition time, not at runtime when the machine processes its
      # first input. This is the "fail fast" principle.
      #
      # @param states [Set<String>] The finite set of states. Must be non-empty.
      # @param alphabet [Set<String>] The finite set of input symbols. Must be non-empty.
      # @param transitions [Hash<Array(String, String), String>] Mapping from
      #   [state, event] to target state. Every target must be in +states+.
      # @param initial [String] The starting state. Must be in +states+.
      # @param accepting [Set<String>] The set of accepting/final states.
      # @param actions [Hash<Array(String, String), #call>, nil] Optional mapping
      #   from [state, event] to a callable that fires when that transition occurs.
      # @raise [ArgumentError] If any validation check fails.
      def initialize(states:, alphabet:, transitions:, initial:, accepting:, actions: nil)
        # --- Validate states ---
        raise ArgumentError, "States set must be non-empty" if states.empty?

        # --- Validate initial state ---
        unless states.include?(initial)
          raise ArgumentError,
            "Initial state '#{initial}' is not in the states set #{states.to_a.sort}"
        end

        # --- Validate accepting states ---
        invalid_accepting = accepting - states
        unless invalid_accepting.empty?
          raise ArgumentError,
            "Accepting states #{invalid_accepting.to_a.sort} are not in " \
            "the states set #{states.to_a.sort}"
        end

        # --- Validate transitions ---
        #
        # Every transition must go FROM a known state ON a known event
        # TO a known state. We check all three.
        transitions.each do |(source, event), target|
          unless states.include?(source)
            raise ArgumentError,
              "Transition source '#{source}' is not in the states set"
          end
          unless alphabet.include?(event)
            raise ArgumentError,
              "Transition event '#{event}' is not in the alphabet #{alphabet.to_a.sort}"
          end
          unless states.include?(target)
            raise ArgumentError,
              "Transition target '#{target}' (from " \
              "(#{source}, #{event})) is not in the states set"
          end
        end

        # --- Validate actions ---
        if actions
          actions.each_key do |(source, event)|
            unless transitions.key?([source, event])
              raise ArgumentError,
                "Action defined for (#{source}, #{event}) but no " \
                "transition exists for that pair"
            end
          end
        end

        # --- Store the 5-tuple + extras ---
        @states = states.to_set.freeze
        @alphabet = alphabet.to_set.freeze
        @transitions = transitions.dup.freeze
        @initial = initial.freeze
        @accepting = accepting.to_set.freeze
        @actions = (actions || {}).dup.freeze

        # --- Mutable execution state ---
        @current_state = initial
        @trace = []
      end

      # The transition function as a hash. Returns a copy so callers cannot
      # modify the internal transitions.
      #
      # @return [Hash<Array(String, String), String>]
      def transitions
        @transitions.dup
      end

      # The execution trace -- a list of all TransitionRecords taken so far.
      # Returns a copy so callers cannot modify the internal trace.
      #
      # @return [Array<TransitionRecord>]
      def trace
        @trace.dup
      end

      # === Processing ===

      # Process a single input event and return the new state.
      #
      # Looks up the transition for (current_state, event), moves to the
      # target state, executes the action (if defined), logs a
      # TransitionRecord, and returns the new current state.
      #
      # How it works step by step:
      #
      #   1. Check that the event is in the alphabet. If not, raise.
      #   2. Look up [current_state, event] in the transition table.
      #   3. If no transition exists, raise (this is a "stuck" DFA).
      #   4. If an action is registered for this transition, call it
      #      with (source, event, target).
      #   5. Append a TransitionRecord to the trace.
      #   6. Update current_state to the target.
      #   7. Return the new current_state.
      #
      # @param event [String] An input symbol from the alphabet.
      # @return [String] The new current state after the transition.
      # @raise [ArgumentError] If the event is not in the alphabet, or if no
      #   transition is defined for (current_state, event).
      def process(event)
        # Validate the event
        unless @alphabet.include?(event)
          raise ArgumentError,
            "Event '#{event}' is not in the alphabet #{@alphabet.to_a.sort}"
        end

        # Look up the transition
        key = [@current_state, event]
        unless @transitions.key?(key)
          raise ArgumentError,
            "No transition defined for (state='#{@current_state}', " \
            "event='#{event}')"
        end

        target = @transitions[key]

        # Execute the action if one exists
        action_name = nil
        if @actions.key?(key)
          action = @actions[key]
          action.call(@current_state, event, target)
          # Try to get a meaningful name for the action
          action_name = if action.respond_to?(:name) && !action.name.empty?
            action.name
          else
            action.to_s
          end
        end

        # Log the transition
        record = TransitionRecord.new(
          @current_state,
          event,
          target,
          action_name
        )
        @trace << record

        # Move to the new state
        @current_state = target
        target
      end

      # Process a sequence of inputs and return the trace.
      #
      # Each input is processed in order. The full trace of transitions
      # is returned. The machine's state is updated after each input.
      #
      # @param events [Array<String>] A list of input symbols.
      # @return [Array<TransitionRecord>] One record per input processed.
      def process_sequence(events)
        trace_start = @trace.length
        events.each { |event| process(event) }
        @trace[trace_start..]
      end

      # Check if the machine accepts the input sequence.
      #
      # Processes the entire sequence and returns true if the machine
      # ends in an accepting state.
      #
      # IMPORTANT: This method does NOT modify the machine's current state
      # or trace. It runs on a fresh copy starting from the initial state.
      #
      # The algorithm is simple:
      #   1. Start at the initial state (ignoring current_state).
      #   2. For each event, look up the transition.
      #   3. If no transition exists, return false immediately (graceful reject).
      #   4. After all events, return true if the final state is accepting.
      #
      # @param events [Array<String>] A list of input symbols.
      # @return [Boolean] True if the machine accepts the sequence.
      # @raise [ArgumentError] If any event is not in the alphabet.
      def accepts(events)
        # Run on a copy so we don't modify this machine's state
        state = @initial
        events.each do |event|
          unless @alphabet.include?(event)
            raise ArgumentError,
              "Event '#{event}' is not in the alphabet #{@alphabet.to_a.sort}"
          end
          key = [state, event]
          return false unless @transitions.key?(key)
          state = @transitions[key]
        end
        @accepting.include?(state)
      end

      # Reset the machine to its initial state and clear the trace.
      #
      # After reset, the machine is in the same state as when it was
      # first constructed -- as if no inputs had ever been processed.
      def reset
        @current_state = @initial
        @trace = []
      end

      # === Introspection ===
      #
      # These methods analyze the structure of the DFA itself, not its
      # execution. They answer questions like "is the DFA well-formed?"
      # and "which states can actually be reached?"

      # Return the set of states reachable from the initial state.
      #
      # Uses breadth-first search over the transition graph. A state is
      # reachable if there exists any sequence of inputs that leads from
      # the initial state to that state.
      #
      # States that are defined but not reachable are "dead weight" --
      # they can never be entered and can be safely removed during
      # minimization.
      #
      # BFS algorithm:
      #   1. Start with a queue containing just the initial state.
      #   2. Pop a state from the queue.
      #   3. For each transition FROM that state, add the target to the
      #      queue if we haven't visited it yet.
      #   4. Repeat until the queue is empty.
      #   5. Return all visited states.
      #
      # @return [Set<String>] The set of reachable state names.
      def reachable_states
        visited = Set.new
        queue = [@initial]

        until queue.empty?
          state = queue.shift
          next if visited.include?(state)
          visited.add(state)

          # Find all states reachable from this one via any input
          @transitions.each do |(source, _event), target|
            if source == state && !visited.include?(target)
              queue << target
            end
          end
        end

        visited.freeze
      end

      # Check if a transition is defined for every (state, input) pair.
      #
      # A complete DFA never gets "stuck" -- every state handles every
      # input. Textbook DFAs are usually complete (missing transitions
      # go to an explicit "dead" or "trap" state). Practical DFAs often
      # omit transitions to save space, treating missing transitions as
      # errors.
      #
      # @return [Boolean] True if every (state, event) pair has a defined transition.
      def complete?
        @states.each do |state|
          @alphabet.each do |event|
            return false unless @transitions.key?([state, event])
          end
        end
        true
      end

      # Check for common issues and return a list of warnings.
      #
      # Checks performed:
      # - Unreachable states (defined but never entered)
      # - Missing transitions (incomplete DFA)
      # - Accepting states that are unreachable
      #
      # @return [Array<String>] Warning messages. Empty if no issues found.
      def validate
        warnings = []

        # Check for unreachable states
        reachable = reachable_states
        unreachable = @states - reachable
        unless unreachable.empty?
          warnings << "Unreachable states: #{unreachable.to_a.sort}"
        end

        # Check for unreachable accepting states
        unreachable_accepting = @accepting - reachable
        unless unreachable_accepting.empty?
          warnings << "Unreachable accepting states: #{unreachable_accepting.to_a.sort}"
        end

        # Check for missing transitions
        missing = []
        @states.to_a.sort.each do |state|
          @alphabet.to_a.sort.each do |event|
            unless @transitions.key?([state, event])
              missing << "(#{state}, #{event})"
            end
          end
        end
        unless missing.empty?
          warnings << "Missing transitions: #{missing.join(", ")}"
        end

        warnings
      end

      # === Visualization ===

      # Return a Graphviz DOT representation of this DFA.
      #
      # Accepting states are drawn as double circles (doublecircle shape).
      # The initial state has an invisible node pointing to it (the
      # standard convention for marking the start state in automata
      # diagrams).
      #
      # The output can be rendered with:
      #     dot -Tpng machine.dot -o machine.png
      #
      # @return [String] A string in DOT format.
      def to_dot
        lines = []
        lines << "digraph DFA {"
        lines << "    rankdir=LR;"
        lines << ""

        # Invisible start node pointing to initial state
        lines << "    __start [shape=point, width=0.2];"
        lines << "    __start -> \"#{@initial}\";"
        lines << ""

        # Accepting states get double circles
        @states.to_a.sort.each do |state|
          shape = @accepting.include?(state) ? "doublecircle" : "circle"
          lines << "    \"#{state}\" [shape=#{shape}];"
        end
        lines << ""

        # Transitions as labeled edges
        # Group transitions with same source and target to combine labels
        edge_labels = {}
        @transitions.sort.each do |(source, event), target|
          key = [source, target]
          edge_labels[key] ||= []
          edge_labels[key] << event
        end

        edge_labels.sort.each do |(source, target), labels|
          label = labels.sort.join(", ")
          lines << "    \"#{source}\" -> \"#{target}\" [label=\"#{label}\"];"
        end

        lines << "}"
        lines.join("\n")
      end

      # Return an ASCII transition table.
      #
      # Example output for the turnstile:
      #
      #           | coin     | push
      #   --------+----------+----------
      #   locked  | unlocked | locked
      #   unlocked| unlocked | locked
      #
      # Accepting states are marked with (*). The initial state is
      # marked with (>).
      #
      # @return [String] A formatted ASCII table string.
      def to_ascii
        sorted_events = @alphabet.to_a.sort
        sorted_states = @states.to_a.sort

        # Calculate column widths
        state_width = sorted_states.map { |s| s.length + 4 }.max # +4 for markers
        event_width = [
          sorted_events.map(&:length).max || 0,
          sorted_states.flat_map { |s|
            sorted_events.map { |e| (@transitions[[s, e]] || "\u2014").length }
          }.max || 0
        ].max
        event_width = [event_width, 5].max # minimum column width

        # Header row
        header = " " * state_width + "\u2502"
        sorted_events.each do |event|
          header += " #{event.ljust(event_width)} \u2502"
        end
        lines = [header]

        # Separator
        sep = "\u2500" * state_width + "\u253C"
        sorted_events.each do
          sep += "\u2500" * (event_width + 2) + "\u253C"
        end
        sep = sep[0..-2] # remove trailing cross
        lines << sep

        # Data rows
        sorted_states.each do |state|
          markers = ""
          markers += ">" if state == @initial
          markers += "*" if @accepting.include?(state)
          label = markers.empty? ? "  #{state}" : "#{markers} #{state}"

          row = "#{label.ljust(state_width)}\u2502"
          sorted_events.each do |event|
            target = @transitions[[state, event]] || "\u2014"
            row += " #{target.ljust(event_width)} \u2502"
          end
          lines << row
        end

        lines.join("\n")
      end

      # Return the transition table as an array of rows.
      #
      # First row is the header: ["State", event1, event2, ...].
      # Subsequent rows: [state_name, target1, target2, ...].
      # Missing transitions are represented as a dash character.
      #
      # @return [Array<Array<String>>]
      def to_table
        sorted_events = @alphabet.to_a.sort
        sorted_states = @states.to_a.sort

        rows = []
        rows << ["State", *sorted_events]

        sorted_states.each do |state|
          row = [state]
          sorted_events.each do |event|
            target = @transitions[[state, event]] || "\u2014"
            row << target
          end
          rows << row
        end

        rows
      end

      # Return a readable representation of the DFA.
      #
      # Includes the key components: states, alphabet, initial state,
      # accepting states, and current state. Useful for debugging and
      # REPL exploration.
      #
      # @return [String]
      def inspect
        "DFA(states=#{@states.to_a.sort}, " \
          "alphabet=#{@alphabet.to_a.sort}, " \
          "initial='#{@initial}', " \
          "accepting=#{@accepting.to_a.sort}, " \
          "current='#{@current_state}')"
      end

      alias_method :to_s, :inspect
    end
  end
end
