"""
coding_adventures_markov_chain — DT28: Markov Chain
=====================================================

A **Markov Chain** is a mathematical model of a system that moves between a
finite set of **states** over time, where the probability of transitioning to
the next state depends *only* on the current state — not on any history of how
the system got there. This "memorylessness" property is the **Markov property**.

Mathematical formulation
------------------------
Given a finite state space S = {s₀, s₁, …, sₙ₋₁}, the entire chain is
captured in a **transition matrix** T where:

    T[i][j] = P(next = sⱼ | current = sᵢ)

Each row of T sums to 1.0 — the chain must go *somewhere*.

Historical context
------------------
Andrei Andreyevich Markov (1856–1922) introduced this model in 1906 while
studying the distribution of vowels and consonants in Pushkin's *Eugene Onegin*.
Claude Shannon (1948) later used it in *A Mathematical Theory of Communication*
to model English text as a statistical process. Since then the model has spread
into every quantitative field.

Design: DirectedGraph + _transitions
-------------------------------------
Internally, this implementation uses two complementary data structures:

  1. A ``DirectedGraph(allow_self_loops=True)`` (DT01) to track **topology**:
     which states can reach which other states. This lets us use graph
     algorithms like reachability, cycle detection, and SCC analysis.

  2. A ``_transitions`` dict for **probabilities**:
     ``_transitions[context][target] = probability``.
     The graph tells us *which* edges exist; this dict tells us *how likely*.

Why separate them? DirectedGraph stores edge weights, but weights in the
graph layer serve topology (e.g., unweighted for adjacency queries). Keeping
probabilities in a dedicated dict makes the probability logic self-contained
and avoids conflating graph-topology weights with stochastic weights.

Order-k chains
--------------
A standard (order-1) chain's next state depends only on the last 1 observed
state. An **order-k** chain extends the memory window to k states:

    P(next | last k states) = T[(s_{n-k}, …, s_{n-1})][s_n]

For order > 1, the "state" from the graph's perspective is a k-tuple of raw
states. The external API stays the same: ``train`` builds the counts, and
``next_state`` accepts either a single value (order=1) or a tuple (order>1).

Smoothing (Laplace / Lidstone)
-------------------------------
When ``smoothing > 0``, every possible (context, target) pair gets a
pseudo-count of ``smoothing`` added before normalisation. This prevents
zero-probability transitions from getting the chain permanently stuck.

    smoothed_count(i→j) = raw_count(i→j) + α
    T[i][j] = smoothed_count(i→j) / (total_raw + α * |all_known_states|)

Quick start::

    from coding_adventures_markov_chain import MarkovChain

    chain = MarkovChain(order=2, smoothing=0.5)
    chain.train_string("the quick brown fox jumps over the lazy dog " * 10)
    text = chain.generate_string("th", 100)
    print(text)           # "the quiche broth fox jumpse lazy…"

    dist = chain.stationary_distribution()
    print(sum(dist.values()))  # ≈ 1.0
"""

from __future__ import annotations

import random
from collections import defaultdict
from typing import Any

from directed_graph import DirectedGraph

__version__ = "0.1.0"

__all__ = ["MarkovChain"]


