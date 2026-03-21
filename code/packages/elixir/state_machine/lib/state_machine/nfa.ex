defmodule CodingAdventures.StateMachine.NFA do
  @moduledoc """
  Non-deterministic Finite Automaton (NFA) with epsilon transitions.

  ## What is an NFA?

  An NFA relaxes the deterministic constraint of a DFA in two ways:

  1. **Multiple transitions:** A single (state, input) pair can lead to
     multiple target states. The machine explores all possibilities
     simultaneously — like spawning parallel universes.

  2. **Epsilon transitions:** The machine can jump to another state
     without consuming any input. These are "free" moves.

  ## The "parallel universes" model

  Think of an NFA as a machine that clones itself at every non-deterministic
  choice point. All clones run in parallel:

  - A clone that reaches a dead end (no transition) simply vanishes.
  - A clone that reaches an accepting state means the whole NFA accepts.
  - If ALL clones die without reaching an accepting state, the NFA rejects.

  The NFA accepts if there EXISTS at least one path through the machine
  that ends in an accepting state.

  ## Why NFAs matter

  NFAs are much easier to construct for certain problems. For example, "does
  this string contain the substring 'abc'?" is trivial as an NFA (just guess
  where 'abc' starts) but requires careful tracking as a DFA.

  Every NFA can be converted to an equivalent DFA via subset construction.
  This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
  efficient execution (O(1) per character).

  ## Formal definition

      NFA = (Q, Sigma, delta, q0, F)

      Q  = finite set of states
      Sigma = finite alphabet (input symbols)
      delta = transition function: Q x (Sigma union {epsilon}) -> P(Q)
             maps (state, input_or_epsilon) to a SET of states
      q0 = initial state
      F  = accepting states

  ## Elixir representation

  Transitions are stored as a map from `{state, event_or_epsilon}` to a
  `MapSet` of target states. The epsilon symbol is the empty string `""`.
  The NFA's current configuration is a `MapSet` of active states, reflecting
  the "parallel universes" model.
  """

  alias CodingAdventures.StateMachine.DFA

  @doc """
  The sentinel value for epsilon transitions.

  We use the empty string "" as the epsilon symbol. This works because
  no real input alphabet should contain the empty string — input symbols
  are always at least one character long.
  """
  @epsilon ""
  def epsilon, do: @epsilon

  defstruct [
    :states,
    :alphabet,
    :transitions,
    :initial,
    :accepting,
    :current
  ]

  @type t :: %__MODULE__{
          states: MapSet.t(String.t()),
          alphabet: MapSet.t(String.t()),
          transitions: %{{String.t(), String.t()} => MapSet.t(String.t())},
          initial: String.t(),
          accepting: MapSet.t(String.t()),
          current: MapSet.t(String.t())
        }

  @doc """
  Create a new NFA.

  ## Parameters

  - `states` — a MapSet of state names (must be non-empty)
  - `alphabet` — a MapSet of input symbols (must not contain empty string)
  - `transitions` — a map from `{state, event_or_epsilon}` to a MapSet of
    target states. Use `""` (epsilon) for epsilon transitions.
  - `initial` — the starting state (must be in `states`)
  - `accepting` — the set of accepting/final states

  ## Returns

  `{:ok, nfa}` on success, `{:error, reason}` on validation failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.NFA
      iex> {:ok, nfa} = NFA.new(
      ...>   MapSet.new(["q0", "q1", "q2"]),
      ...>   MapSet.new(["a", "b"]),
      ...>   %{{"q0", "a"} => MapSet.new(["q0", "q1"]),
      ...>     {"q0", "b"} => MapSet.new(["q0"]),
      ...>     {"q1", "b"} => MapSet.new(["q2"]),
      ...>     {"q2", "a"} => MapSet.new(["q2"]),
      ...>     {"q2", "b"} => MapSet.new(["q2"])},
      ...>   "q0",
      ...>   MapSet.new(["q2"])
      ...> )
      iex> MapSet.member?(nfa.current, "q0")
      true
  """
  def new(states, alphabet, transitions, initial, accepting) do
    with :ok <- validate_non_empty(states),
         :ok <- validate_no_epsilon_in_alphabet(alphabet),
         :ok <- validate_member(initial, states, "Initial state"),
         :ok <- validate_subset(accepting, states, "Accepting states"),
         :ok <- validate_transitions(transitions, states, alphabet) do
      nfa = %__MODULE__{
        states: states,
        alphabet: alphabet,
        transitions: transitions,
        initial: initial,
        accepting: accepting,
        current: MapSet.new()
      }

      # The NFA starts in the epsilon closure of the initial state
      current = epsilon_closure(nfa, MapSet.new([initial]))
      {:ok, %{nfa | current: current}}
    end
  end

  @doc """
  Compute the epsilon closure of a set of states.

  Starting from the given states, follow ALL epsilon transitions recursively.
  Return the full set of states reachable via zero or more epsilon transitions.

  This is the key operation that makes NFAs work: before and after processing
  each input, we expand to include all states reachable via "free" epsilon moves.

  The algorithm is a simple BFS over epsilon edges:

  1. Start with the input set
  2. For each state, find epsilon transitions
  3. Add all targets to the set
  4. Repeat until no new states are found

  ## Parameters

  - `nfa` — the NFA struct (used for its transitions)
  - `state_set` — a MapSet of starting states

  ## Returns

  A MapSet of all states reachable via epsilon transitions from any state
  in the input set.

  ## Examples

  Given: q0 --epsilon--> q1 --epsilon--> q2

      epsilon_closure(nfa, MapSet.new(["q0"])) == MapSet.new(["q0", "q1", "q2"])
  """
  def epsilon_closure(%__MODULE__{} = nfa, state_set) do
    do_epsilon_closure(MapSet.to_list(state_set), state_set, nfa.transitions)
  end

  defp do_epsilon_closure([], closure, _transitions), do: closure

  defp do_epsilon_closure([state | rest], closure, transitions) do
    targets = Map.get(transitions, {state, @epsilon}, MapSet.new())

    new_states =
      targets
      |> MapSet.difference(closure)
      |> MapSet.to_list()

    new_closure = Enum.reduce(new_states, closure, &MapSet.put(&2, &1))
    do_epsilon_closure(rest ++ new_states, new_closure, transitions)
  end

  @doc """
  Process one input event and return the updated NFA.

  For each current state, find all transitions on this event. Take the union
  of all target states, then compute the epsilon closure of the result.

  ## Parameters

  - `nfa` — the current NFA struct
  - `event` — an input symbol from the alphabet

  ## Returns

  `{:ok, new_nfa}` on success, `{:error, reason}` on failure.
  """
  def process(%__MODULE__{} = nfa, event) do
    if not MapSet.member?(nfa.alphabet, event) do
      {:error, "Event '#{event}' is not in the alphabet #{inspect(Enum.sort(nfa.alphabet))}"}
    else
      # Collect all target states from all current states
      next_states =
        nfa.current
        |> Enum.reduce(MapSet.new(), fn state, acc ->
          targets = Map.get(nfa.transitions, {state, event}, MapSet.new())
          MapSet.union(acc, targets)
        end)

      # Expand via epsilon closure
      new_current = epsilon_closure(nfa, next_states)
      {:ok, %{nfa | current: new_current}}
    end
  end

  @doc """
  Check if the NFA accepts the input sequence.

  The NFA accepts if, after processing all inputs, ANY of the current states
  is an accepting state.

  Does NOT modify the NFA — runs a fresh simulation from the initial state.

  ## Parameters

  - `nfa` — the NFA struct
  - `events` — a list of input symbols

  ## Returns

  `true` if the NFA accepts, `false` otherwise.
  """
  def accepts?(%__MODULE__{} = nfa, events) do
    # Simulate from initial state
    start = epsilon_closure(nfa, MapSet.new([nfa.initial]))

    result =
      Enum.reduce_while(events, start, fn event, current ->
        if not MapSet.member?(nfa.alphabet, event) do
          {:halt, {:error, "Event '#{event}' is not in the alphabet"}}
        else
          next =
            current
            |> Enum.reduce(MapSet.new(), fn state, acc ->
              targets = Map.get(nfa.transitions, {state, event}, MapSet.new())
              MapSet.union(acc, targets)
            end)

          next_with_epsilon = epsilon_closure(nfa, next)

          if MapSet.size(next_with_epsilon) == 0 do
            {:halt, MapSet.new()}
          else
            {:cont, next_with_epsilon}
          end
        end
      end)

    case result do
      {:error, reason} -> raise ArgumentError, reason
      final_states -> MapSet.size(MapSet.intersection(final_states, nfa.accepting)) > 0
    end
  end

  @doc """
  Reset the NFA to the initial state (with epsilon closure).

  Returns a new NFA struct with current set to the epsilon closure of
  the initial state.
  """
  def reset(%__MODULE__{} = nfa) do
    current = epsilon_closure(nfa, MapSet.new([nfa.initial]))
    %{nfa | current: current}
  end

  @doc """
  Convert this NFA to an equivalent DFA using subset construction.

  ## The Subset Construction Algorithm

  The key insight: if an NFA can be in states {q0, q1, q3} simultaneously,
  we create a single DFA state representing that entire set. The DFA's
  states are sets of NFA states.

  Algorithm:

  1. Start with d0 = epsilon-closure({q0})
  2. For each DFA state D and each input symbol a:
     - For each NFA state q in D, find delta(q, a)
     - Take the union of all targets
     - Compute epsilon-closure of the union
     - That is the new DFA state D'
  3. Repeat until no new DFA states are discovered
  4. A DFA state is accepting if it contains ANY NFA accepting state

  DFA state names are generated from sorted NFA state names:

      MapSet.new(["q0", "q1"]) -> "{q0,q1}"

  ## Returns

  `{:ok, dfa}` — a DFA struct that recognizes exactly the same language
  as this NFA.
  """
  def to_dfa(%__MODULE__{} = nfa) do
    # Step 1: initial DFA state = epsilon-closure of NFA initial state
    start_closure = epsilon_closure(nfa, MapSet.new([nfa.initial]))
    dfa_start = state_set_name(start_closure)

    # Track DFA states and transitions as we discover them
    dfa_states = MapSet.new([dfa_start])
    dfa_transitions = %{}
    dfa_accepting = if has_accepting?(start_closure, nfa.accepting), do: MapSet.new([dfa_start]), else: MapSet.new()

    # Map from DFA state name -> MapSet of NFA states
    state_map = %{dfa_start => start_closure}

    # BFS over DFA states
    {dfa_states, dfa_transitions, dfa_accepting, _state_map} =
      do_subset_construction(
        [dfa_start],
        dfa_states,
        dfa_transitions,
        dfa_accepting,
        state_map,
        nfa
      )

    DFA.new(
      dfa_states,
      nfa.alphabet,
      dfa_transitions,
      dfa_start,
      dfa_accepting
    )
  end

  defp do_subset_construction([], dfa_states, dfa_transitions, dfa_accepting, state_map, _nfa) do
    {dfa_states, dfa_transitions, dfa_accepting, state_map}
  end

  defp do_subset_construction(
         [current_name | rest],
         dfa_states,
         dfa_transitions,
         dfa_accepting,
         state_map,
         nfa
       ) do
    current_nfa_states = Map.fetch!(state_map, current_name)

    {new_worklist, dfa_states, dfa_transitions, dfa_accepting, state_map} =
      nfa.alphabet
      |> Enum.sort()
      |> Enum.reduce(
        {[], dfa_states, dfa_transitions, dfa_accepting, state_map},
        fn event, {worklist_acc, states_acc, trans_acc, accept_acc, map_acc} ->
          # Collect all NFA states reachable via this event
          next_nfa =
            current_nfa_states
            |> Enum.reduce(MapSet.new(), fn nfa_state, acc ->
              targets = Map.get(nfa.transitions, {nfa_state, event}, MapSet.new())
              MapSet.union(acc, targets)
            end)

          # Epsilon closure of the result
          next_closure = epsilon_closure(nfa, next_nfa)

          if MapSet.size(next_closure) == 0 do
            # Dead state — no transition
            {worklist_acc, states_acc, trans_acc, accept_acc, map_acc}
          else
            next_name = state_set_name(next_closure)
            trans_acc = Map.put(trans_acc, {current_name, event}, next_name)

            if MapSet.member?(states_acc, next_name) do
              {worklist_acc, states_acc, trans_acc, accept_acc, map_acc}
            else
              states_acc = MapSet.put(states_acc, next_name)
              map_acc = Map.put(map_acc, next_name, next_closure)

              accept_acc =
                if has_accepting?(next_closure, nfa.accepting),
                  do: MapSet.put(accept_acc, next_name),
                  else: accept_acc

              {worklist_acc ++ [next_name], states_acc, trans_acc, accept_acc, map_acc}
            end
          end
        end
      )

    do_subset_construction(
      rest ++ new_worklist,
      dfa_states,
      dfa_transitions,
      dfa_accepting,
      state_map,
      nfa
    )
  end

  defp has_accepting?(state_set, accepting) do
    MapSet.size(MapSet.intersection(state_set, accepting)) > 0
  end

  @doc """
  Return a Graphviz DOT representation of this NFA.

  Epsilon transitions are labeled with the epsilon symbol. Non-deterministic
  transitions (multiple targets) produce multiple edges from the same source.

  ## Returns

  A string in DOT format.
  """
  def to_dot(%__MODULE__{} = nfa) do
    lines = [
      "digraph NFA {",
      "    rankdir=LR;",
      "",
      "    __start [shape=point, width=0.2];",
      "    __start -> \"#{nfa.initial}\";",
      ""
    ]

    # State shapes
    state_lines =
      nfa.states
      |> Enum.sort()
      |> Enum.map(fn state ->
        shape = if MapSet.member?(nfa.accepting, state), do: "doublecircle", else: "circle"
        "    \"#{state}\" [shape=#{shape}];"
      end)

    # Group transitions by (source, target) to combine labels
    edge_labels =
      nfa.transitions
      |> Enum.sort()
      |> Enum.reduce(%{}, fn {{source, event}, targets}, acc ->
        label = if event == @epsilon, do: "\u03b5", else: event

        Enum.reduce(Enum.sort(targets), acc, fn target, inner_acc ->
          Map.update(inner_acc, {source, target}, [label], &(&1 ++ [label]))
        end)
      end)

    edge_lines =
      edge_labels
      |> Enum.sort()
      |> Enum.map(fn {{source, target}, labels} ->
        label = Enum.join(labels, ", ")
        "    \"#{source}\" -> \"#{target}\" [label=\"#{label}\"];"
      end)

    lines = lines ++ state_lines ++ [""] ++ edge_lines ++ ["}"]
    Enum.join(lines, "\n")
  end

  @doc """
  Convert a set of state names to a DFA state name string.

  The name is deterministic: sorted state names joined with commas
  and wrapped in braces.

  ## Examples

      iex> CodingAdventures.StateMachine.NFA.state_set_name(MapSet.new(["q0", "q2", "q1"]))
      "{q0,q1,q2}"
  """
  def state_set_name(state_set) do
    "{" <> (state_set |> Enum.sort() |> Enum.join(",")) <> "}"
  end

  # === Private validation helpers ===

  defp validate_non_empty(set) do
    if MapSet.size(set) == 0 do
      {:error, "States set must be non-empty"}
    else
      :ok
    end
  end

  defp validate_no_epsilon_in_alphabet(alphabet) do
    if MapSet.member?(alphabet, @epsilon) do
      {:error, "Alphabet must not contain the empty string (reserved for epsilon)"}
    else
      :ok
    end
  end

  defp validate_member(value, set, name) do
    if MapSet.member?(set, value) do
      :ok
    else
      {:error, "#{name} '#{value}' is not in the states set"}
    end
  end

  defp validate_subset(subset, superset, name) do
    invalid = MapSet.difference(subset, superset)

    if MapSet.size(invalid) == 0 do
      :ok
    else
      {:error, "#{name} #{inspect(Enum.sort(invalid))} are not in the states set"}
    end
  end

  defp validate_transitions(transitions, states, alphabet) do
    Enum.reduce_while(transitions, :ok, fn {{source, event}, targets}, :ok ->
      cond do
        not MapSet.member?(states, source) ->
          {:halt, {:error, "Transition source '#{source}' is not in the states set"}}

        event != @epsilon and not MapSet.member?(alphabet, event) ->
          {:halt,
           {:error, "Transition event '#{event}' is not in the alphabet and is not epsilon"}}

        true ->
          invalid_targets = MapSet.difference(targets, states)

          if MapSet.size(invalid_targets) > 0 do
            {:halt,
             {:error,
              "Transition targets #{inspect(Enum.sort(invalid_targets))} (from (#{source}, #{inspect(event)})) are not in the states set"}}
          else
            {:cont, :ok}
          end
      end
    end)
  end
end
