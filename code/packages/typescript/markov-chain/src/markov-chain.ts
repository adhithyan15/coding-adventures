/**
 * markov-chain.ts — General-Purpose Markov Chain (DT28)
 * ======================================================
 *
 * A **Markov Chain** models a system that moves between a finite set of
 * *states* over time, where the probability of the next state depends
 * *only* on the current state — not on any history of how we got here.
 * This "memorylessness" is the **Markov property**.
 *
 * Think of it like a board game where each square has a spinner: land on
 * square A and the spinner might say 70% chance → B, 30% chance → C. No
 * matter how many times you've visited A before, the spinner always shows
 * those same probabilities. Your history doesn't matter — only where you
 * are *right now*.
 *
 * A concrete example: English letter frequencies. After the letter "q",
 * "u" follows ~98% of the time. After "t", "h" follows about 30% of the
 * time. A Markov chain trained on a large text corpus captures exactly
 * these conditional probabilities, and can then *generate* plausible
 * text by sampling from them one letter at a time.
 *
 * ## Architecture Overview
 *
 * We split data across two structures:
 *
 * 1. `_graph` — a `Graph` from `@coding-adventures/directed-graph`.
 *    It tracks *topology*: which states exist, and which pairs have a
 *    non-zero transition probability. An edge `A → B` in the graph
 *    means `probability(A, B) > 0`.
 *
 * 2. `_transitions` — a `Map<string, Map<string, number>>` storing the
 *    actual probability values. The graph tells us *structure*; the map
 *    tells us *strength*.
 *
 * Why two structures? The graph gives us fast neighbor queries (what can
 * I reach from state X?) and topology algorithms (cycle detection, etc.)
 * for free, while the transitions map gives us O(1) probability lookups.
 *
 * ## Order-k Chains
 *
 * A standard (order-1) chain's next state depends only on the last 1
 * state. An **order-k** chain uses the last k states as context:
 *
 * ```
 * Order 1:  P(next | current)
 * Order 2:  P(next | last_2_states)
 * Order 3:  P(next | last_3_states)
 * ```
 *
 * For order > 1, we encode a k-gram context as a single string key by
 * joining the k states with a `\x00` (null byte) separator. This lets
 * us use the same `Map<string, Map<string, number>>` structure regardless
 * of order — the key just gets longer.
 *
 * ```
 * Order-2 context ["a", "b"] → key "a\x00b"
 * Order-3 context ["t", "h", "e"] → key "t\x00h\x00e"
 * ```
 *
 * ## Smoothing
 *
 * Without smoothing, unseen transitions have probability 0. This can
 * cause generation to get "stuck" in a state with no outgoing edges.
 * **Laplace / Lidstone smoothing** adds a small pseudo-count α to every
 * possible transition before normalising:
 *
 * ```
 *   smoothed_prob(i → j) = (count(i → j) + α) / (total_count(i) + α × |states|)
 * ```
 *
 * Setting α = 1.0 is classic Laplace smoothing. Setting α = 0.0 gives
 * maximum-likelihood estimates (no smoothing). We apply smoothing at
 * training time so every probability query is O(1).
 *
 * ## Connection to Directed Graph (DT01)
 *
 * The directed graph lets us ask structural questions like "is this chain
 * ergodic?" (does every state eventually reach every other state?) or
 * "which states are absorbing?" (states with no outgoing edges) — both of
 * which are important for stationary-distribution computation. Using the
 * graph as a substrate makes the Markov chain composable with graph
 * algorithms without re-implementing them.
 */

import { Graph } from "@coding-adventures/directed-graph";

// ---------------------------------------------------------------------------
// Internal Types
// ---------------------------------------------------------------------------

/**
 * Raw transition counts accumulated during training, before normalisation.
 *
 * We keep counts separate from probabilities so that multiple `train()`
 * calls accumulate correctly — we re-normalise only after all training data
 * is ingested.
 *
 * Structure: `Map<context_key, Map<target_state, raw_count>>`
 */
type CountTable = Map<string, Map<string, number>>;

// ---------------------------------------------------------------------------
// MarkovChain class
// ---------------------------------------------------------------------------

