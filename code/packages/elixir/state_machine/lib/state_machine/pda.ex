defmodule CodingAdventures.StateMachine.PDA do
  @moduledoc """
  Pushdown Automaton (PDA) — a finite automaton with a stack.

  ## What is a PDA?

  A PDA is a state machine augmented with a **stack** — an unbounded LIFO
  (last-in, first-out) data structure. The stack gives the PDA the ability
  to "remember" things that a finite automaton cannot, like how many open
  parentheses it has seen.

  This extra memory is exactly what is needed to recognize **context-free
  languages** — the class of languages that includes balanced parentheses,
  nested HTML tags, arithmetic expressions, and most programming language
  syntax.

  ## The Chomsky Hierarchy Connection

      Regular languages    subset  Context-free languages  subset  Context-sensitive  subset  RE
      (DFA/NFA)                    (PDA)                           (LBA)                      (TM)

  A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
  recognize "does this string have equal numbers of a's and b's?" — that
  requires counting, and a DFA has no memory beyond its finite state.

  A PDA can recognize "a^n b^n" (n a's followed by n b's) because it can
  push an 'a' for each 'a' it reads, then pop an 'a' for each 'b'. If the
  stack is empty at the end, the counts match.

  ## Formal Definition

      PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)

      Q  = finite set of states
      Sigma = input alphabet
      Gamma = stack alphabet (may differ from Sigma)
      delta = transition function: Q x (Sigma union {epsilon}) x Gamma -> P(Q x Gamma*)
      q0 = initial state
      Z0 = initial stack symbol (bottom marker)
      F  = accepting states

  Our implementation is deterministic (DPDA): at most one transition applies
  at any time. This is simpler to implement and trace, and sufficient for
  most practical parsing tasks.

  ## Elixir Design

  Since Elixir is immutable, the PDA struct carries the full configuration
  (state + stack + trace). Every operation returns a new struct. The stack
  is represented as a list where the last element is the top (matching the
  Python implementation's convention).
  """

  defmodule Transition do
    @moduledoc """
    A single transition rule for a pushdown automaton.

    A PDA transition says: "If I am in state `source`, and I see input
    `event` (or epsilon if nil), and the top of my stack is `stack_read`,
    then move to state `target` and replace the stack top with `stack_push`.

    ## Stack semantics

    - `stack_push: []` — pop the top (consume it)
    - `stack_push: ["X"]` — replace top with X
    - `stack_push: ["X", "Y"]` — pop top, push X, then push Y (Y is new top)
    - `stack_push: [stack_read]` — leave the stack unchanged

    ## Examples

        %Transition{source: "q0", event: "(", stack_read: "$", target: "q0",
                    stack_push: ["$", "("]}
        # "In q0, reading '(', with '$' on top: stay in q0, push '(' above '$'"
    """
    defstruct [:source, :event, :stack_read, :target, :stack_push]

    @type t :: %__MODULE__{
            source: String.t(),
            event: String.t() | nil,
            stack_read: String.t(),
            target: String.t(),
            stack_push: [String.t()]
          }
  end

  defmodule TraceEntry do
    @moduledoc """
    One step in a PDA's execution trace.

    Captures the full state of the PDA at each transition: which rule
    fired, what the stack looked like after.
    """
    defstruct [:source, :event, :stack_read, :target, :stack_push, :stack_after]

    @type t :: %__MODULE__{
            source: String.t(),
            event: String.t() | nil,
            stack_read: String.t(),
            target: String.t(),
            stack_push: [String.t()],
            stack_after: [String.t()]
          }
  end

  defstruct [
    :states,
    :input_alphabet,
    :stack_alphabet,
    :transitions,
    :initial,
    :initial_stack_symbol,
    :accepting,
    :current,
    :stack,
    :trace
  ]

  @type t :: %__MODULE__{
          states: MapSet.t(String.t()),
          input_alphabet: MapSet.t(String.t()),
          stack_alphabet: MapSet.t(String.t()),
          transitions: [Transition.t()],
          initial: String.t(),
          initial_stack_symbol: String.t(),
          accepting: MapSet.t(String.t()),
          current: String.t(),
          stack: [String.t()],
          trace: [TraceEntry.t()]
        }

  @doc """
  Create a new PDA.

  ## Parameters

  - `states` — finite set of states
  - `input_alphabet` — finite set of input symbols
  - `stack_alphabet` — finite set of stack symbols
  - `transitions` — list of `Transition` structs
  - `initial` — starting state
  - `initial_stack_symbol` — symbol placed on the stack initially (typically "$")
  - `accepting` — set of accepting/final states

  ## Returns

  `{:ok, pda}` on success, `{:error, reason}` on validation failure.

  ## Examples

      iex> alias CodingAdventures.StateMachine.PDA
      iex> alias CodingAdventures.StateMachine.PDA.Transition
      iex> {:ok, pda} = PDA.new(
      ...>   MapSet.new(["q0", "accept"]),
      ...>   MapSet.new(["(", ")"]),
      ...>   MapSet.new(["(", "$"]),
      ...>   [
      ...>     %Transition{source: "q0", event: "(", stack_read: "$",
      ...>                 target: "q0", stack_push: ["$", "("]},
      ...>     %Transition{source: "q0", event: "(", stack_read: "(",
      ...>                 target: "q0", stack_push: ["(", "("]},
      ...>     %Transition{source: "q0", event: ")", stack_read: "(",
      ...>                 target: "q0", stack_push: []},
      ...>     %Transition{source: "q0", event: nil, stack_read: "$",
      ...>                 target: "accept", stack_push: []},
      ...>   ],
      ...>   "q0",
      ...>   "$",
      ...>   MapSet.new(["accept"])
      ...> )
      iex> pda.current
      "q0"
  """
  def new(states, input_alphabet, stack_alphabet, transitions, initial, initial_stack_symbol, accepting) do
    with :ok <- validate_non_empty(states),
         :ok <- validate_member(initial, states, "Initial state", "states set"),
         :ok <- validate_member(initial_stack_symbol, stack_alphabet, "Initial stack symbol", "stack alphabet"),
         :ok <- validate_subset(accepting, states, "Accepting states", "states set"),
         :ok <- validate_deterministic(transitions) do
      # Build transition index for fast lookup: {state, event_or_nil, stack_top} => Transition
      transition_index =
        transitions
        |> Enum.map(fn t -> {{t.source, t.event, t.stack_read}, t} end)
        |> Map.new()

      {:ok,
       %__MODULE__{
         states: states,
         input_alphabet: input_alphabet,
         stack_alphabet: stack_alphabet,
         transitions: transitions,
         initial: initial,
         initial_stack_symbol: initial_stack_symbol,
         accepting: accepting,
         current: initial,
         stack: [initial_stack_symbol],
         trace: [],
         # Store the index as part of the struct via a workaround — we put it in
         # the transitions field as a tuple so we can access it later.
         # Actually, let's just store it separately. We'll use the module attribute pattern.
       }
       |> Map.put(:_transition_index, transition_index)}
    end
  end

  @doc """
  Process one input symbol.

  Checks for a transition on the given event with the current stack top.
  If found, applies it and returns the updated PDA.

  ## Parameters

  - `pda` — the current PDA struct
  - `event` — an input symbol

  ## Returns

  `{:ok, new_pda}` on success, `{:error, reason}` if no transition matches.
  """
  def process(%__MODULE__{} = pda, event) do
    case find_transition(pda, event) do
      nil ->
        stack_top = stack_top(pda)
        {:error, "No transition for (state='#{pda.current}', event=#{inspect(event)}, stack_top=#{inspect(stack_top)})"}

      transition ->
        {:ok, apply_transition(pda, transition)}
    end
  end

  @doc """
  Process a sequence of inputs and return the updated PDA with a trace.

  After processing all inputs, tries epsilon transitions until none are
  available (this handles acceptance transitions that fire at end-of-input).

  ## Parameters

  - `pda` — the current PDA struct
  - `events` — list of input symbols

  ## Returns

  `{:ok, new_pda, trace}` on success, `{:error, reason}` on failure.
  """
  def process_sequence(%__MODULE__{} = pda, events) do
    trace_start = length(pda.trace)

    result =
      Enum.reduce_while(events, {:ok, pda}, fn event, {:ok, acc} ->
        case process(acc, event) do
          {:ok, new_pda} -> {:cont, {:ok, new_pda}}
          {:error, reason} -> {:halt, {:error, reason}}
        end
      end)

    case result do
      {:ok, pda_after} ->
        # Try epsilon transitions at end of input
        pda_after = try_epsilons(pda_after)
        new_trace = Enum.drop(pda_after.trace, trace_start)
        {:ok, pda_after, new_trace}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Check if the PDA accepts the input sequence.

  Processes all inputs, then tries epsilon transitions until none are
  available. Returns true if the final state is accepting.

  Does NOT modify this PDA — runs on a fresh simulation.

  ## Parameters

  - `pda` — the PDA struct
  - `events` — list of input symbols

  ## Returns

  `true` if the PDA accepts, `false` otherwise.
  """
  def accepts?(%__MODULE__{} = pda, events) do
    index = Map.get(pda, :_transition_index, build_index(pda.transitions))

    # Simulate on fresh state
    state = pda.initial
    stack = [pda.initial_stack_symbol]

    result =
      Enum.reduce_while(events, {state, stack}, fn event, {st, stk} ->
        if stk == [] do
          {:halt, :reject}
        else
          top = List.last(stk)

          case Map.get(index, {st, event, top}) do
            nil -> {:halt, :reject}
            t ->
              new_stack = List.delete_at(stk, -1) ++ t.stack_push
              {:cont, {t.target, new_stack}}
          end
        end
      end)

    case result do
      :reject ->
        false

      {state, stack} ->
        # Try epsilon transitions at end of input
        max_epsilon = length(pda.transitions) + 1
        {final_state, _final_stack} = try_epsilons_sim(state, stack, index, max_epsilon)
        MapSet.member?(pda.accepting, final_state)
    end
  end

  @doc """
  Reset to initial state with initial stack.

  Returns a new PDA in the same configuration as when first constructed.
  """
  def reset(%__MODULE__{} = pda) do
    %{pda | current: pda.initial, stack: [pda.initial_stack_symbol], trace: []}
  end

  @doc """
  Get the top of the stack, or nil if empty.
  """
  def stack_top(%__MODULE__{stack: []}), do: nil
  def stack_top(%__MODULE__{stack: stack}), do: List.last(stack)

  # === Private helpers ===

  defp find_transition(pda, event) do
    index = Map.get(pda, :_transition_index, build_index(pda.transitions))

    if pda.stack == [] do
      nil
    else
      top = List.last(pda.stack)
      Map.get(index, {pda.current, event, top})
    end
  end

  defp apply_transition(pda, transition) do
    # Pop the stack top (it was "read" by the transition)
    new_stack = List.delete_at(pda.stack, -1)

    # Push new symbols (first element goes deepest, matching Python convention)
    new_stack = new_stack ++ transition.stack_push

    entry = %TraceEntry{
      source: transition.source,
      event: transition.event,
      stack_read: transition.stack_read,
      target: transition.target,
      stack_push: transition.stack_push,
      stack_after: new_stack
    }

    %{pda | current: transition.target, stack: new_stack, trace: pda.trace ++ [entry]}
  end

  defp try_epsilons(pda) do
    case find_transition(pda, nil) do
      nil -> pda
      transition -> pda |> apply_transition(transition) |> try_epsilons()
    end
  end

  defp try_epsilons_sim(state, stack, _index, 0), do: {state, stack}
  defp try_epsilons_sim(state, [], _index, _remaining), do: {state, []}

  defp try_epsilons_sim(state, stack, index, remaining) do
    top = List.last(stack)

    case Map.get(index, {state, nil, top}) do
      nil ->
        {state, stack}

      t ->
        new_stack = List.delete_at(stack, -1) ++ t.stack_push
        try_epsilons_sim(t.target, new_stack, index, remaining - 1)
    end
  end

  defp build_index(transitions) do
    transitions
    |> Enum.map(fn t -> {{t.source, t.event, t.stack_read}, t} end)
    |> Map.new()
  end

  defp validate_non_empty(set) do
    if MapSet.size(set) == 0 do
      {:error, "States set must be non-empty"}
    else
      :ok
    end
  end

  defp validate_member(value, set, value_name, set_name) do
    if MapSet.member?(set, value) do
      :ok
    else
      {:error, "#{value_name} '#{value}' is not in the #{set_name}"}
    end
  end

  defp validate_subset(subset, superset, subset_name, superset_name) do
    invalid = MapSet.difference(subset, superset)

    if MapSet.size(invalid) == 0 do
      :ok
    else
      {:error, "#{subset_name} #{inspect(Enum.sort(invalid))} are not in the #{superset_name}"}
    end
  end

  defp validate_deterministic(transitions) do
    keys = Enum.map(transitions, fn t -> {t.source, t.event, t.stack_read} end)
    unique_keys = Enum.uniq(keys)

    if length(keys) == length(unique_keys) do
      :ok
    else
      # Find the duplicate
      duplicate =
        keys
        |> Enum.frequencies()
        |> Enum.find(fn {_k, v} -> v > 1 end)
        |> elem(0)

      {source, event, stack_read} = duplicate

      {:error,
       "Duplicate transition for (#{source}, #{inspect(event)}, #{inspect(stack_read)}) — this PDA must be deterministic"}
    end
  end
end
