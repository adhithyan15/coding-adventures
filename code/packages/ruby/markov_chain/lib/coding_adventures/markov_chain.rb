# frozen_string_literal: true

# --------------------------------------------------------------------------
# markov_chain.rb — General-purpose Markov Chain (DT28)
# --------------------------------------------------------------------------
#
# A Markov Chain models a system that moves between a finite set of *states*
# over time.  The key property — the **Markov property** — is that the
# probability of the next state depends *only* on the current state, not on
# the history of how the system arrived there.
#
# Think of it like a forgetful navigator: wherever you are right now is all
# the information you need to decide where to go next.
#
# == Mathematical Background
#
# The full chain is captured in a **transition matrix** T where T[i][j] is
# the probability of going from state sᵢ to state sⱼ.  Each row must sum
# to exactly 1.0 — the chain must go *somewhere* on every step.
#
#   States:   {Sunny, Cloudy, Rainy}
#
#   T:            Sunny   Cloudy  Rainy
#   Sunny   [      0.7     0.2    0.1  ]
#   Cloudy  [      0.3     0.4    0.3  ]
#   Rainy   [      0.2     0.3    0.5  ]
#
# == Training
#
# We estimate T from observed data by counting how often each pair of
# consecutive states (context → next) appears, then normalising.
#
#   Observations: [A, B, A, C, A, B, B, A]
#   Counts:  A→B:2  A→C:1  B→A:2  B→B:1  C→A:1
#   T:  A→{B:2/3, C:1/3}  B→{A:2/3, B:1/3}  C→{A:1/1}
#
# == Laplace / Lidstone Smoothing
#
# When a transition has zero observations, it would get probability 0.0.
# That "locks out" those transitions forever, which can cause `next_state`
# to get stuck.  Smoothing adds a small pseudo-count α to every transition:
#
#   smoothed_count(i→j) = raw_count(i→j) + α
#   T[i][j] = smoothed_count(i→j) / Σⱼ smoothed_count(i→j)
#
#   α = 0.0  → no smoothing (default)
#   α = 1.0  → Laplace smoothing
#   α > 0    → Lidstone smoothing
#
# == Order-k Chains
#
# A standard (order-1) chain's next state depends only on the last 1 state.
# An order-k chain extends the memory window to k states:
#
#   P(next | s_{n-k}, …, s_{n-1}) = T[context_k][next]
#
# For text: order-1 uses single chars as context; order-2 uses pairs ("th"),
# order-3 uses triples ("the"), etc.  Higher orders produce more realistic
# output but need exponentially more training data.
#
# == Internal Design
#
# We wrap a `CodingAdventures::DirectedGraph::Graph` (with `allow_self_loops:
# true`) to store which (context, target) transitions *exist* as edges.
# The graph handles topology; a separate `@transitions` hash stores the
# actual floating-point probabilities.
#
# Why two data structures?
#   - The graph lets us reuse existing BFS/DFS and node-management code.
#   - The hash gives O(1) probability lookup without going through the graph.
#
# == Thread Safety
#
# This class is NOT thread-safe.  Wrap in a Mutex for concurrent access.
# --------------------------------------------------------------------------