/**
 * A general-purpose Markov Chain with optional order-k context and smoothing.
 *
 * ### Quick start — order-1 character chain
 *
 * ```typescript
 * import { MarkovChain } from "@coding-adventures/markov-chain";
 *
 * const chain = new MarkovChain(1, 0.1);
 * chain.trainString("abcabcabc");
 *
 * // Generate 20 characters starting from "a"
 * const text = chain.generateString("a", 20);
 * // → "abcabcabcabcabcabcab" (or similar)
 * ```
 *
 * ### Order-2 word chain
 *
 * ```typescript
 * const chain = new MarkovChain(2);
 * chain.train(["the", "quick", "brown", "fox", "the", "quick", "red", "fox"]);
 * chain.generate("the\x00quick", 5);
 * // ["the", "quick", "brown", "fox", ...] or ["the", "quick", "red", "fox", ...]
 * ```
 */
export class MarkovChain {
  // ------------------------------------------------------------------
  // Configuration
  // ------------------------------------------------------------------

  /**
   * The order k of this Markov chain. Order 1 uses just the current state
   * as context; order k uses the last k states. Must be >= 1.
   */
  private readonly _order: number;

  /**
   * Laplace/Lidstone smoothing parameter α.
   *
   * - 0.0: no smoothing (maximum-likelihood estimates)
   * - 1.0: Laplace smoothing (add-one)
   * - α > 0: Lidstone smoothing (add-α)
   */
  private readonly _smoothing: number;

  // ------------------------------------------------------------------
  // Data Structures
  // ------------------------------------------------------------------

  /**
   * The directed graph tracking state topology.
   *
   * - Nodes = state keys (single states for order=1, joined k-grams for
   *   order>1, plus individual states from the alphabet).
   * - Edges = non-zero probability transitions (after smoothing).
   *
   * We use `Graph` with self-loops allowed because a state can transition
   * to itself (e.g., in a weather model "Sunny → Sunny" is common).
   */
  private readonly _graph: Graph;

  /**
   * The probability table.
   *
   * `_transitions.get(contextKey).get(targetState)` = P(target | context).
   *
   * Only populated after `_normalise()` is called (which happens at the
   * end of every `train()` call).
   */
  private readonly _transitions: Map<string, Map<string, number>> = new Map();

  /**
   * Raw counts accumulated during training, before normalisation.
   *
   * Kept separate from `_transitions` so multiple `train()` calls
   * accumulate correctly.
   */
  private readonly _counts: CountTable = new Map();

  /**
   * The canonical set of all known *individual* states (not k-gram keys).
   *
   * This is the alphabet Σ. Used in smoothing denominator and for
   * `states()` queries.
   */
  private readonly _alphabet: Set<string> = new Set();

  // ------------------------------------------------------------------
  // Constructor
  // ------------------------------------------------------------------

  /**
   * Create a new Markov chain.
   *
   * @param order     - Context window size k (default 1). Order 1 = standard
   *                    first-order Markov chain. Order 2 = last 2 states as
   *                    context. Must be >= 1.
   * @param smoothing - Laplace/Lidstone smoothing α (default 0.0). Set to
   *                    1.0 for add-one smoothing, or a small value like 0.1
   *                    for mild smoothing.
   * @param states    - Optional pre-registered alphabet. Useful when you know
   *                    the full state space in advance and want smoothing to
   *                    apply over ALL states even before training.
   *
   * @example
   * ```typescript
   * // Basic chain
   * const chain = new MarkovChain();
   *
   * // Order-2 with Laplace smoothing
   * const chain2 = new MarkovChain(2, 1.0);
   *
   * // Pre-register alphabet for proper smoothing
   * const chain3 = new MarkovChain(1, 1.0, ["A", "B", "C"]);
   * ```
   */
  constructor(
    order: number = 1,
    smoothing: number = 0.0,
    states?: string[]
  ) {
    if (order < 1) {
      throw new Error(`Order must be >= 1, got ${order}`);
    }
    if (smoothing < 0) {
      throw new Error(`Smoothing must be >= 0, got ${smoothing}`);
    }

    this._order = order;
    this._smoothing = smoothing;

    // Allow self-loops because a state CAN transition back to itself.
    // Example: "Sunny → Sunny" in a weather model.
    this._graph = new Graph({ allowSelfLoops: true });

    // Pre-register states if provided.
    if (states !== undefined) {
      for (const s of states) {
        this._alphabet.add(s);
        this._graph.addNode(s);
      }
    }
  }

