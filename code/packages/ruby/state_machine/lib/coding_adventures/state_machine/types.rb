# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Core types shared by all state machine implementations.
# ---------------------------------------------------------------------------
#
# === The Building Blocks ===
#
# Every state machine -- whether it is a simple traffic light controller or
# a complex HTML tokenizer -- is built from the same fundamental concepts:
#
# - **State**: where the machine is right now (e.g., "locked", "red", "q0")
# - **Event**: what input the machine just received (e.g., "coin", "timer", "a")
# - **Transition**: the rule that says "in state X, on event Y, go to state Z"
# - **Action**: an optional side effect that fires when a transition occurs
# - **TransitionRecord**: a logged entry capturing one step of execution
#
# These types are deliberately simple -- strings and Structs. This makes
# state machines easy to define, serialize, and visualize. There are no
# abstract base classes or complex hierarchies here.
#
# === Type Conventions ===
#
# States and events are just strings. We use the same convention as the
# Python implementation -- plain strings for maximum simplicity. You can
# define a state machine in one line without first declaring an enum class.
#
# Actions are callable objects (Procs or lambdas) that receive three
# arguments: (source_state, event, target_state). They are fire-and-forget
# side effects -- logging, incrementing counters, emitting tokens, etc.
# The state machine itself does not depend on action return values.
#
# === Ruby Implementation Notes ===
#
# We use Ruby's Struct class for TransitionRecord because it gives us:
# - Named fields with positional or keyword construction
# - Automatic equality comparison (==)
# - A readable #inspect / #to_s
# - Immutability when frozen
# - Value semantics (two records with the same fields are equal)
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # One step in a state machine's execution trace.
    #
    # Every time a machine processes an input and transitions from one state
    # to another, a TransitionRecord is created. This gives complete
    # visibility into the machine's execution history.
    #
    # === Why trace everything? ===
    #
    # In the coding-adventures philosophy, we want to be able to trace any
    # computation all the way down to the logic gates that implement it.
    # TransitionRecords are the state machine layer's contribution to that
    # trace: they record exactly what happened, when, and why.
    #
    # You can replay an execution by walking through its list of
    # TransitionRecords. You can verify correctness by checking that the
    # source of each record matches the target of the previous one. You
    # can visualize the execution path on a state diagram by highlighting
    # the edges that were traversed.
    #
    # === Fields ===
    #
    # - source: the state before the transition
    # - event: the input that triggered it (nil for epsilon transitions)
    # - target: the state after the transition
    # - action_name: the name of the action that fired, if any
    #
    # Example:
    #   TransitionRecord.new("locked", "coin", "unlocked", nil)
    #   # "The machine was in 'locked', received 'coin', moved to 'unlocked'"
    #
    #   TransitionRecord.new("q0", nil, "q1", nil)
    #   # "Epsilon transition from q0 to q1 (no input consumed)"
    #
    TransitionRecord = Struct.new(:source, :event, :target, :action_name) do
      # Override to_s for readable output in traces and debugging.
      #
      # Example:
      #   record.to_s  # => "locked --coin--> unlocked"
      #   record.to_s  # => "q0 --epsilon--> q1"
      def to_s
        event_label = event.nil? ? "epsilon" : event
        "#{source} --#{event_label}--> #{target}"
      end
    end
  end
end
