defmodule CodingAdventures.MarkovChain do
  @moduledoc """
  DT28 — Markov Chain
  ===================

  A **Markov Chain** is a mathematical model for systems that hop between discrete
  states over time. The key property — called the **Markov property** — is that the
  next state depends *only* on the current state, not on how we got here.

  Think of it like a weather forecast that only uses today's weather:

      Today is Sunny  →  Tomorrow: Sunny (70%), Cloudy (20%), Rainy (10%)
      Today is Cloudy →  Tomorrow: Sunny (30%), Cloudy (40%), Rainy (30%)
      Today is Rainy  →  Tomorrow: Sunny (20%), Cloudy (30%), Rainy (50%)

  The full model is captured in a **transition matrix** T where `T[i][j]` is the
  probability of moving from state i to state j. Every row sums to 1.0.

  ## Historical Context

  Andrei Markov (1856–1922) introduced this model in 1906 while studying the
  distribution of vowels and consonants in Pushkin's *Eugene Onegin*. He wanted to
  prove that the Law of Large Numbers applied to *dependent* events — a rebuttal to
  critics who thought probability only worked for coin-flips.

  Claude Shannon later used Markov chains in his 1948 *A Mathematical Theory of
  Communication* to model English text statistically. Since then the model has spread
  to PageRank, gene prediction, game AI, LZMA compression, and MCMC sampling.

  ## Architecture

  This module is a **pure functional** implementation. Every function takes a `t()`
  struct and returns a new updated `t()` — no mutation, no hidden state.

  The struct holds:

  - `order`       — how many past states to use as context (k-gram length).
  - `smoothing`   — Laplace/Lidstone smoothing parameter α (0.0 = no smoothing).
  - `graph`       — a `CodingAdventures.DirectedGraph.Graph` for topology (who can
                    transition to whom).
  - `counts`      — raw transition counts accumulated during training.
  - `transitions` — normalised probability rows (recomputed after each training call).
  - `all_states`  — flat list of unique single-element states seen or pre-registered.

  ## Order-k Chains

  For order=1 (default), the context key is just the current state `s`.
  For order=k, the context key is a **list** of k states: `[s_{n-k}, …, s_{n-1}]`.

  Example with order=2 trained on "abcabc":

      Context ["a","b"] → next "c" with probability 1.0
      Context ["b","c"] → next "a" with probability 1.0
      Context ["c","a"] → next "b" with probability 1.0

  Internally the context list `["a","b"]` is stored as the map key directly.
  Elixir lists are comparable and work as map keys.

  ## Smoothing

  When smoothing α > 0, every possible transition from a context to any known state
  gets at least α added to its count before normalisation:

      smoothed_count(context → s) = raw_count(context → s) + α
      denominator = Σ_s (raw_count(context → s) + α)
                  = total_observed_from_context + α * |all_states|

  This prevents any transition from having probability 0, ensuring the chain can
  always move (never gets stuck).

  ## Usage

      alias CodingAdventures.MarkovChain

      chain = MarkovChain.new()
      chain = MarkovChain.train(chain, ~w[A B A C A B B A])
      {:ok, next} = MarkovChain.next_state(chain, "A")
      {:ok, seq}  = MarkovChain.generate(chain, "A", 10)

  For text generation:

      chain = MarkovChain.new(2, 0.1)          # order-2 with light smoothing
      chain = MarkovChain.train_string(chain, "abcabcabc")
      {:ok, text} = MarkovChain.generate_string(chain, "ab", 20)
      # → "abcabcabcabcabcabcab"
  """

  alias CodingAdventures.DirectedGraph.Graph

  # ---------------------------------------------------------------------------
  # Struct Definition
  # ---------------------------------------------------------------------------
  #
  # We enforce all keys to catch accidental construction without `new/3`.

  @enforce_keys [:order, :smoothing, :graph, :counts, :transitions, :all_states]
  defstruct [:order, :smoothing, :graph, :counts, :transitions, :all_states]

  @type t :: %__MODULE__{
          # How many previous states form the context key (k in "order-k chain").
          order: pos_integer(),
          # Smoothing parameter α: 0.0 = none, 1.0 = Laplace.
          smoothing: float(),
          # Directed graph for topology: edge from_context → to_state.
          graph: Graph.t(),
          # Raw counts: %{context_key => %{state => integer}}.
          counts: %{any() => %{any() => non_neg_integer()}},
          # Normalised probabilities: %{context_key => %{state => float}}.
          transitions: %{any() => %{any() => float()}},
          # Flat list of all unique single-element states (the alphabet).
          all_states: [any()]
        }

  # ---------------------------------------------------------------------------
  # Constructor
  # ---------------------------------------------------------------------------

  @doc """
  Create a new empty Markov chain.

  ## Parameters

  - `order`     — memory depth. `1` = standard chain (next depends on current only).
                  `2` = bigram context, etc. Defaults to `1`.
  - `smoothing` — Laplace/Lidstone α. `0.0` = no smoothing (default). `1.0` = Laplace.
  - `states`    — optional pre-registration of the alphabet. Useful when you know
                  all possible states upfront and want smoothing to cover them all.

  ## Examples

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> CodingAdventures.MarkovChain.states(chain)
      []

      iex> chain = CodingAdventures.MarkovChain.new(1, 1.0, ["A", "B", "C"])
      iex> length(CodingAdventures.MarkovChain.states(chain))
      3
  """
  @spec new(pos_integer(), float(), [any()]) :: t()
  def new(order \\ 1, smoothing \\ 0.0, states \\ []) do
    %__MODULE__{
      order: order,
      smoothing: smoothing,
      graph: Graph.new(allow_self_loops: true),
      counts: %{},
      transitions: %{},
      all_states: Enum.uniq(states)
    }
  end

  # ---------------------------------------------------------------------------
  # Training
  # ---------------------------------------------------------------------------

  @doc """
  Train the chain on a sequence of states.

  Slides a window of size `order + 1` over the sequence. For each window:
  - The first `order` elements form the **context key**.
  - The last element is the **target** (next state).
  - We increment `counts[context][target]`.

  After processing all windows, every row is re-normalised to probabilities
  (applying smoothing if configured).

  Calling `train/2` multiple times accumulates counts across calls, so the chain
  learns from the combined data.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B", "A", "C"])
      iex> CodingAdventures.MarkovChain.probability(chain, "A", "B")
      0.5
  """
  @spec train(t(), [any()]) :: t()
  def train(%__MODULE__{} = chain, sequence) when length(sequence) < 2, do: chain

  def train(%__MODULE__{} = chain, sequence) do
    # Step 1: Collect all new states to expand the alphabet.
    new_all_states = Enum.uniq(chain.all_states ++ sequence)

    # Step 2: Slide the window across the sequence, accumulating raw counts.
    # Window = [s_0, s_1, …, s_{order-1}, s_order]
    # Context = Enum.slice(window, 0, order)
    # Target  = List.last(window)
    new_counts =
      sequence
      |> Enum.chunk_every(chain.order + 1, 1, :discard)
      |> Enum.reduce(chain.counts, fn window, acc_counts ->
        context = Enum.slice(window, 0, chain.order)
        target = List.last(window)

        # For order-1, store context key as the bare state (not a 1-element list).
        # This keeps keys intuitive: %{"A" => %{"B" => 2}} rather than
        # %{["A"] => %{"B" => 2}}.
        context_key = if chain.order == 1, do: hd(context), else: context

        # Increment the count for this (context_key, target) pair.
        row = Map.get(acc_counts, context_key, %{})
        updated_row = Map.update(row, target, 1, &(&1 + 1))
        Map.put(acc_counts, context_key, updated_row)
      end)

    # Step 3: Re-normalise all rows into probabilities.
    # We must use ALL known states for smoothing denominators, not just those
    # seen from each context.
    new_transitions = normalise(new_counts, new_all_states, chain.smoothing)

    # Step 4: Sync the directed graph topology.
    # The `transitions` map only ever contains entries with prob > 0.0
    # (see `normalise/3`), so every entry here is a genuine transition to record.
    # We add both nodes and the directed edge; `add_edge` is idempotent on
    # duplicates, and `allow_self_loops: true` is set on construction so
    # self-transitions (e.g. B→B) are always accepted.
    new_graph =
      Enum.reduce(new_transitions, chain.graph, fn {ctx_key, row}, g ->
        Enum.reduce(row, g, fn {target, _prob}, g_inner ->
          {:ok, g2} = Graph.add_node(g_inner, ctx_key)
          {:ok, g3} = Graph.add_node(g2, target)
          {:ok, g4} = Graph.add_edge(g3, ctx_key, target)
          g4
        end)
      end)

    %{
      chain
      | counts: new_counts,
        transitions: new_transitions,
        graph: new_graph,
        all_states: new_all_states
    }
  end

  @doc """
  Train on a string, treating each character as a state.

  This is a convenience wrapper around `train/2`. The string is split into a
  list of single-character strings (graphemes), then passed to `train/2`.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new(2)
      iex> chain = CodingAdventures.MarkovChain.train_string(chain, "abcabc")
      iex> CodingAdventures.MarkovChain.probability(chain, ["a","b"], "c")
      1.0
  """
  @spec train_string(t(), String.t()) :: t()
  def train_string(%__MODULE__{} = chain, text) do
    chars = String.graphemes(text)
    train(chain, chars)
  end

  # ---------------------------------------------------------------------------
  # Sampling
  # ---------------------------------------------------------------------------

  @doc """
  Sample the next state from the current context.

  Uses transition probabilities to pick a successor by sampling from the
  categorical distribution for `current`. This is done by:

  1. Drawing a uniform random float r ∈ (0, 1].
  2. Walking the sorted transitions for `current`, accumulating probability.
  3. Returning the first state where the cumulative sum exceeds r.

  For order-k chains, `current` should be the k-gram list context key.

  Returns `{:error, reason}` if `current` is not a known context key.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B"])
      iex> {:ok, next} = CodingAdventures.MarkovChain.next_state(chain, "A")
      iex> next == "B"
      true
  """
  @spec next_state(t(), any()) :: {:ok, any()} | {:error, String.t()}
  def next_state(%__MODULE__{} = chain, current) do
    case Map.fetch(chain.transitions, current) do
      :error ->
        {:error, "Unknown state: #{inspect(current)}"}

      {:ok, row} ->
        # Sample from the categorical distribution defined by `row`.
        #
        # Algorithm (inverse CDF / roulette-wheel selection):
        # 1. Pick r ~ Uniform(0, 1).
        # 2. Walk states in sorted order, subtracting each probability from r.
        # 3. Return the first state where the running remainder drops to ≤ 0.
        # 4. If floating-point rounding leaves a tiny residual after all entries,
        #    return the last state (it absorbs the remaining probability mass).
        r = :rand.uniform()

        # Sort entries by key for deterministic traversal order.
        sorted = Enum.sort_by(row, fn {state, _prob} -> inspect(state) end)

        # Use a tagged tuple accumulator so the halted state is always
        # distinguishable from the float remainder regardless of state type.
        result =
          Enum.reduce_while(sorted, {:cont, r}, fn {state, prob}, {:cont, remaining} ->
            new_remaining = remaining - prob

            if new_remaining <= 0 do
              {:halt, {:picked, state}}
            else
              {:cont, {:cont, new_remaining}}
            end
          end)

        case result do
          {:picked, state} ->
            {:ok, state}

          # Floating-point rounding edge case: after walking all entries the
          # cumulative probability fell just short of 1.0. Return the last entry.
          {:cont, _remaining} ->
            {last_state, _} = List.last(sorted)
            {:ok, last_state}
        end
    end
  end

  @doc """
  Generate a sequence of `length` states, starting from `start`.

  `start` is included as the first element of the result. Each subsequent state
  is sampled using `next_state/2`.

  For order-k chains (k > 1), `start` must be a list of k states forming the
  initial context. The output list begins with all elements of `start`, followed
  by newly generated states.

  Returns `{:error, reason}` if the starting state/context is not known.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> seq = Enum.flat_map(1..5, fn _ -> ["A","B"] end)
      iex> chain = CodingAdventures.MarkovChain.train(chain, seq)
      iex> {:ok, result} = CodingAdventures.MarkovChain.generate(chain, "A", 4)
      iex> length(result)
      4
  """
  @spec generate(t(), any(), integer()) :: {:ok, [any()]} | {:error, String.t()}
  def generate(%__MODULE__{} = _chain, start, length) when length <= 0 do
    {:ok, if(is_list(start), do: start, else: [start])}
  end

  def generate(%__MODULE__{} = chain, start, length) do
    if chain.order == 1 do
      generate_order1(chain, start, length)
    else
      generate_orderk(chain, start, length)
    end
  end

  # Order-1: each step uses the current single state as the context key.
  defp generate_order1(chain, start, length) do
    # Include `start` as element #1, generate (length - 1) more.
    do_generate1(chain, start, length - 1, [start])
  end

  defp do_generate1(_chain, _current, 0, acc), do: {:ok, Enum.reverse(acc)}

  defp do_generate1(chain, current, remaining, acc) do
    case next_state(chain, current) do
      {:ok, next} -> do_generate1(chain, next, remaining - 1, [next | acc])
      {:error, _} = err -> err
    end
  end

  # Order-k: the context is a list of k states.
  # `start` is expected to be a list of exactly `order` states.
  defp generate_orderk(chain, start, length) do
    # Normalise start to a list of length `order`.
    context =
      cond do
        is_list(start) -> start
        true -> [start]
      end

    # The output sequence starts with the context elements, then we generate
    # more states until the total is `length`.
    if length <= chain.order do
      {:ok, Enum.take(context, length)}
    else
      # We have `chain.order` elements from the context prefix;
      # need `length - chain.order` more.
      do_generatek(chain, context, length - chain.order, context)
    end
  end

  defp do_generatek(_chain, _context, 0, acc), do: {:ok, acc}

  defp do_generatek(chain, context, remaining, acc) do
    case next_state(chain, context) do
      {:ok, next} ->
        # Slide the window: drop oldest, append new.
        new_context = tl(context) ++ [next]
        do_generatek(chain, new_context, remaining - 1, acc ++ [next])

      {:error, _} = err ->
        err
    end
  end

  @doc """
  Generate a string of `length` characters using character-level transitions.

  A convenience wrapper around `generate/3` for character chains. `seed` must
  be at least `order` characters long — it provides the initial context window.

  The returned string has exactly `length` characters.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new(2)
      iex> chain = CodingAdventures.MarkovChain.train_string(chain, "abcabcabc")
      iex> {:ok, text} = CodingAdventures.MarkovChain.generate_string(chain, "ab", 9)
      iex> text
      "abcabcabc"
  """
  @spec generate_string(t(), String.t(), integer()) :: {:ok, String.t()} | {:error, String.t()}
  def generate_string(%__MODULE__{} = chain, seed, length) do
    seed_chars = String.graphemes(seed)

    start =
      if chain.order == 1 do
        # For order-1, use the last character of the seed as the start state.
        List.last(seed_chars)
      else
        # For order-k, use the LAST `order` characters as the context.
        Enum.take(seed_chars, -chain.order)
      end

    case generate(chain, start, length) do
      {:ok, chars} -> {:ok, Enum.join(chars)}
      {:error, _} = err -> err
    end
  end

  # ---------------------------------------------------------------------------
  # Probability Query
  # ---------------------------------------------------------------------------

  @doc """
  Return the transition probability from `from` (context) to `to` (next state).

  Returns `0.0` if the transition was never observed (and smoothing = 0.0).
  For order-k chains, `from` should be the k-gram context list.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B"])
      iex> CodingAdventures.MarkovChain.probability(chain, "A", "B")
      1.0

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B"])
      iex> CodingAdventures.MarkovChain.probability(chain, "A", "C")
      0.0
  """
  @spec probability(t(), any(), any()) :: float()
  def probability(%__MODULE__{} = chain, from, to) do
    chain.transitions
    |> Map.get(from, %{})
    |> Map.get(to, 0.0)
  end

  # ---------------------------------------------------------------------------
  # Stationary Distribution (Power Iteration)
  # ---------------------------------------------------------------------------

  @doc """
  Compute the stationary distribution π using power iteration.

  The stationary distribution answers: "In the long run, what fraction of time
  does the chain spend in each state?"

  Mathematically: `π · T = π` (π is a left eigenvector of T with eigenvalue 1).

  We use **power iteration**:

  1. Start with a uniform distribution: π[s] = 1/n for all n context keys.
  2. Multiply: π_new[s_j] = Σ_{s_i} π[s_i] * T[s_i][s_j]
  3. Repeat until the maximum change in any entry is < 1e-10.

  Returns `{:error, reason}` if:
  - The chain has no states.
  - The chain fails to converge in 10,000 iterations.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A","B","A","C","A","B","B","A"])
      iex> {:ok, dist} = CodingAdventures.MarkovChain.stationary_distribution(chain)
      iex> abs(Enum.sum(Map.values(dist)) - 1.0) < 1.0e-9
      true
  """
  @spec stationary_distribution(t()) :: {:ok, %{any() => float()}} | {:error, String.t()}
  def stationary_distribution(%__MODULE__{} = chain) do
    context_keys = Map.keys(chain.transitions)

    if Enum.empty?(context_keys) do
      {:error, "Cannot compute stationary distribution: chain has no states"}
    else
      n = length(context_keys)
      uniform = 1.0 / n

      pi = Map.new(context_keys, fn k -> {k, uniform} end)
      do_power_iteration(pi, chain.transitions, context_keys, 0)
    end
  end

  # Convergence threshold: stop when max |π_new[s] - π[s]| < epsilon.
  @convergence_epsilon 1.0e-10
  @max_iterations 10_000

  defp do_power_iteration(_pi, _transitions, _keys, iteration)
       when iteration >= @max_iterations do
    {:error, "Stationary distribution did not converge in #{@max_iterations} iterations"}
  end

  defp do_power_iteration(pi, transitions, keys, iteration) do
    # One power-iteration step:
    # π_new[s_j] = Σ_{s_i} π[s_i] * T[s_i][s_j]
    #
    # Iterate over all source context keys s_i, distributing their probability
    # mass to each successor s_j according to T[s_i][s_j].
    pi_new =
      Enum.reduce(keys, Map.new(keys, fn k -> {k, 0.0} end), fn source, acc ->
        pi_source = Map.get(pi, source, 0.0)
        row = Map.get(transitions, source, %{})

        Enum.reduce(row, acc, fn {target, prob}, inner_acc ->
          # Only accumulate mass for targets that are also context keys
          # (i.e., part of our probability distribution).
          if Map.has_key?(inner_acc, target) do
            Map.update(inner_acc, target, pi_source * prob, &(&1 + pi_source * prob))
          else
            inner_acc
          end
        end)
      end)

    # Re-normalise to correct any floating-point drift from repeated iteration.
    total = Enum.sum(Map.values(pi_new))

    pi_normalised =
      if total > 0 do
        Map.new(pi_new, fn {k, v} -> {k, v / total} end)
      else
        pi_new
      end

    # Compute convergence metric: max |π_new[s] - π[s]|.
    max_delta =
      Enum.reduce(keys, 0.0, fn k, acc ->
        delta = abs(Map.get(pi_normalised, k, 0.0) - Map.get(pi, k, 0.0))
        max(acc, delta)
      end)

    if max_delta < @convergence_epsilon do
      {:ok, pi_normalised}
    else
      do_power_iteration(pi_normalised, transitions, keys, iteration + 1)
    end
  end

  # ---------------------------------------------------------------------------
  # Inspection
  # ---------------------------------------------------------------------------

  @doc """
  Return all registered single-element states (the alphabet).

  This is the flat list of atoms/strings/terms that form the state space.
  For order-1 chains this equals the set of context keys in the transition
  matrix. For order-k chains this is the flat alphabet from which k-grams
  are built.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B"])
      iex> Enum.sort(CodingAdventures.MarkovChain.states(chain))
      ["A", "B"]
  """
  @spec states(t()) :: [any()]
  def states(%__MODULE__{} = chain), do: chain.all_states

  @doc """
  Return the full transition matrix as a nested map.

  The outer key is the context (state for order-1, list for order-k).
  The inner key is the target state.
  The value is the transition probability (a float between 0.0 and 1.0).

  Zero-probability transitions are omitted from the sparse representation.

  ## Example

      iex> chain = CodingAdventures.MarkovChain.new()
      iex> chain = CodingAdventures.MarkovChain.train(chain, ["A", "B"])
      iex> CodingAdventures.MarkovChain.transition_matrix(chain)
      %{"A" => %{"B" => 1.0}}
  """
  @spec transition_matrix(t()) :: %{any() => %{any() => float()}}
  def transition_matrix(%__MODULE__{} = chain), do: chain.transitions

  # ---------------------------------------------------------------------------
  # Private: Normalise Counts → Probabilities
  # ---------------------------------------------------------------------------

  # Convert raw count maps into probability distributions.
  #
  # The smoothing formula (Lidstone / Laplace when α = 1):
  #
  #   P(context → target) = (count(context → target) + α)
  #                         / (total_from_context + α * |all_states|)
  #
  # When α = 0 this reduces to plain relative frequency.
  # When α = 1 this is Laplace (add-one) smoothing.
  #
  # We only include entries with prob > 0 to keep the map sparse.
  defp normalise(counts, all_states, smoothing) do
    n = length(all_states)

    Map.new(counts, fn {context_key, row} ->
      total_observed = Enum.sum(Map.values(row))
      denominator = total_observed + smoothing * n

      row_probs =
        Enum.reduce(all_states, %{}, fn state, acc ->
          raw = Map.get(row, state, 0)

          prob =
            if denominator > 0 do
              (raw + smoothing) / denominator
            else
              0.0
            end

          # Sparse: skip zero-probability entries.
          if prob > 0.0 do
            Map.put(acc, state, prob)
          else
            acc
          end
        end)

      {context_key, row_probs}
    end)
  end
end