  // ------------------------------------------------------------------
  // Training
  // ------------------------------------------------------------------

  /**
   * Train the chain on a sequence of states.
   *
   * We slide a window of size `order + 1` over the sequence:
   * - The first `order` elements form the context key.
   * - The last element is the target state.
   *
   * For order=1 and sequence ["A", "B", "A", "C"]:
   * ```
   * Window ["A", "B"] → context "A",  target "B",  count[A][B]++
   * Window ["B", "A"] → context "B",  target "A",  count[B][A]++
   * Window ["A", "C"] → context "A",  target "C",  count[A][C]++
   * ```
   *
   * After counting, we normalise each context row into probabilities
   * using the Laplace/Lidstone formula if `smoothing > 0`.
   *
   * Multiple `train()` calls **accumulate** counts — so you can call
   * `train()` in batches and the probabilities will reflect all batches.
   *
   * @param sequence - Array of state tokens to train on.
   */
  train(sequence: string[]): void {
    // Need at least order+1 elements to form one (context, target) pair.
    if (sequence.length < this._order + 1) {
      return;
    }

    // Slide the window over the sequence.
    // Window: indices [i .. i+order] inclusive.
    for (let i = 0; i <= sequence.length - this._order - 1; i++) {
      // The context is the first `order` elements of the window.
      const contextSlice = sequence.slice(i, i + this._order);
      const contextKey = contextSlice.join("\x00");

      // The target is the element immediately after the context.
      const target = sequence[i + this._order];

      // Register all encountered states in the alphabet.
      for (const s of contextSlice) {
        this._alphabet.add(s);
        this._graph.addNode(s);
      }
      this._alphabet.add(target);
      this._graph.addNode(target);

      // Increment the count for this (context, target) pair.
      if (!this._counts.has(contextKey)) {
        this._counts.set(contextKey, new Map());
      }
      const row = this._counts.get(contextKey)!;
      row.set(target, (row.get(target) ?? 0) + 1);
    }

    // Re-normalise all rows to probabilities after updating counts.
    this._normalise();
  }

  /**
   * Train the chain on a plain string, treating each character as a state.
   *
   * This is a convenience wrapper around `train()` for character-level
   * text models.
   *
   * @param text - The training text. Each character becomes one state token.
   *
   * @example
   * ```typescript
   * chain.trainString("the quick brown fox");
   * chain.generateString("t", 30);
   * ```
   */
  trainString(text: string): void {
    this.train(Array.from(text));
  }

  // ------------------------------------------------------------------
  // Internal normalisation
  // ------------------------------------------------------------------

  /**
   * Convert raw counts in `_counts` into probability distributions in
   * `_transitions`, applying Laplace/Lidstone smoothing.
   *
   * After this call, `_transitions.get(ctx).get(target)` gives the
   * probability of transitioning from context `ctx` to state `target`.
   *
   * ### Smoothing formula
   *
   * For context `ctx`, target `t`, smoothing α, and alphabet size |Σ|:
   *
   * ```
   * smoothed_count(t) = raw_count(ctx → t) + α
   * denominator       = sum_k(raw_count(ctx → k)) + α × |Σ|
   * prob(ctx → t)     = smoothed_count(t) / denominator
   * ```
   *
   * When α = 0, this reduces to plain maximum-likelihood: prob = count / total.
   * When α = 1, this is Laplace smoothing (add-one).
   *
   * We only store non-zero probabilities in `_transitions` (so if α=0
   * and count=0, we don't store that transition). We do, however, add the
   * graph edge only for non-zero transitions, so the graph topology stays
   * accurate.
   *
   * The graph edges are rebuilt from scratch on each normalisation to
   * reflect the current probability table accurately.
   */
  private _normalise(): void {
    const alphabetSize = this._alphabet.size;

    for (const [ctx, targetCounts] of this._counts) {
      // Total raw count for this context row.
      let rawTotal = 0;
      for (const c of targetCounts.values()) {
        rawTotal += c;
      }

      // Denominator includes smoothing pseudo-counts for ALL known states.
      const denominator = rawTotal + this._smoothing * alphabetSize;

      // Build the new probability row.
      const probRow: Map<string, number> = new Map();

      if (this._smoothing > 0) {
        // Smoothing ON: every alphabet state gets a non-zero probability.
        for (const state of this._alphabet) {
          const rawCount = targetCounts.get(state) ?? 0;
          const smoothedCount = rawCount + this._smoothing;
          const prob = smoothedCount / denominator;
          probRow.set(state, prob);
        }
      } else {
        // Smoothing OFF: only states with observed transitions get non-zero prob.
        for (const [state, count] of targetCounts) {
          probRow.set(state, count / rawTotal);
        }
      }

      this._transitions.set(ctx, probRow);

      // Update graph topology for the context key node → each target.
      // For order=1, ctx is a single state and the node is that state.
      // For order>1, ctx is a joined k-gram (we add a "virtual" node for it).
      if (!this._graph.hasNode(ctx)) {
        this._graph.addNode(ctx);
      }
      for (const [target, prob] of probRow) {
        if (prob > 0) {
          if (!this._graph.hasEdge(ctx, target)) {
            this._graph.addEdge(ctx, target);
          }
        }
      }
    }
  }