class MarkovChain:
    """General-purpose order-k Markov Chain with optional Laplace/Lidstone smoothing.

    A Markov Chain is trained on sequences of states and can then:
    - Sample the next state given the current context (``next_state``).
    - Generate sequences of arbitrary length (``generate``, ``generate_string``).
    - Compute the stationary distribution via power iteration.
    - Report transition probabilities.

    The chain uses a ``DirectedGraph`` internally for topology queries
    (reachability, cycle detection, strongly connected components) while
    keeping the probability table in a separate ``_transitions`` dict.

    Args:
        order:     Memory depth k. order=1 means next state depends only on
                   the last 1 state. order=2 uses the last 2 states as context.
        smoothing: Laplace/Lidstone pseudo-count α ≥ 0. 0.0 = no smoothing;
                   1.0 = standard Laplace smoothing; any α > 0 = Lidstone.
        states:    Optional pre-registered alphabet. States listed here are
                   included in the smoothing denominator even if never seen
                   in training. Useful when you know the full alphabet upfront.

    Example (weather model)::

        chain = MarkovChain()
        chain.train(["Sunny", "Cloudy", "Rainy", "Sunny", "Sunny", "Cloudy"])
        chain.probability("Sunny", "Cloudy")  # 0.5
        chain.generate("Sunny", 5)            # ["Sunny", "Cloudy", …]

    Example (order-2 character chain)::

        chain = MarkovChain(order=2, smoothing=0.1)
        chain.train_string("abcabcabc")
        chain.generate_string("ab", 9)  # "abcabcabc" (deterministic)
    """

    def __init__(
        self,
        order: int = 1,
        smoothing: float = 0.0,
        states: list | None = None,
    ) -> None:
        # k — how many previous states form the context key.
        # order=1 → context is a single state (most common case).
        # order=2 → context is a pair, e.g., ('a', 'b') → next char.
        self._order: int = order

        # α — pseudo-count added to every (context, target) pair when
        # normalising. 0.0 means no smoothing; 1.0 is classic Laplace.
        self._smoothing: float = smoothing

        # _raw_counts[context][target] = int — raw transition counts from
        # training. Kept separately so multiple train() calls accumulate
        # counts before re-normalising (as required by the spec).
        self._raw_counts: dict[Any, dict[Any, int]] = defaultdict(
            lambda: defaultdict(int)
        )

        # _transitions[context][target] = float — normalised probabilities.
        # Rebuilt from _raw_counts every time train() is called.
        # For order=1: context is a raw state (str, int, etc.).
        # For order>1: context is a tuple of k raw states.
        self._transitions: dict[Any, dict[Any, float]] = {}

        # _all_known_states — the universe of individual (atomic) states.
        # Includes both pre-registered states AND states seen in training.
        # Used as the denominator universe for smoothing.
        self._all_known_states: set[Any] = set()
        if states is not None:
            self._all_known_states.update(states)

        # _graph — DirectedGraph with allow_self_loops=True (a state can
        # transition to itself, e.g., "aa" in text). Used for topology
        # queries; edge existence tracks non-zero probability transitions.
        self._graph: DirectedGraph = DirectedGraph(allow_self_loops=True)

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, sequence: list) -> None:
        """Train the chain on a sequence of states.

        Slides a window of size (order + 1) over the sequence. For each
        window position i, the context is ``sequence[i]`` (order=1) or
        ``tuple(sequence[i:i+order])`` (order>1), and the target is
        ``sequence[i+order]``.

        Counts accumulate across multiple ``train()`` calls, so calling
        train twice doubles the training data before re-normalising.

        After counting, all rows are normalised using Laplace/Lidstone
        smoothing over the full ``_all_known_states`` universe.

        Args:
            sequence: A list of hashable states in observation order.

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])
            chain.probability("A", "B")  # ≈ 0.667
            chain.probability("A", "C")  # ≈ 0.333
        """
        # Register all unique states seen in this sequence as known states.
        # This grows _all_known_states monotonically — states are never removed.
        for item in sequence:
            self._all_known_states.add(item)

        # Slide a window of size (order + 1) over the sequence.
        # We need at least (order + 1) elements to get one transition.
        for i in range(len(sequence) - self._order):
            # Build the context key.
            if self._order == 1:
                # Fast path: context is the single element at position i.
                context: Any = sequence[i]
            else:
                # Slow path: context is an immutable k-tuple (hashable key).
                context = tuple(sequence[i : i + self._order])

            # The target is the element that follows the context window.
            target: Any = sequence[i + self._order]

            # Accumulate the raw count.
            self._raw_counts[context][target] += 1

        # Re-normalise the full table with smoothing.
        # We call _normalize() after every train() so the table stays fresh.
        self._normalize()

    def train_string(self, text: str) -> None:
        """Train the chain on a string, treating each character as a state.

        This is a convenience wrapper around ``train()`` for character-level
        Markov chains. Each character is an individual state (e.g., 'a',
        ' ', 't').

        Args:
            text: The string to train on. Every character becomes a state.

        Example::

            chain = MarkovChain(order=2)
            chain.train_string("abcabcabc")
            chain.generate_string("ab", 9)  # "abcabcabc"
        """
        # Convert the string to a list of single characters and delegate.
        self.train(list(text))

    def _normalize(self) -> None:
        """Rebuild ``_transitions`` from ``_raw_counts`` with smoothing.

        Called after every ``train()`` call. Iterates over every context
        that has been seen in training and computes the probability of each
        possible next state (target) from ``_all_known_states``.

        Laplace / Lidstone formula (α = self._smoothing):

            P(context → target) =
                (raw_count(context, target) + α)
                ────────────────────────────────
                (sum_of_raw_counts + α × |all_known_states|)

        When α = 0 (no smoothing), only observed transitions get non-zero
        probabilities. When α > 0, every state in the known universe gets
        at least α pseudo-counts, preventing zero probabilities.

        After building the probability table, we sync the ``_graph`` so
        every non-zero (context, target) pair has a corresponding directed
        edge. This keeps the graph's topology in sync with the probability
        table for graph algorithm queries.
        """
        n_states = len(self._all_known_states)

        # Rebuild the full transitions table from scratch.
        # We don't carry over old values — counts are the source of truth.
        new_transitions: dict[Any, dict[Any, float]] = {}

        for context, target_counts in self._raw_counts.items():
            # Sum of all raw counts from this context.
            # Used as the base for the denominator.
            raw_total = sum(target_counts.values())

            # Denominator includes a pseudo-count for every known state.
            # If smoothing=0, denominator = raw_total (pure MLE).
            denominator = raw_total + self._smoothing * n_states

            row: dict[Any, float] = {}

            if self._smoothing > 0.0:
                # Smoothed path: every known state gets at least α counts.
                # This iterates over all known states, so every target appears.
                for target in self._all_known_states:
                    raw = target_counts.get(target, 0)
                    prob = (raw + self._smoothing) / denominator
                    if prob > 0.0:
                        row[target] = prob
            else:
                # Unsmoothed path: only observed transitions have probability.
                # Faster — we only look at what was actually counted.
                for target, raw in target_counts.items():
                    row[target] = raw / denominator

            new_transitions[context] = row

        # Overwrite the transitions table atomically.
        self._transitions = new_transitions

        # Sync topology: ensure the directed graph reflects non-zero edges.
        # We only add edges (never remove) — once a transition is observed,
        # its edge persists for graph algorithm queries.
        for context, row in self._transitions.items():
            for target, prob in row.items():
                if prob > 0.0:
                    # For order=1, context is a raw state (the graph node).
                    # For order>1, context is a k-tuple (also a valid node).
                    if not self._graph.has_node(context):
                        self._graph.add_node(context)
                    if not self._graph.has_node(target):
                        self._graph.add_node(target)
                    if not self._graph.has_edge(context, target):
                        self._graph.add_edge(context, target)

    # ------------------------------------------------------------------
    # Sampling
    # ------------------------------------------------------------------

    def next_state(self, current: Any) -> Any:
        """Sample the next state given the current context.

        Uses the trained transition probabilities to pick the next state
        via categorical sampling (cumulative probability scan).

        For order=1: ``current`` is a single state.
        For order>1: ``current`` is a k-tuple of states (the last k states).

        Args:
            current: The current context (single state or k-tuple).

        Returns:
            The sampled next state.

        Raises:
            ValueError: If ``current`` is not a known context (never seen
                        in training and not pre-registered).

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "A", "C"])
            chain.next_state("A")   # "B" or "C" (stochastic)
            chain.next_state("X")   # raises ValueError
        """
        if current not in self._transitions:
            raise ValueError(
                f"Unknown state {current!r}. "
                f"This state was never seen in training. "
                f"Train the chain before calling next_state()."
            )

        # The probability row for this context.
        # It's a dict of {target: probability} that sums to 1.0.
        row = self._transitions[current]

        # Categorical sampling using a single uniform random draw.
        # We scan through the targets in insertion order, accumulating
        # probability mass. When the cumulative sum crosses ``r``, we
        # have found our sampled state.
        #
        # Why this works:
        #   Imagine the [0, 1] interval sliced into segments, one per target,
        #   each with width = probability. The uniform draw r falls into
        #   exactly one segment — that is our sample.
        r = random.random()
        cumulative = 0.0
        for target, prob in row.items():
            cumulative += prob
            if r < cumulative:
                return target

        # Floating point rounding can cause cumulative to barely miss 1.0.
        # Return the last target as a safe fallback — it has non-zero probability.
        return next(reversed(list(row.keys())))

    def generate(self, start: Any, length: int) -> list:
        """Generate a sequence of exactly ``length`` states.

        Starts from ``start`` and repeatedly calls ``next_state`` to extend
        the sequence. The result includes ``start`` as the first element.

        For order=1:
            ``start`` is a single state. Each step appends ``next_state(last)``.

        For order>1:
            ``start`` is a k-tuple (the initial context window). Each step
            appends ``next_state(window)`` and shifts the window forward by 1:
            new_window = (window[1:] + (next,)).

        Args:
            start:  Starting state (single value for order=1, k-tuple for order>1).
            length: Total number of elements in the returned list (includes start).

        Returns:
            A list of exactly ``length`` states.

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])
            seq = chain.generate("A", 10)
            len(seq)    # 10
            seq[0]      # "A"
        """
        if self._order == 1:
            # Order-1 path: result is a flat list of single states.
            # Start with the seed state and extend step by step.
            result = [start]
            current = start
            while len(result) < length:
                current = self.next_state(current)
                result.append(current)
        else:
            # Order-k path: start is a k-tuple (the context window).
            # We track the sliding window but emit only the individual states
            # from the window, then the newly generated states.
            #
            # Example (order=2, start=("a","b")):
            #   result = ["a", "b"]
            #   window = ("a", "b")  → next_state → "c"
            #   result = ["a", "b", "c"]
            #   window = ("b", "c")  → next_state → "a"
            #   ...
            result = list(start)  # Unpack the k-tuple as the initial states
            window = start  # Current context window (k-tuple)
            while len(result) < length:
                next_s = self.next_state(window)
                result.append(next_s)
                # Slide the window: drop the oldest element, add the newest.
                window = tuple(list(window[1:]) + [next_s])

        return result[:length]

    def generate_string(self, seed: str, length: int) -> str:
        """Generate a string of exactly ``length`` characters.

        A convenience wrapper around ``generate()`` for character-level chains.
        The ``seed`` provides the initial context window.

        Args:
            seed:   Starting characters. Must have at least ``order`` characters.
                    For order=1, the last character is the starting state.
                    For order>1, the last ``order`` characters form the context.
            length: Total number of characters in the returned string.

        Returns:
            A string of exactly ``length`` characters starting with ``seed``.

        Raises:
            ValueError: If ``seed`` has fewer than ``order`` characters.

        Example::

            chain = MarkovChain(order=2)
            chain.train_string("abcabcabc")
            chain.generate_string("ab", 9)   # "abcabcabc"
        """
        if len(seed) < self._order:
            raise ValueError(
                f"Seed {seed!r} is too short: need at least {self._order} "
                f"characters for an order-{self._order} chain."
            )

        if self._order == 1:
            # Order-1: start from the last character of the seed.
            # We pre-fill the result with the seed, then keep generating
            # until we reach the target length.
            start_char = seed[-1]
            result_chars = list(seed)
            # Generate enough additional characters.
            # We need (length - len(seed)) more characters if seed is shorter.
            extra_needed = length - len(seed)
            if extra_needed > 0:
                extra = self.generate(start_char, extra_needed + 1)
                # extra[0] == start_char (already in result), skip it.
                result_chars.extend(extra[1:])
            # Trim or pad: take exactly `length` chars starting from the end
            # of whatever we built, but the spec says start with the seed.
            # So we keep the seed at the front and trim total to `length`.
            return "".join(result_chars[:length])
        else:
            # Order-k: extract the last `order` characters as the initial window.
            start_window = tuple(seed[-self._order :])
            # Prefix the result with the seed characters that precede the window.
            prefix = list(seed[: -self._order])

            # generate() will return `length - len(prefix)` states total,
            # starting from the window.
            remaining = length - len(prefix)
            generated = self.generate(start_window, remaining)
            return "".join(prefix + generated)[:length]

    # ------------------------------------------------------------------
    # Probability queries
    # ------------------------------------------------------------------

    def probability(self, from_state: Any, to_state: Any) -> float:
        """Return the transition probability from ``from_state`` to ``to_state``.

        Returns 0.0 if:
        - ``from_state`` was never observed in training, OR
        - ``to_state`` was never observed as a successor of ``from_state``
          (and smoothing is 0).

        Args:
            from_state: The source context (single state for order=1,
                        k-tuple for order>1).
            to_state:   The target state.

        Returns:
            The probability in [0.0, 1.0].

        Example::

            chain.train(["A", "B", "A", "C"])
            chain.probability("A", "B")   # 0.5
            chain.probability("A", "Z")   # 0.0 (never seen)
        """
        # Graceful: return 0.0 for completely unknown contexts.
        if from_state not in self._transitions:
            return 0.0
        # Graceful: return 0.0 for unseen (context, target) pairs.
        return self._transitions[from_state].get(to_state, 0.0)

    # ------------------------------------------------------------------
    # Stationary distribution
    # ------------------------------------------------------------------

    def stationary_distribution(self) -> dict[Any, float]:
        """Compute the stationary distribution π via power iteration.

        The stationary distribution answers: "In the long run, what fraction
        of time does the chain spend in each state?" For an ergodic chain
        (all states mutually reachable, no periodic traps), there is a unique
        distribution π such that:

            π · T = π    (π is unchanged by one step of the chain)

        This is computed iteratively:

            π_new[s_j] = Σᵢ π[s_i] * T[s_i][s_j]

        until max(|π_new[s] - π[s]|) < 1e-10 or we exhaust 10,000 iterations.

        Only order-1 states are used (the atomic state space). For order>1
        chains, the context k-tuples are NOT the stationary states — the
        raw states are.

        Returns:
            A dict mapping each state to its long-run probability.

        Raises:
            ValueError: If the chain has no trained transitions, or if power
                        iteration does not converge (non-ergodic chain).

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "A", "B"])
            dist = chain.stationary_distribution()
            sum(dist.values())  # ≈ 1.0
        """
        if not self._transitions:
            raise ValueError(
                "Cannot compute stationary distribution: chain has no transitions. "
                "Call train() first."
            )

        # Gather the set of atomic states. For order=1 the context keys ARE
        # the states. For order>1, the context keys are k-tuples so we
        # extract individual elements from the target side.
        # We use _all_known_states which always contains atomic state values.
        raw_states = sorted(
            self._all_known_states, key=lambda s: str(s)
        )  # sorted for determinism
        n = len(raw_states)

        if n == 0:
            raise ValueError("Cannot compute stationary distribution: no known states.")

        # Initial distribution: uniform — every state equally likely.
        pi: dict[Any, float] = {s: 1.0 / n for s in raw_states}

        # Power iteration: apply T repeatedly until convergence.
        # We look up T[s_i][s_j] from _transitions.
        # For order=1, transitions keys are raw states, so this is direct.
        # For order>1, we skip this method (undefined for k-tuple contexts).
        for _ in range(10_000):
            pi_new: dict[Any, float] = {}
            for s_j in raw_states:
                # The new probability of being in s_j =
                #   sum over all s_i of (being in s_i) * P(s_i → s_j)
                mass = sum(
                    pi[s_i] * self._transitions.get(s_i, {}).get(s_j, 0.0)
                    for s_i in raw_states
                )
                pi_new[s_j] = mass

            # Check convergence: max absolute change across all states.
            max_delta = max(abs(pi_new[s] - pi[s]) for s in raw_states)
            if max_delta < 1e-10:
                return pi_new

            pi = pi_new

        raise ValueError(
            "Stationary distribution did not converge after 10,000 iterations. "
            "The chain may not be ergodic (some states may be unreachable from others, "
            "or the chain may be periodic)."
        )

    # ------------------------------------------------------------------
    # Inspection
    # ------------------------------------------------------------------

    def states(self) -> list:
        """Return the list of all known atomic states.

        Includes states that were:
        - Pre-registered in the constructor via the ``states`` argument, OR
        - Seen during training.

        For order>1, returns the raw atomic states (not the k-tuple contexts).

        Returns:
            A sorted list of all known states (sorted by str representation
            for determinism).

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "C"])
            chain.states()   # ["A", "B", "C"]
        """
        return sorted(self._all_known_states, key=lambda s: str(s))

    def transition_matrix(self) -> dict[Any, dict[Any, float]]:
        """Return a copy of the full transition probability table.

        The returned dict has the same structure as ``_transitions``:

            { context → { target → probability } }

        For order=1: context is a raw state.
        For order>1: context is a k-tuple.

        Returns a **copy** so callers cannot mutate the internal state.

        Returns:
            A dict mapping each context to its probability row.

        Example::

            chain = MarkovChain()
            chain.train(["A", "B", "A"])
            chain.transition_matrix()
            # {"A": {"B": 1.0}, "B": {"A": 1.0}}
        """
        # Deep-copy to prevent callers from accidentally mutating internals.
        return {context: dict(row) for context, row in self._transitions.items()}

    # ------------------------------------------------------------------
    # Dunder methods
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        n_states = len(self._all_known_states)
        n_contexts = len(self._transitions)
        return (
            f"MarkovChain("
            f"order={self._order}, "
            f"smoothing={self._smoothing}, "
            f"states={n_states}, "
            f"contexts={n_contexts})"
        )