module CodingAdventures
  class MarkovChain
    # -----------------------------------------------------------------------
    # Construction
    # -----------------------------------------------------------------------

    # Creates a new, empty Markov Chain.
    #
    # Parameters:
    #
    #   order:     (Integer) How many previous states form the context key.
    #              order=1 is the classic Markov chain.  order=2 uses pairs
    #              of consecutive states as context, and so on.
    #              Default: 1.
    #
    #   smoothing: (Float) Laplace / Lidstone smoothing parameter α.
    #              0.0 means no smoothing; 1.0 is classic Laplace.
    #              Applied during normalisation so that unseen transitions
    #              get a small probability rather than staying at 0.
    #              Default: 0.0.
    #
    #   states:    (Array or nil) Optional list of all known states to
    #              pre-register in the alphabet.  When smoothing > 0, ALL
    #              states in this list appear in the denominator, even if
    #              they were never seen in training.
    #              Default: nil (states discovered from training data).
    #
    # Example:
    #
    #   # Classic unsmoothed character chain
    #   chain = CodingAdventures::MarkovChain.new
    #
    #   # Order-2 chain with Laplace smoothing over explicit alphabet
    #   chain = CodingAdventures::MarkovChain.new(
    #     order: 2,
    #     smoothing: 1.0,
    #     states: ("a".."z").to_a
    #   )
    def initialize(order: 1, smoothing: 0.0, states: nil)
      @order     = order
      @smoothing = smoothing.to_f

      # The directed graph stores which (context → target) transitions exist
      # as edges.  self-loops are allowed because a state can transition back
      # to itself (e.g., "aa" → 'a' is perfectly valid in a character chain).
      @graph = CodingAdventures::DirectedGraph::Graph.new(allow_self_loops: true)

      # @transitions is the probability table.
      # Structure: { context_key => { target_state => Float } }
      # For order-1 chains, context_key is the state itself.
      # For order-k chains, context_key is a frozen Array of k states.
      @transitions = {}

      # @counts accumulates raw observations before normalisation.
      # Structure: { context_key => { target_state => Integer } }
      @counts = {}

      # @states_set is the full alphabet — all states ever seen or pre-registered.
      # We use a Hash (with value true) rather than an Array for O(1) inclusion.
      @states_set = {}

      # Pre-register any states the caller supplied.
      if states
        states.each { |s| register_state(s) }
      end
    end

    # -----------------------------------------------------------------------
    # Training
    # -----------------------------------------------------------------------

    # Train the chain on an array of states.
    #
    # The algorithm slides a window of size (order + 1) across the sequence.
    # For each window position i, the context is sequence[i, order] (a
    # sub-array of exactly `order` elements, frozen for use as a hash key),
    # and the target is sequence[i + order].
    #
    # Example for order=1, sequence=[A, B, A, C]:
    #   i=0: context=[A]  target=B  → count([A]→B) += 1
    #   i=1: context=[B]  target=A  → count([B]→A) += 1
    #   i=2: context=[A]  target=C  → count([A]→C) += 1
    #
    # After counting, we call _normalise to turn counts into probabilities.
    # Multiple calls to `train` accumulate counts before re-normalising, so
    # the chain learns from all training data combined.
    #
    # Parameters:
    #   sequence: (Array) A sequence of states to learn from.
    #
    # Returns: self (for method chaining)
    def train(sequence)
      # Register all states we encounter so the alphabet is complete.
      sequence.each { |s| register_state(s) }

      # Slide the window: we need at least (order + 1) items to form one pair.
      max_i = sequence.length - @order - 1
      (0..max_i).each do |i|
        # The context is the k-gram starting at position i.
        # We use sequence[i, @order] (Array#slice with length) and freeze it
        # so it can safely be used as a Hash key without mutation concerns.
        context = sequence[i, @order].freeze

        # The target is the element immediately after the context window.
        target = sequence[i + @order]

        # Accumulate the raw transition count.
        @counts[context] ||= {}
        @counts[context][target] = (@counts[context][target] || 0) + 1

        # Register the edge in the graph (topology only; duplicate edges are
        # silently ignored by Set semantics inside the graph).
        @graph.add_node(context)
        @graph.add_node(target)
        @graph.add_edge(context, target)
      end

      # Re-normalise all rows so probabilities reflect the updated counts.
      _normalise
      self
    end

    # Convenience wrapper: treat each character of a string as a state.
    #
    # Equivalent to `train(text.chars)`.
    #
    # Example:
    #   chain.train_string("abcabc")  # trains on ['a','b','c','a','b','c']
    #
    # Returns: self
    def train_string(text)
      train(text.chars)
    end

    # -----------------------------------------------------------------------
    # Sampling
    # -----------------------------------------------------------------------

    # Sample one transition from the current context.
    #
    # For order-1 chains, `current` is a single state value (e.g. "A").
    # For order-k chains, `current` is the k-tuple context key (frozen Array,
    # e.g. ["a", "b"].freeze for order-2).
    #
    # Internally, all contexts are stored as frozen Arrays — even for order-1.
    # This method accepts a plain state value for order-1 convenience and
    # converts it to the internal key automatically.
    #
    # We sample from the categorical distribution defined by the row
    # @transitions[context_key]:
    #
    #   1. Generate r = random float in [0, 1)
    #   2. Walk through (target, probability) pairs in insertion order,
    #      accumulating a running sum.
    #   3. Return the first target where the cumsum exceeds r.
    #
    # This is the standard alias/CDF method for discrete sampling.  It runs
    # in O(n) where n is the number of possible next states, which is fine
    # for the sizes we encounter in practice.
    #
    # Raises KeyError if `current` is not a known context in the chain.
    def next_state(current)
      context_key = _to_context_key(current)
      unless @transitions.key?(context_key)
        raise KeyError, "Unknown state: #{current.inspect}"
      end

      row = @transitions[context_key]
      r = rand

      cumsum = 0.0
      row.each do |target, prob|
        cumsum += prob
        return target if r < cumsum
      end

      # Floating-point rounding can leave cumsum just below 1.0 when r is
      # very close to 1.0.  Return the last entry as a safe fallback.
      row.keys.last
    end

    # Generate a sequence of exactly `length` states, starting from `start`.
    #
    # The output array always begins with `start` and contains exactly
    # `length` elements total (start counts as element #1).
    #
    # For order-1 chains: `start` is a single state.
    # For order-k chains: `start` is a seed of exactly `order` states
    # (we use start[0, order] as the initial context).
    #
    # Example (order=1):
    #   chain.generate("A", 5)  # => ["A", "B", "A", "C", "A"]  (varies)
    #
    # Returns: Array of exactly `length` states.
    def generate(start, length)
      # For order-1 we wrap the single start value in an array to form the
      # initial context key, then unwrap for the output.
      if @order == 1
        result = [start]
        context = [start].freeze

        (length - 1).times do
          nxt = next_state(context)
          result << nxt
          context = [nxt].freeze
        end

        result
      else
        # For order-k, start must be an array of at least `order` elements.
        # The initial context is start[0, @order].
        start_arr = Array(start)
        result = start_arr.dup
        context = start_arr[0, @order].freeze

        (length - start_arr.length).times do
          nxt = next_state(context)
          result << nxt
          # Advance the context window by one: drop the oldest, append the new.
          context = (context[1..] + [nxt]).freeze
        end

        result
      end
    end

    # Generate a String of exactly `length` characters.
    #
    # For order-1 chains, `seed` must be a single character (or a 1-char
    # string).  For order-k chains, `seed` must be at least `order`
    # characters long.
    #
    # Internally calls `generate` then joins the result array back into a
    # string.
    #
    # Example:
    #   chain.generate_string("th", 10)  # => "the quicha" (varies)
    #
    # Returns: String of exactly `length` characters.
    def generate_string(seed, length)
      if @order == 1
        # Single-char seed: treat seed's first char as the start state.
        chars = generate(seed[0], length)
        chars.join
      else
        # Multi-char seed: split into an array of chars for context.
        seed_chars = seed.chars
        chars = generate(seed_chars, length)
        chars.join
      end
    end

    # -----------------------------------------------------------------------
    # Probability queries
    # -----------------------------------------------------------------------

    # Return the probability T[from][to].
    #
    # For order-1 chains, `from` is a single state value (e.g. "A").
    # For order-k chains, `from` is the frozen context Array (e.g. ["a","b"]).
    #
    # Internally converts a plain state value to the frozen Array context key
    # when dealing with order-1 chains, so callers never have to think about
    # the internal key representation.
    #
    # Returns 0.0 if the transition was never observed (and smoothing == 0).
    def probability(from, to)
      context_key = _to_context_key(from)
      return 0.0 unless @transitions.key?(context_key)

      @transitions[context_key][to] || 0.0
    end

    # -----------------------------------------------------------------------
    # Stationary distribution (power iteration)
    # -----------------------------------------------------------------------

    # Compute the stationary distribution π via power iteration.
    #
    # The stationary distribution is the unique probability vector π such
    # that π · T = π.  It answers: "In the long run, what fraction of time
    # does the chain spend in each state?"
    #
    # Power iteration works by repeatedly multiplying an arbitrary initial
    # distribution by T until convergence.  Starting from the uniform
    # distribution converges fastest in practice.
    #
    #   π₀ = { s: 1/n  for each state s }
    #   πₙ₊₁[j] = Σᵢ  πₙ[i] · T[i][j]
    #   Stop when max |πₙ₊₁[s] - πₙ[s]| < 1e-10
    #
    # Convergence requires an **ergodic** chain — every state reachable from
    # every other state.  A non-ergodic chain may oscillate or stagnate.
    #
    # Raises RuntimeError after 10_000 iterations without convergence.
    #
    # Returns: Hash { state => Float }, values sum to ≈ 1.0.
    def stationary_distribution
      # We only operate over order-1 states (individual state symbols).
      all_states = @states_set.keys
      n = all_states.length

      raise RuntimeError, "Chain has no states" if n.zero?

      # Uniform initial distribution: each state gets probability 1/n.
      pi = {}
      all_states.each { |s| pi[s] = 1.0 / n }

      max_iterations = 10_000
      tolerance = 1e-10

      max_iterations.times do
        pi_new = {}
        all_states.each { |s| pi_new[s] = 0.0 }

        # π_new[j] = Σᵢ π[i] · T[i][j]
        # For order-1 chains, the transition key is [state].freeze
        all_states.each do |from_state|
          context = [from_state].freeze
          next unless @transitions.key?(context)

          @transitions[context].each do |to_state, prob|
            pi_new[to_state] = (pi_new[to_state] || 0.0) + pi[from_state] * prob
          end
        end

        # Check convergence: max absolute change across all states.
        max_delta = all_states.map { |s| (pi_new[s] - pi[s]).abs }.max

        pi = pi_new
        return pi if max_delta < tolerance
      end

      raise RuntimeError, "Chain is not ergodic: stationary distribution did not converge after #{max_iterations} iterations"
    end

    # -----------------------------------------------------------------------
    # Inspection
    # -----------------------------------------------------------------------

    # Returns a sorted array of all known states (the alphabet).
    #
    # For order-1 chains this is exactly the set of states seen in training
    # (plus any pre-registered via the `states:` constructor argument).
    def states
      @states_set.keys.sort
    end

    # Returns the full transition table as a nested Hash.
    #
    # Structure: { context_key => { target_state => Float } }
    #
    # For order-1 chains, context keys are single state values.
    # For order-k chains, context keys are frozen Arrays.
    def transition_matrix
      @transitions.dup
    end

    private

    # -----------------------------------------------------------------------
    # Private helpers
    # -----------------------------------------------------------------------

    # Convert a public-API `from` argument to the internal context key.
    #
    # Internally, ALL context keys are frozen Arrays — even for order-1 chains
    # where the context is a single state.  This uniformity simplifies the
    # implementation of `next_state`, `probability`, and `generate`.
    #
    # For order-1 chains:
    #   plain state value "A"          → ["A"].freeze
    #   already-wrapped ["A"].freeze   → ["A"].freeze  (unchanged)
    #
    # For order-k chains (k > 1):
    #   frozen Array ["a","b"]         → ["a","b"].freeze
    #   mutable Array ["a","b"]        → ["a","b"].freeze
    #
    # The caller is responsible for passing an appropriate `from` — we do
    # minimal coercion here to keep the method fast.
    def _to_context_key(from)
      if from.is_a?(Array)
        from.frozen? ? from : from.freeze
      else
        # Order-1: wrap a single state in a 1-element frozen Array.
        [from].freeze
      end
    end

    # Register a single state in the @states_set alphabet.
    #
    # We only track individual state values here (not k-gram contexts),
    # because the smoothing denominator and stationary distribution operate
    # over the symbol alphabet, not the context space.
    #
    # For order > 1, individual symbols are registered when `train` calls
    # this for each element of the sequence; the k-gram contexts are managed
    # separately in @counts.
    def register_state(state)
      @states_set[state] = true
      true
    end

    # Normalise all rows in @counts into probability distributions in
    # @transitions, applying Laplace / Lidstone smoothing if @smoothing > 0.
    #
    # This method is called after every `train` call to keep @transitions
    # up to date.  It rebuilds @transitions from scratch so accumulated
    # counts always reflect the correct normalised probabilities.
    #
    # The smoothing denominator uses ALL known states (the full alphabet),
    # not just the states that appear in a given row.  This ensures that
    # the smoothing is consistent regardless of which states were seen first.
    #
    # Algorithm:
    #   For each context c in @counts:
    #     total = Σ_target (raw_count[c][target] + α) over ALL states
    #           = Σ_seen raw_count[c][target] + α * |all_states|
    #     T[c][target] = (raw_count[c][target] + α) / total   for each target
    #
    # When α = 0 (no smoothing), only targets with raw_count > 0 get
    # non-zero probabilities and only those are stored.
    def _normalise
      all_states = @states_set.keys
      n_states = all_states.length

      @transitions = {}

      @counts.each do |context, target_counts|
        # Sum of all observed counts for this context.
        observed_sum = target_counts.values.sum

        # Total denominator = observed counts + smoothing bump for every state.
        total = observed_sum + @smoothing * n_states

        next if total.zero?

        row = {}

        if @smoothing > 0.0
          # With smoothing: every state in the alphabet gets at least α / total.
          all_states.each do |s|
            raw = target_counts[s] || 0
            p = (raw + @smoothing) / total
            row[s] = p if p > 0.0
          end
        else
          # Without smoothing: only emit entries for targets with raw_count > 0.
          target_counts.each do |target, count|
            row[target] = count.to_f / total
          end
        end

        @transitions[context] = row
      end
    end
  end
end