  // ------------------------------------------------------------------
  // Querying
  // ------------------------------------------------------------------

  /**
   * Sample the next state from the current state (or context key).
   *
   * For order=1, pass a single state string.
   * For order>1, pass the joined context key (k states joined by `\x00`).
   *
   * Uses weighted random sampling over the transition row:
   *
   * ```
   * Transition row for "A": { "B": 0.7, "C": 0.3 }
   * r = Math.random()  (uniform in [0, 1))
   * cumulative = 0
   * for each (state, prob) in row:
   *   cumulative += prob
   *   if r < cumulative: return state  ← first state whose CDF ≥ r
   * ```
   *
   * This is **inverse-CDF sampling** (also called the roulette-wheel method).
   * It correctly handles any number of states and any probability distribution.
   *
   * @param current - The current state or context key.
   * @throws {Error} If `current` is not in the transition table.
   */
  nextState(current: string): string {
    const row = this._transitions.get(current);
    if (row === undefined || row.size === 0) {
      throw new Error(
        `Unknown state: "${current}". Train the chain first or check for typos.`
      );
    }

    // Inverse-CDF (roulette-wheel) sampling.
    const r = Math.random();
    let cumulative = 0;
    let lastState = "";
    for (const [state, prob] of row) {
      cumulative += prob;
      lastState = state;
      if (r < cumulative) {
        return state;
      }
    }

    // Floating-point rounding can push cumulative just under 1.0, so we
    // fall through to the last state rather than throwing an error.
    return lastState;
  }

  /**
   * Generate a sequence of exactly `length` states.
   *
   * For order=1:
   * - `start` is the first state. The result includes `start` as the
   *   first element and grows by sampling `length - 1` more transitions.
   *
   * For order>1:
   * - `start` is the initial context key (k states joined by `\x00`).
   * - The last state of the context is the first element of the result.
   * - We then slide the window forward `length - 1` times.
   *
   * Example (order=2, start="a\x00b", length=5):
   * ```
   * Context: "a\x00b"
   * Result so far: ["b"]   ← last char of context
   * Sample next from "a\x00b" → "c", result: ["b", "c"]
   * Context slides: "b\x00c", sample → "a", result: ["b", "c", "a"]
   * etc.
   * ```
   *
   * Wait — the spec says `generate(start, length)` returns exactly
   * `length` items. For order=1, start is item 1 and we sample length-1
   * more. For order>1, start is the context key and we produce length
   * new samples from it (the context's last element is item 1).
   *
   * @param start  - Initial state (order=1) or context key (order>1).
   * @param length - Exact number of items to return.
   * @returns Array of exactly `length` state strings.
   */
  generate(start: string, length: number): string[] {
    if (length <= 0) {
      return [];
    }

    if (this._order === 1) {
      // Order-1: start is the first state in the result.
      const result: string[] = [start];
      let current = start;

      for (let i = 1; i < length; i++) {
        current = this.nextState(current);
        result.push(current);
      }

      return result;
    } else {
      // Order-k (k > 1): start is the context key "s0\x00s1\x00...\x00s_{k-1}".
      // The last state in the context key is the first output element.
      const contextParts = start.split("\x00");
      const result: string[] = [contextParts[contextParts.length - 1]];
      let contextKey = start;

      for (let i = 1; i < length; i++) {
        const next = this.nextState(contextKey);
        result.push(next);

        // Slide the window: drop the oldest context state, add the new one.
        const parts = contextKey.split("\x00");
        parts.shift();
        parts.push(next);
        contextKey = parts.join("\x00");
      }

      return result;
    }
  }

