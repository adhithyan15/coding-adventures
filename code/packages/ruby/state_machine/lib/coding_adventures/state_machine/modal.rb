# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Modal State Machine -- multiple sub-machines with mode switching.
# ---------------------------------------------------------------------------
#
# === What is a Modal State Machine? ===
#
# A modal state machine is a collection of named sub-machines (modes), each
# a DFA, with transitions that switch between them. When a mode switch
# occurs, the active sub-machine changes.
#
# Think of it like a text editor with Normal, Insert, and Visual modes. Each
# mode handles keystrokes differently, and certain keys switch between modes.
#
# === Why modal machines matter ===
#
# The most important use case is **context-sensitive tokenization**. Consider
# HTML: the characters `p > .foo { color: red; }` mean completely different
# things depending on whether they appear inside a `<style>` tag (CSS) or
# in normal text. A single set of token rules cannot handle both contexts.
#
# A modal state machine solves this: the HTML tokenizer has modes like
# DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
# with its own token rules. Certain tokens (like seeing `<style>`) trigger
# a mode switch.
#
# This is how real browser engines tokenize HTML, and it is the key
# abstraction that the grammar-tools lexer needs to support HTML, Markdown,
# and other context-sensitive languages.
#
# === Connection to the Chomsky Hierarchy ===
#
# A single DFA recognizes regular languages (Type 3). A modal state machine
# is more powerful: it can track context (which mode am I in?) and switch
# rules accordingly. This moves us toward context-sensitive languages
# (Type 1), though a modal machine is still not as powerful as a full
# linear-bounded automaton.
#
# In practice, modal machines + pushdown automata cover the vast majority
# of real-world parsing needs.
#
# === Ruby Implementation Notes ===
#
# - Modes are stored in a Hash mapping mode name (String) to DFA.
# - Mode transitions map [current_mode, trigger] to target mode name.
# - When a mode switch occurs, the target mode's DFA is reset.
# - ModeTransitionRecord is a Struct capturing each mode switch event.
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # Record of a mode switch event.
    #
    # Captures which mode we switched from and to, and what triggered it.
    # Useful for debugging and tracing the tokenizer's behavior.
    #
    # Example:
    #   ModeTransitionRecord.new("data", "enter_tag", "tag")
    #   # "Switched from data mode to tag mode, triggered by enter_tag"
    ModeTransitionRecord = Struct.new(:from_mode, :trigger, :to_mode)

    class ModalStateMachine
      # The name of the currently active mode (String).
      attr_reader :current_mode

      # Create a new Modal State Machine.
      #
      # @param modes [Hash<String, DFA>] A hash mapping mode names to DFA sub-machines.
      # @param mode_transitions [Hash<Array(String, String), String>] Mapping from
      #   [current_mode, trigger] to the name of the mode to switch to.
      # @param initial_mode [String] The name of the starting mode.
      # @raise [ArgumentError] If validation fails.
      def initialize(modes:, mode_transitions:, initial_mode:)
        if modes.empty?
          raise ArgumentError, "At least one mode must be provided"
        end

        unless modes.key?(initial_mode)
          raise ArgumentError,
            "Initial mode '#{initial_mode}' is not in the modes dict"
        end

        # Validate mode transitions
        mode_transitions.each do |(from_mode, _trigger), to_mode|
          unless modes.key?(from_mode)
            raise ArgumentError,
              "Mode transition source '#{from_mode}' is not a valid mode"
          end
          unless modes.key?(to_mode)
            raise ArgumentError,
              "Mode transition target '#{to_mode}' is not a valid mode"
          end
        end

        @modes = modes.dup.freeze
        @mode_transitions = mode_transitions.dup.freeze
        @initial_mode = initial_mode.freeze
        @current_mode = initial_mode
        @mode_trace = []

        # --- Build internal graph of mode transitions ---
        #
        # The mode graph captures the structure of mode switching: each mode
        # is a node, and each mode transition [mode, trigger] => target_mode
        # becomes a labeled edge with the trigger as the label. This makes
        # the mode transition topology available for structural queries
        # (e.g., "which modes are reachable from the initial mode?").
        @mode_graph = CodingAdventures::DirectedGraph::LabeledGraph.new
        modes.each_key { |mode| @mode_graph.add_node(mode) }
        mode_transitions.each do |(mode, trigger), target_mode|
          @mode_graph.add_edge(mode, target_mode, trigger)
        end
      end

      # The DFA for the current mode.
      #
      # @return [DFA]
      def active_machine
        @modes[@current_mode]
      end

      # All modes and their DFAs.
      #
      # @return [Hash<String, DFA>]
      def modes
        @modes.dup
      end

      # The history of mode switches. Returns a copy.
      #
      # @return [Array<ModeTransitionRecord>]
      def mode_trace
        @mode_trace.dup
      end

      # === Processing ===

      # Process an input event in the current mode's DFA.
      #
      # Delegates to the active DFA's process() method. The event must
      # be valid for the current mode's alphabet.
      #
      # @param event [String] An input symbol for the current mode's DFA.
      # @return [String] The new state of the active DFA.
      # @raise [ArgumentError] If the event is invalid for the current mode.
      def process(event)
        @modes[@current_mode].process(event)
      end

      # Switch to a different mode based on a trigger event.
      #
      # Looks up [current_mode, trigger] in the mode transitions.
      # If found, switches to the target mode and resets its DFA
      # to the initial state.
      #
      # === Why reset the target DFA? ===
      #
      # When entering a new mode, we want a clean slate. The previous
      # state of that mode's DFA is irrelevant -- we're starting fresh
      # in that context. Think of it like opening a new tab in a browser:
      # you start at the home page, not where you left off last time.
      #
      # @param trigger [String] The event that triggers the mode switch.
      # @return [String] The name of the new mode.
      # @raise [ArgumentError] If no mode transition exists for this trigger.
      def switch_mode(trigger)
        key = [@current_mode, trigger]
        unless @mode_transitions.key?(key)
          raise ArgumentError,
            "No mode transition for (mode='#{@current_mode}', " \
            "trigger='#{trigger}')"
        end

        new_mode = @mode_transitions[key]
        old_mode = @current_mode

        # Reset the target mode's DFA to its initial state
        @modes[new_mode].reset

        # Record the switch
        @mode_trace << ModeTransitionRecord.new(old_mode, trigger, new_mode)

        @current_mode = new_mode
        new_mode
      end

      # Reset to initial mode and reset all sub-machines.
      #
      # After reset, the modal machine is in the same state as when
      # it was first constructed -- initial mode active, all DFAs at
      # their initial states, mode trace cleared.
      def reset
        @current_mode = @initial_mode
        @mode_trace = []
        @modes.each_value(&:reset)
      end

      # Return a readable representation.
      #
      # @return [String]
      def inspect
        "ModalStateMachine(modes=#{@modes.keys.sort}, " \
          "current_mode='#{@current_mode}')"
      end

      alias_method :to_s, :inspect
    end
  end
end
