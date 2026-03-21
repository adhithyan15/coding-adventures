defmodule CodingAdventures.StateMachine.Types do
  @moduledoc """
  Core types shared by all state machine implementations.

  ## The Building Blocks

  Every state machine — whether it is a simple traffic light controller or
  a complex HTML tokenizer — is built from the same fundamental concepts:

  - **State**: where the machine is right now (e.g., "locked", "red", "q0")
  - **Event**: what input the machine just received (e.g., "coin", "timer", "a")
  - **Transition**: the rule that says "in state X, on event Y, go to state Z"
  - **TransitionRecord**: a logged entry capturing one step of execution

  These types are deliberately simple — strings and structs. This makes
  state machines easy to define, serialize, and visualize. There are no
  behaviours or complex hierarchies here.

  ## Type Aliases

  States and events are just strings. In Elixir we use `@type` annotations
  for clarity — when you see `state()` in a typespec, you know it is a
  state name, not just any arbitrary string.

  Why strings and not atoms? Strings are safer for user-supplied input
  (atoms are not garbage collected), simpler to serialize, and flexible
  enough for any naming convention. For the same reason, the grammar-tools
  package uses strings for token names and grammar rule names.
  """

  @typedoc "A named state in a state machine. Examples: \"locked\", \"q0\", \"SNT\"."
  @type state :: String.t()

  @typedoc "An input symbol that triggers a transition. Examples: \"coin\", \"a\", \"taken\"."
  @type event :: String.t()

  @typedoc """
  One step in a state machine's execution trace.

  Every time a machine processes an input and transitions from one state
  to another, a TransitionRecord is created. This gives complete
  visibility into the machine's execution history.

  ## Why trace everything?

  In the coding-adventures philosophy, we want to be able to trace any
  computation all the way down to the logic gates that implement it.
  TransitionRecords are the state machine layer's contribution to that
  trace: they record exactly what happened, when, and why.

  You can replay an execution by walking through its list of
  TransitionRecords. You can verify correctness by checking that the
  source of each record matches the target of the previous one. You
  can visualize the execution path on a state diagram by highlighting
  the edges that were traversed.

  ## Fields

  - `source` — the state before the transition
  - `event` — the input that triggered it (nil for epsilon transitions)
  - `target` — the state after the transition
  - `action_name` — the name of the action that fired, if any

  ## Examples

      %TransitionRecord{source: "locked", event: "coin", target: "unlocked", action_name: nil}
      # "The machine was in 'locked', received 'coin', moved to 'unlocked'"

      %TransitionRecord{source: "q0", event: nil, target: "q1", action_name: nil}
      # "Epsilon transition from q0 to q1 (no input consumed)"
  """
  defstruct [:source, :event, :target, :action_name]

  @type t :: %__MODULE__{
          source: state(),
          event: event() | nil,
          target: state(),
          action_name: String.t() | nil
        }
end