  /**
   * Generate a string of exactly `length` characters.
   *
   * A convenience wrapper for character-level Markov chains.
   *
   * For order=1: `seed` is the starting character (1 char minimum).
   * For order=k: `seed` is the k-character starting context.
   *
   * The seed itself is **not** included in the output — we generate
   * `length` *new* characters from the seed context.
   *
   * Wait — the spec says `generateString("ab", 9) === "abcabcabc"` for
   * an order-2 chain trained on "abcabcabc". The seed "ab" must appear
   * at the start of the output. So we include the seed in the output.
   *
   * Algorithm:
   * 1. Use the last `order` chars of `seed` as the initial context key.
   * 2. Call `generate(contextKey, length - (order - 1))` to get enough
   *    states to fill the result.
   * 3. Prepend the leading part of the seed to reconstruct exactly
   *    `length` characters.
   *
   * For order=1, seed="a", length=5: output is "a" + 4 sampled chars.
   * For order=2, seed="ab", length=9: output is "ab" + 7 sampled chars.
   *
   * @param seed   - Starting context. Length must be >= `order`.
   * @param length - Exact number of characters to return.
   * @returns String of exactly `length` characters.
   *
   * @throws {Error} If `seed` is shorter than `order`.
   */
  generateString(seed: string, length: number): string {
    if (seed.length < this._order) {
      throw new Error(
        `Seed "${seed}" must be at least ${this._order} character(s) for order=${this._order}`
      );
    }
    if (length <= 0) {
      return "";
    }

    // Extract the last `order` characters from the seed as the initial context.
    const initialContext = seed.slice(seed.length - this._order);

    // The prefix is everything in seed BEFORE the LAST character.
    // Why before the last character? Because `generate(contextKey, n)` always
    // starts its output from the last character of the context key. So:
    //
    // Order=1, seed="t":
    //   prefix = ""  (nothing before last char)
    //   contextKey = "t"
    //   generate("t", 9) → ["t", next1, ..., next8]   ← "t" IS in the output
    //   result = "" + "tnext1...next8"  = 9 chars  ✓
    //
    // Order=2, seed="ab":
    //   prefix = "a"  (everything before last char)
    //   contextKey = "a\x00b"
    //   generate("a\x00b", 8) → ["b", "c", "a", "b", "c", "a", "b", "c"]  8 items
    //   result = "a" + "bcabcabc" = "abcabcabc"  (9 chars)  ✓
    //
    // Order=3, seed="abc":
    //   prefix = "ab"
    //   contextKey = "a\x00b\x00c"
    //   generate("a\x00b\x00c", length-2) starts with "c"
    //   result = "ab" + "c..." = length chars  ✓
    const prefix = seed.slice(0, seed.length - 1);

    // We need `length - prefix.length` items from generate (which starts from
    // the last char of the context and extends forward).
    const toGenerate = length - prefix.length;
    if (toGenerate <= 0) {
      return seed.slice(0, length);
    }

    // Form the context key for order-k generation.
    const contextKey =
      this._order === 1
        ? initialContext
        : Array.from(initialContext).join("\x00");

    // `generate` returns a sequence starting from the last char of context.
    // For order=1, generate("a", n) → ["a", next1, next2, ...]
    // For order=2, generate("a\x00b", n) → ["b", next1, next2, ...]
    const generated = this.generate(contextKey, toGenerate);

    return prefix + generated.join("");
  }

  // ------------------------------------------------------------------
  // Probability queries
  // ------------------------------------------------------------------

  /**
   * Return the probability of transitioning from state `from` to `to`.
   *
   * Returns `0.0` if either state is unknown or the transition has never
   * been observed (and smoothing is 0).
   *
   * For order>1, `from` should be the joined context key.
   *
   * @param from - Source state or context key.
   * @param to   - Target state.
   */
  probability(from: string, to: string): number {
    const row = this._transitions.get(from);
    if (row === undefined) {
      return 0.0;
    }
    return row.get(to) ?? 0.0;
  }

