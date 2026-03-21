defmodule CodingAdventures.StateMachine.Minimize do
  @moduledoc """
  DFA Minimization using Hopcroft's algorithm.

  ## What is DFA minimization?

  Two DFA states are **equivalent** if, for every possible input sequence,
  they either both lead to acceptance or both lead to rejection. Equivalent
  states can be merged without changing the language the DFA recognizes.

  DFA minimization finds and merges all equivalent states, producing the
  **smallest possible DFA** for a given regular language. This minimal DFA
  is unique (up to state renaming) — no matter how you construct a DFA for
  a language, minimization always produces the same result.

  ## Why minimize?

  1. **Efficiency:** Fewer states = less memory, faster lookup tables.
  2. **Canonical form:** Two DFAs recognize the same language if and only if
     their minimal forms are identical (after renaming). This gives us a way
     to test language equivalence.
  3. **Clean up subset construction:** Converting an NFA to a DFA via subset
     construction often produces many redundant states. Minimization removes
     them.

  ## Hopcroft's Algorithm

  The algorithm works by **partition refinement**:

  1. Start with two groups: accepting states and non-accepting states.
     (These are definitely NOT equivalent to each other.)

  2. For each group and each input symbol, check: do all states in the
     group go to the same group on that input? If not, split the group.

  3. Repeat until no group can be split.

  4. Each final group becomes one state in the minimized DFA.

  Time complexity: O(n log n) where n = number of states.
  """

  alias CodingAdventures.StateMachine.DFA

  @doc """
  Minimize a DFA using Hopcroft's algorithm.

  Returns a new DFA with the minimum number of states that recognizes
  the same language as the input DFA. Unreachable states are removed
  first, then equivalent states are merged.

  The minimized DFA is unique (up to state naming) for any regular
  language — this is a fundamental theorem of automata theory.

  ## Parameters

  - `dfa` — the DFA to minimize

  ## Returns

  `{:ok, minimized_dfa}` — a new, minimized DFA.

  ## Examples

      iex> alias CodingAdventures.StateMachine.{DFA, Minimize}
      iex> {:ok, big} = DFA.new(
      ...>   MapSet.new(["q0", "q1", "q2", "q3"]),
      ...>   MapSet.new(["a", "b"]),
      ...>   %{{"q0", "a"} => "q1", {"q0", "b"} => "q2",
      ...>     {"q1", "a"} => "q1", {"q1", "b"} => "q1",
      ...>     {"q2", "a"} => "q2", {"q2", "b"} => "q2",
      ...>     {"q3", "a"} => "q3", {"q3", "b"} => "q3"},
      ...>   "q0",
      ...>   MapSet.new(["q1", "q2"])
      ...> )
      iex> {:ok, small} = Minimize.minimize(big)
      iex> MapSet.size(small.states) < MapSet.size(big.states)
      true
  """
  def minimize(%DFA{} = dfa) do
    # Step 0: Remove unreachable states
    reachable = DFA.reachable_states(dfa)
    reachable_accepting = MapSet.intersection(dfa.accepting, reachable)

    # Filter transitions to only reachable states
    transitions =
      dfa.transitions
      |> Enum.filter(fn {{s, _e}, t} ->
        MapSet.member?(reachable, s) and MapSet.member?(reachable, t)
      end)
      |> Map.new()

    # Step 1: Initial partition — accepting vs non-accepting
    accepting = reachable_accepting
    non_accepting = MapSet.difference(reachable, accepting)

    partitions = []
    partitions = if MapSet.size(accepting) > 0, do: partitions ++ [accepting], else: partitions

    partitions =
      if MapSet.size(non_accepting) > 0, do: partitions ++ [non_accepting], else: partitions

    if length(partitions) == 0 do
      # Edge case: no reachable states
      {:ok, dfa}
    else
      # Step 2-3: Iteratively refine partitions
      alphabet = Enum.sort(dfa.alphabet)
      partitions = refine_partitions(partitions, alphabet, transitions)

      # Step 4: Build the minimized DFA
      state_to_partition =
        partitions
        |> Enum.flat_map(fn partition ->
          Enum.map(partition, fn state -> {state, partition} end)
        end)
        |> Map.new()

      # Build new states, transitions, initial, accepting
      new_states =
        partitions
        |> Enum.map(&partition_name/1)
        |> MapSet.new()

      new_accepting =
        partitions
        |> Enum.filter(fn partition ->
          MapSet.size(MapSet.intersection(partition, accepting)) > 0
        end)
        |> Enum.map(&partition_name/1)
        |> MapSet.new()

      new_transitions =
        partitions
        |> Enum.reduce(%{}, fn partition, acc ->
          name = partition_name(partition)
          representative = partition |> Enum.sort() |> hd()

          Enum.reduce(alphabet, acc, fn event, inner_acc ->
            case Map.get(transitions, {representative, event}) do
              nil ->
                inner_acc

              target ->
                target_partition = Map.fetch!(state_to_partition, target)
                target_name = partition_name(target_partition)
                Map.put(inner_acc, {name, event}, target_name)
            end
          end)
        end)

      # Find the new initial state
      initial_partition = Map.fetch!(state_to_partition, dfa.initial)
      new_initial = partition_name(initial_partition)

      DFA.new(
        new_states,
        dfa.alphabet,
        new_transitions,
        new_initial,
        new_accepting
      )
    end
  end

  # Iteratively refine partitions until stable
  defp refine_partitions(partitions, alphabet, transitions) do
    new_partitions =
      Enum.flat_map(partitions, fn group ->
        split_group(group, alphabet, transitions, partitions)
      end)

    if new_partitions == partitions do
      partitions
    else
      refine_partitions(new_partitions, alphabet, transitions)
    end
  end

  @doc """
  Attempt to split a group based on transition targets.

  Two states in the same group are equivalent only if, for every input
  symbol, they transition to states in the same partition. If they
  differ on any input, they must be in different groups.

  This is a public function for testing purposes.
  """
  def split_group(group, alphabet, transitions, partitions) do
    if MapSet.size(group) <= 1 do
      [group]
    else
      # Build a lookup: state -> which partition index it belongs to
      state_to_partition_idx =
        partitions
        |> Enum.with_index()
        |> Enum.flat_map(fn {partition, idx} ->
          Enum.map(partition, fn state -> {state, idx} end)
        end)
        |> Map.new()

      # Try each event — if any event splits the group, return the split
      Enum.reduce_while(alphabet, [group], fn event, _acc ->
        signatures =
          Enum.group_by(group, fn state ->
            case Map.get(transitions, {state, event}) do
              nil -> nil
              target -> Map.get(state_to_partition_idx, target)
            end
          end)

        if map_size(signatures) > 1 do
          subgroups = Enum.map(signatures, fn {_sig, states} -> MapSet.new(states) end)
          {:halt, subgroups}
        else
          {:cont, [group]}
        end
      end)
    end
  end

  # Generate a name for a partition (group of equivalent states)
  defp partition_name(partition) do
    members = Enum.sort(partition)

    if length(members) == 1 do
      hd(members)
    else
      "{" <> Enum.join(members, ",") <> "}"
    end
  end
end
