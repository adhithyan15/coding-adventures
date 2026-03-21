defmodule CodingAdventures.StateMachine.DFA do
  @moduledoc """
  Deterministic Finite Automaton (DFA) — the workhorse of state machines.

  ## What is a DFA?

  A DFA is the simplest kind of state machine. It has a fixed set of states,
  reads input symbols one at a time, and follows exactly one transition for
  each (state, input) pair. There is no ambiguity, no guessing, no backtracking.

  Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):

      Q  = a finite set of states
      Sigma = a finite set of input symbols (the "alphabet")
      delta = a transition function: Q x Sigma -> Q
      q0 = the initial state (q0 in Q)
      F  = a set of accepting/final states (F subset of Q)

  ## Why "deterministic"?

  "Deterministic" means there is exactly ONE next state for every (state, input)
  combination. Given the same starting state and the same input sequence, a DFA
  always follows the same path and reaches the same final state. This makes DFAs
  predictable, efficient, and easy to implement in hardware — which is why they
  appear everywhere from CPU branch predictors to network protocol handlers.

  ## Example: a turnstile

  A turnstile at a subway station has two states: locked and unlocked.
  Insert a coin -> it unlocks. Push the arm -> it locks.

      States:      MapSet.new(["locked", "unlocked"])
      Alphabet:    MapSet.new(["coin", "push"])
      Transitions: %{{"locked", "coin"} => "unlocked",
                     {"locked", "push"} => "locked",
                     {"unlocked", "coin"} => "unlocked",
                     {"unlocked", "push"} => "locked"}
      Initial:     "locked"
      Accepting:   MapSet.new(["unlocked"])

  This DFA answers the question: "after this sequence of coin/push events,
  is the turnstile unlocked?"

  ## Immutability in Elixir

  Because Elixir data is immutable, every operation (process, reset) returns
  a NEW DFA struct rather than mutating in place. This is actually a perfect
  fit for automata theory, where each step produces a new "configuration."
  The `accepts?/2` function is naturally side-effect-free since nothing is
  ever mutated.
  """

  alias CodingAdventures.StateMachine.Types, as: TransitionRecord
  alias CodingAdventures.DirectedGraph.LabeledGraph

  defstruct [
    :states,
    :alphabet,
    :transitions,
    :initial,
    :accepting,
    :current,
    :trace,
    :graph
  ]

  @type t :: %__MODULE__{
          states: MapSet.t(String.t()),
          alphabet: MapSet.t(String.t()),
          transitions: %{{String.t(), String.t()} => String.t()},
          initial: String.t(),
          accepting: MapSet.t(String.t()),
          current: String.t(),
          trace: [TransitionRecord.t()],
          graph: LabeledGraph.t()
        }

  @doc """
  Create a new DFA.

  Validates all inputs eagerly so that errors are caught at definition time,
  not at runtime when the machine processes its first input. This is the
  "fail fast" principle.

  ## Parameters

  - `states` — a MapSet of state names (must be non-empty)
  - `alphabet` — a MapSet of input symbols (must be non-empty)
  - `transitions` — a map from `{state, event}` tuples to target state strings.
    Every target must be in `states`. Not every `{state, event}` pair needs a
    transition — missing transitions cause errors at processing time.
  - `initial` — the starting state (must be in `states`)
  - `accepting` — the set of accepting/final states (must be a subset of `states`)

  ## Returns

  `{:ok, dfa}` on success, `{:error, reason}` on validation failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.DFA
      iex> {:ok, dfa} = DFA.new(
      ...>   MapSet.new(["locked", "unlocked"]),
      ...>   MapSet.new(["coin", "push"]),
      ...>   %{{"locked", "coin"} => "unlocked",
      ...>     {"locked", "push"} => "locked",
      ...>     {"unlocked", "coin"} => "unlocked",
      ...>     {"unlocked", "push"} => "locked"},
      ...>   "locked",
      ...>   MapSet.new(["unlocked"])
      ...> )
      iex> dfa.current
      "locked"
  """
  def new(states, alphabet, transitions, initial, accepting) do
    with :ok <- validate_non_empty(states, "States"),
         :ok <- validate_member(initial, states, "Initial state", "states set"),
         :ok <- validate_subset(accepting, states, "Accepting states", "states set"),
         :ok <- validate_transitions(transitions, states, alphabet) do
      # --- Build internal graph representation ---
      #
      # We maintain a LabeledGraph alongside the transitions map.
      # The map provides O(1) lookups for process() (the hot path).
      # The graph provides structural queries like reachable_states() via
      # transitive_closure, avoiding the need for hand-rolled BFS.
      #
      # Each state becomes a node. Each transition (source, event) -> target
      # becomes a labeled edge from source to target with the event as label.
      graph = LabeledGraph.new()

      graph =
        Enum.reduce(states, graph, fn state, acc ->
          {:ok, acc} = LabeledGraph.add_node(acc, state)
          acc
        end)

      graph =
        Enum.reduce(transitions, graph, fn {{source, event}, target}, acc ->
          {:ok, acc} = LabeledGraph.add_edge(acc, source, target, event)
          acc
        end)

      {:ok,
       %__MODULE__{
         states: states,
         alphabet: alphabet,
         transitions: transitions,
         initial: initial,
         accepting: accepting,
         current: initial,
         trace: [],
         graph: graph
       }}
    end
  end

  @doc """
  Process a single input event and return the updated DFA.

  Looks up the transition for `{current_state, event}`, moves to the
  target state, logs a TransitionRecord, and returns the new DFA.

  ## Parameters

  - `dfa` — the current DFA struct
  - `event` — an input symbol from the alphabet

  ## Returns

  `{:ok, new_dfa}` on success, `{:error, reason}` on failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.DFA
      iex> {:ok, dfa} = DFA.new(
      ...>   MapSet.new(["a", "b"]),
      ...>   MapSet.new(["x"]),
      ...>   %{{"a", "x"} => "b", {"b", "x"} => "a"},
      ...>   "a", MapSet.new(["b"])
      ...> )
      iex> {:ok, dfa} = DFA.process(dfa, "x")
      iex> dfa.current
      "b"
  """
  def process(%__MODULE__{} = dfa, event) do
    cond do
      not MapSet.member?(dfa.alphabet, event) ->
        {:error, "Event '#{event}' is not in the alphabet #{inspect(Enum.sort(dfa.alphabet))}"}

      not Map.has_key?(dfa.transitions, {dfa.current, event}) ->
        {:error, "No transition defined for (state='#{dfa.current}', event='#{event}')"}

      true ->
        target = Map.fetch!(dfa.transitions, {dfa.current, event})

        record = %TransitionRecord{
          source: dfa.current,
          event: event,
          target: target,
          action_name: nil
        }

        {:ok,
         %{dfa | current: target, trace: dfa.trace ++ [record]}}
    end
  end

  @doc """
  Process a sequence of inputs and return the updated DFA with a trace.

  Each input is processed in order. The full trace of transitions generated
  during this call is returned alongside the updated DFA.

  ## Parameters

  - `dfa` — the current DFA struct
  - `events` — a list of input symbols

  ## Returns

  `{:ok, new_dfa, trace}` on success, `{:error, reason}` on failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.DFA
      iex> {:ok, dfa} = DFA.new(
      ...>   MapSet.new(["a", "b"]),
      ...>   MapSet.new(["x"]),
      ...>   %{{"a", "x"} => "b", {"b", "x"} => "a"},
      ...>   "a", MapSet.new(["b"])
      ...> )
      iex> {:ok, new_dfa, trace} = DFA.process_sequence(dfa, ["x", "x", "x"])
      iex> length(trace)
      3
  """
  def process_sequence(%__MODULE__{} = dfa, events) do
    trace_start = length(dfa.trace)

    result =
      Enum.reduce_while(events, {:ok, dfa}, fn event, {:ok, acc} ->
        case process(acc, event) do
          {:ok, new_dfa} -> {:cont, {:ok, new_dfa}}
          {:error, reason} -> {:halt, {:error, reason}}
        end
      end)

    case result do
      {:ok, new_dfa} ->
        new_trace = Enum.drop(new_dfa.trace, trace_start)
        {:ok, new_dfa, new_trace}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Check if the machine accepts the input sequence.

  Processes the entire sequence from the initial state and returns true if the
  machine ends in an accepting state. This function does NOT modify the DFA —
  it operates on a fresh simulation starting from the initial state.

  In Elixir this is naturally side-effect-free since the DFA struct is immutable.

  ## Parameters

  - `dfa` — the DFA struct (used for its definition, not its current state)
  - `events` — a list of input symbols

  ## Returns

  `true` if the machine ends in an accepting state, `false` otherwise.
  Raises if an event is not in the alphabet.

  ## Examples

      iex> alias CodingAdventures.StateMachine.DFA
      iex> {:ok, dfa} = DFA.new(
      ...>   MapSet.new(["locked", "unlocked"]),
      ...>   MapSet.new(["coin", "push"]),
      ...>   %{{"locked", "coin"} => "unlocked",
      ...>     {"locked", "push"} => "locked",
      ...>     {"unlocked", "coin"} => "unlocked",
      ...>     {"unlocked", "push"} => "locked"},
      ...>   "locked",
      ...>   MapSet.new(["unlocked"])
      ...> )
      iex> DFA.accepts?(dfa, ["coin"])
      true
      iex> DFA.accepts?(dfa, ["coin", "push"])
      false
  """
  def accepts?(%__MODULE__{} = dfa, events) do
    # Simulate from initial state without modifying the DFA
    result =
      Enum.reduce_while(events, {:ok, dfa.initial}, fn event, {:ok, state} ->
        cond do
          not MapSet.member?(dfa.alphabet, event) ->
            {:halt, {:error, "Event '#{event}' is not in the alphabet"}}

          not Map.has_key?(dfa.transitions, {state, event}) ->
            {:halt, {:reject}}

          true ->
            {:cont, {:ok, Map.fetch!(dfa.transitions, {state, event})}}
        end
      end)

    case result do
      {:ok, final_state} -> MapSet.member?(dfa.accepting, final_state)
      {:reject} -> false
      {:error, reason} -> raise ArgumentError, reason
    end
  end

  @doc """
  Reset the machine to its initial state and clear the trace.

  Returns a new DFA in the same state as when it was first constructed —
  as if no inputs had ever been processed.

  ## Examples

      iex> alias CodingAdventures.StateMachine.DFA
      iex> {:ok, dfa} = DFA.new(
      ...>   MapSet.new(["a", "b"]),
      ...>   MapSet.new(["x"]),
      ...>   %{{"a", "x"} => "b", {"b", "x"} => "a"},
      ...>   "a", MapSet.new(["b"])
      ...> )
      iex> {:ok, dfa} = DFA.process(dfa, "x")
      iex> dfa.current
      "b"
      iex> dfa = DFA.reset(dfa)
      iex> dfa.current
      "a"
  """
  def reset(%__MODULE__{} = dfa) do
    %{dfa | current: dfa.initial, trace: []}
  end

  @doc """
  Return the set of states reachable from the initial state.

  Uses breadth-first search over the transition graph. A state is reachable
  if there exists any sequence of inputs that leads from the initial state
  to that state.

  States that are defined but not reachable are "dead weight" — they can
  never be entered and can be safely removed during minimization.

  ## Returns

  A MapSet of reachable state names.
  """
  def reachable_states(%__MODULE__{} = dfa) do
    # Delegates to the internal LabeledGraph's transitive_closure,
    # which performs a BFS over the transition graph. transitive_closure
    # returns all nodes reachable FROM the initial state (not including
    # the initial state itself), so we union it with the initial state.
    case LabeledGraph.transitive_closure(dfa.graph, dfa.initial) do
      {:ok, closure} -> MapSet.put(closure, dfa.initial)
      {:error, _} -> MapSet.new([dfa.initial])
    end
  end

  @doc """
  Check if a transition is defined for every (state, input) pair.

  A complete DFA never gets "stuck" — every state handles every input.
  Textbook DFAs are usually complete (missing transitions go to an explicit
  "dead" or "trap" state). Practical DFAs often omit transitions to save
  space, treating missing transitions as errors.

  ## Returns

  `true` if every `{state, event}` pair has a defined transition.
  """
  def complete?(%__MODULE__{} = dfa) do
    Enum.all?(dfa.states, fn state ->
      Enum.all?(dfa.alphabet, fn event ->
        Map.has_key?(dfa.transitions, {state, event})
      end)
    end)
  end

  @doc """
  Check for common issues and return a list of warnings.

  Checks performed:

  - Unreachable states (defined but never entered)
  - Missing transitions (incomplete DFA)
  - Accepting states that are unreachable

  ## Returns

  A list of warning message strings. Empty if no issues found.
  """
  def validate(%__MODULE__{} = dfa) do
    reachable = reachable_states(dfa)
    unreachable = MapSet.difference(dfa.states, reachable)
    unreachable_accepting = MapSet.difference(dfa.accepting, reachable)

    warnings = []

    warnings =
      if MapSet.size(unreachable) > 0 do
        warnings ++ ["Unreachable states: #{inspect(Enum.sort(unreachable))}"]
      else
        warnings
      end

    warnings =
      if MapSet.size(unreachable_accepting) > 0 do
        warnings ++
          ["Unreachable accepting states: #{inspect(Enum.sort(unreachable_accepting))}"]
      else
        warnings
      end

    missing =
      for state <- Enum.sort(dfa.states),
          event <- Enum.sort(dfa.alphabet),
          not Map.has_key?(dfa.transitions, {state, event}),
          do: "(#{state}, #{event})"

    warnings =
      if length(missing) > 0 do
        warnings ++ ["Missing transitions: #{Enum.join(missing, ", ")}"]
      else
        warnings
      end

    warnings
  end

  @doc """
  Return a Graphviz DOT representation of this DFA.

  Accepting states are drawn as double circles (doublecircle shape).
  The initial state has an invisible node pointing to it (the standard
  convention for marking the start state in automata diagrams).

  The output can be rendered with:

      dot -Tpng machine.dot -o machine.png

  ## Returns

  A string in DOT format.
  """
  def to_dot(%__MODULE__{} = dfa) do
    lines = [
      "digraph DFA {",
      "    rankdir=LR;",
      "",
      "    __start [shape=point, width=0.2];",
      "    __start -> \"#{dfa.initial}\";",
      ""
    ]

    # State shapes
    state_lines =
      dfa.states
      |> Enum.sort()
      |> Enum.map(fn state ->
        shape = if MapSet.member?(dfa.accepting, state), do: "doublecircle", else: "circle"
        "    \"#{state}\" [shape=#{shape}];"
      end)

    # Group transitions by (source, target) to combine labels
    edge_labels =
      dfa.transitions
      |> Enum.sort()
      |> Enum.reduce(%{}, fn {{source, event}, target}, acc ->
        Map.update(acc, {source, target}, [event], &(&1 ++ [event]))
      end)

    edge_lines =
      edge_labels
      |> Enum.sort()
      |> Enum.map(fn {{source, target}, labels} ->
        label = labels |> Enum.sort() |> Enum.join(", ")
        "    \"#{source}\" -> \"#{target}\" [label=\"#{label}\"];"
      end)

    lines = lines ++ state_lines ++ [""] ++ edge_lines ++ ["}"]
    Enum.join(lines, "\n")
  end

  @doc """
  Return an ASCII transition table.

  Accepting states are marked with (*). The initial state is marked with (>).

  ## Returns

  A formatted ASCII table string.

  ## Example output for the turnstile:

               | coin     | push
      ---------+----------+----------
      > locked | unlocked | locked
      *unlocked| unlocked | locked
  """
  def to_ascii(%__MODULE__{} = dfa) do
    sorted_events = Enum.sort(dfa.alphabet)
    sorted_states = Enum.sort(dfa.states)

    # Calculate column widths
    state_width =
      sorted_states
      |> Enum.map(fn s -> String.length(s) + 4 end)
      |> Enum.max(fn -> 8 end)

    event_width =
      Enum.max([
        5,
        sorted_events |> Enum.map(&String.length/1) |> Enum.max(fn -> 0 end),
        for(s <- sorted_states, e <- sorted_events, do: String.length(Map.get(dfa.transitions, {s, e}, "—")))
        |> Enum.max(fn -> 0 end)
      ])

    # Header
    header =
      String.pad_trailing("", state_width) <>
        "│" <>
        Enum.map_join(sorted_events, "", fn event ->
          " " <> String.pad_trailing(event, event_width) <> " │"
        end)

    # Separator
    sep =
      String.duplicate("─", state_width) <>
        "┼" <>
        Enum.map_join(sorted_events, "", fn _event ->
          String.duplicate("─", event_width + 2) <> "┼"
        end)

    sep = String.trim_trailing(sep, "┼")

    # Data rows
    rows =
      Enum.map(sorted_states, fn state ->
        markers =
          cond do
            state == dfa.initial and MapSet.member?(dfa.accepting, state) -> ">*"
            state == dfa.initial -> ">"
            MapSet.member?(dfa.accepting, state) -> "*"
            true -> ""
          end

        label =
          if markers == "",
            do: "  " <> state,
            else: markers <> " " <> state

        row =
          String.pad_trailing(label, state_width) <>
            "│" <>
            Enum.map_join(sorted_events, "", fn event ->
              target = Map.get(dfa.transitions, {state, event}, "—")
              " " <> String.pad_trailing(target, event_width) <> " │"
            end)

        row
      end)

    Enum.join([header, sep | rows], "\n")
  end

  @doc """
  Return the transition table as a list of rows.

  First row is the header: `["State", event1, event2, ...]`.
  Subsequent rows: `[state_name, target1, target2, ...]`.
  Missing transitions are represented as "—".

  ## Returns

  A list of string lists, suitable for formatting or export.
  """
  def to_table(%__MODULE__{} = dfa) do
    sorted_events = Enum.sort(dfa.alphabet)
    sorted_states = Enum.sort(dfa.states)

    header = ["State" | sorted_events]

    rows =
      Enum.map(sorted_states, fn state ->
        targets =
          Enum.map(sorted_events, fn event ->
            Map.get(dfa.transitions, {state, event}, "—")
          end)

        [state | targets]
      end)

    [header | rows]
  end

  # === Private validation helpers ===

  defp validate_non_empty(set, name) do
    if MapSet.size(set) == 0 do
      {:error, "#{name} set must be non-empty"}
    else
      :ok
    end
  end

  defp validate_member(value, set, value_name, set_name) do
    if MapSet.member?(set, value) do
      :ok
    else
      {:error, "#{value_name} '#{value}' is not in the #{set_name} #{inspect(Enum.sort(set))}"}
    end
  end

  defp validate_subset(subset, superset, subset_name, superset_name) do
    invalid = MapSet.difference(subset, superset)

    if MapSet.size(invalid) == 0 do
      :ok
    else
      {:error,
       "#{subset_name} #{inspect(Enum.sort(invalid))} are not in the #{superset_name} #{inspect(Enum.sort(superset))}"}
    end
  end

  defp validate_transitions(transitions, states, alphabet) do
    result =
      Enum.reduce_while(transitions, :ok, fn {{source, event}, target}, :ok ->
        cond do
          not MapSet.member?(states, source) ->
            {:halt, {:error, "Transition source '#{source}' is not in the states set"}}

          not MapSet.member?(alphabet, event) ->
            {:halt,
             {:error,
              "Transition event '#{event}' is not in the alphabet #{inspect(Enum.sort(alphabet))}"}}

          not MapSet.member?(states, target) ->
            {:halt,
             {:error,
              "Transition target '#{target}' (from (#{source}, #{event})) is not in the states set"}}

          true ->
            {:cont, :ok}
        end
      end)

    result
  end
end