  // ------------------------------------------------------------------
  // Stationary distribution
  // ------------------------------------------------------------------

  /**
   * Compute the stationary distribution π via power iteration.
   *
   * The stationary distribution answers: "In the long run, what fraction
   * of time does the chain spend in each state?"
   *
   * Mathematically, π is a probability vector satisfying `π · T = π`,
   * i.e., it is a left eigenvector of the transition matrix with
   * eigenvalue 1.
   *
   * ### Power Iteration Algorithm
   *
   * Start with a uniform distribution (each state gets weight 1/n).
   * Repeatedly multiply by the transition matrix until the distribution
   * stops changing:
   *
   * ```
   * π₀ = { s: 1/n for s in states }
   * loop:
   *   π_new[s_j] = Σ_{s_i} π[s_i] × T[s_i][s_j]
   *   if max |π_new[s] - π[s]| < 1e-10: break
   *   π = π_new
   * ```
   *
   * This converges for **ergodic** chains — chains where every state is
   * reachable from every other state (irreducible) and there are no
   * periodic traps (aperiodic). For non-ergodic chains, the iteration
   * may not converge to a unique distribution.
   *
   * We run at most 10,000 iterations to prevent infinite loops on
   * non-ergodic chains.
   *
   * @returns Map from state → probability (all values sum to ≈ 1.0).
   * @throws {Error} If the chain has no trained states.
   */
  stationaryDistribution(): Map<string, number> {
    // We compute the stationary distribution over the individual states
    // in the alphabet (not over k-gram context keys).
    const stateList = Array.from(this._alphabet);
    const n = stateList.length;

    if (n === 0) {
      throw new Error(
        "Cannot compute stationary distribution: chain has no states."
      );
    }

    // Start with a uniform distribution.
    // Map each state to its current probability estimate.
    let pi: Map<string, number> = new Map();
    for (const s of stateList) {
      pi.set(s, 1 / n);
    }

    const MAX_ITER = 10_000;
    const TOLERANCE = 1e-10;

    for (let iter = 0; iter < MAX_ITER; iter++) {
      const piNew: Map<string, number> = new Map();
      for (const sj of stateList) {
        piNew.set(sj, 0);
      }

      // π_new[sj] = Σ_{si} π[si] × T[si][sj]
      // T[si][sj] is stored in _transitions under the key `si` (for order=1).
      for (const si of stateList) {
        const piSi = pi.get(si) ?? 0;
        if (piSi === 0) continue;

        const row = this._transitions.get(si);
        if (row === undefined) continue;

        for (const sj of stateList) {
          const tij = row.get(sj) ?? 0;
          piNew.set(sj, (piNew.get(sj) ?? 0) + piSi * tij);
        }
      }

      // Check convergence: max absolute change across all states.
      let maxDelta = 0;
      for (const s of stateList) {
        const delta = Math.abs((piNew.get(s) ?? 0) - (pi.get(s) ?? 0));
        if (delta > maxDelta) {
          maxDelta = delta;
        }
      }

      pi = piNew;

      if (maxDelta < TOLERANCE) {
        break;
      }
    }

    return pi;
  }

  // ------------------------------------------------------------------
  // Inspection
  // ------------------------------------------------------------------

  /**
   * Return the list of all known individual states in the alphabet.
   *
   * For order-1 chains this is every state ever seen in training.
   * For order-k chains this is still the individual tokens — not the
   * joined k-gram context keys.
   */
  states(): string[] {
    return Array.from(this._alphabet);
  }

  /**
   * Return the full transition probability table.
   *
   * For order=1, keys are individual states.
   * For order>1, keys are joined context strings (k states joined by `\x00`).
   *
   * The returned map is a snapshot — mutating it does not affect the chain.
   */
  transitionMatrix(): Map<string, Map<string, number>> {
    // Return a deep copy so callers can't accidentally mutate internal state.
    const copy: Map<string, Map<string, number>> = new Map();
    for (const [ctx, row] of this._transitions) {
      copy.set(ctx, new Map(row));
    }
    return copy;
  }
}
