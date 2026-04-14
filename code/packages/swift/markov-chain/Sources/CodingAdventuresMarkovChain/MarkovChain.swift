// ============================================================================
// MarkovChain.swift — A General-Purpose Markov Chain
// ============================================================================
//
// ## What is a Markov Chain?
//
// A Markov Chain is a mathematical model for systems that move between a
// finite set of **states** over time, where the probability of the next state
// depends *only* on the current state — not on the full history. This is the
// **Markov property**, informally called "memorylessness."
//
// Think of it like a board game with weighted dice: standing on square A,
// you roll and move to B with probability 0.7 or C with probability 0.3.
// Where you came from doesn't matter — only where you are now.
//
//     ┌──────────────────────────────────────────┐
//     │  States:  {Sunny, Cloudy, Rainy}         │
//     │                                          │
//     │   Sunny ──0.7──▶ Sunny                  │
//     │   Sunny ──0.2──▶ Cloudy                 │
//     │   Sunny ──0.1──▶ Rainy                  │
//     │   Cloudy ──0.3──▶ Sunny                 │
//     │   Cloudy ──0.4──▶ Cloudy                │
//     │   Cloudy ──0.3──▶ Rainy                 │
//     │   Rainy ──0.2──▶ Sunny                  │
//     │   Rainy ──0.3──▶ Cloudy                 │
//     │   Rainy ──0.5──▶ Rainy                  │
//     └──────────────────────────────────────────┘
//
// Each row of the **transition matrix** sums to 1.0 because the chain must
// go *somewhere* (even if it stays in the same state).
//
// ## Where Do Markov Chains Appear?
//
//   Text generation   — states = characters or words; train on a corpus and
//                       sample new text by following transitions.
//   PageRank          — states = web pages; the stationary distribution gives
//                       each page's long-run visitation frequency.
//   Compression       — LZMA uses the current byte as a state and transition
//                       probabilities to drive a range coder.
//   Biology           — nucleotide sequences (A, C, G, T); CpG island models.
//   Game AI           — NPC mood transitions, procedural narrative.
//
// ## Order-k Chains
//
// A standard (order-1) chain uses only the *last* observed state as context.
// An order-k chain uses the last *k* states:
//
//   Order 1:  "e"  → next-letter probabilities
//   Order 2:  "th" → next-letter probabilities (much more realistic text)
//   Order 3:  "the"→ next-letter probabilities (nearly verbatim at small scale)
//
// We represent the k-gram context as a single String key by joining the k
// states with a null byte ("\0") separator — a character that won't appear in
// normal text and is safe as a key component.
//
// ## Design Overview
//
//   • `graph`       — a `DirectedGraph.Graph` tracking which (context, target)
//                     transitions are topologically possible (edges exist).
//   • `counts`      — raw observation counts, accumulated across `train` calls.
//   • `transitions` — normalised probability table, recomputed after training.
//   • `allStates`   — the unique base states seen (not k-gram keys).
//   • `smoothing`   — Lidstone parameter α; 0 = no smoothing, 1 = Laplace.
//   • `order`       — the k in order-k.
//
// The separation of `counts` from `transitions` lets you call `train` multiple
// times (accumulating evidence) before the final probabilities are needed.
//
// ============================================================================

import DirectedGraph

// ============================================================================
// MARK: - Error Types
// ============================================================================

/// Errors that the MarkovChain can throw.
///
/// Using a dedicated error enum (rather than generic `Error` strings) lets
/// callers match on specific cases and give the user precise diagnostics.
public enum MarkovError: Error, CustomStringConvertible {

    /// The requested state has never been seen during training and is not
    /// in the initial `states` list passed to the constructor.
    case unknownState(String)

    /// The chain is not **ergodic** — it has unreachable states or absorbing
    /// traps, so there is no unique stationary distribution.
    ///
    /// Ergodicity requires every state to be reachable from every other state
    /// (the underlying directed graph is strongly connected).
    case notErgodic

    /// `generateString` was called with a seed shorter than the chain's order.
    ///
    /// An order-k chain needs k seed characters to form the initial context key.
    case seedTooShort(required: Int, got: Int)

    public var description: String {
        switch self {
        case .unknownState(let s):
            return "Unknown state: '\(s)'"
        case .notErgodic:
            return "Chain is not ergodic — stationary distribution does not exist"
        case .seedTooShort(let required, let got):
            return "Seed too short: need \(required) characters, got \(got)"
        }
    }
}

// ============================================================================
// MARK: - MarkovChain
// ============================================================================

/// A general-purpose Markov Chain over `String` states.
///
/// ## Quick Start
///
///     var chain = MarkovChain(order: 1, smoothing: 0.0)
///     chain.train(["A", "B", "A", "C", "A", "B"])
///     let next = try chain.nextState("A")    // samples "B" or "C"
///     let seq  = try chain.generate(start: "A", length: 5)
///
/// ## Character-level text generation
///
///     var chain = MarkovChain(order: 2)
///     chain.trainString("the quick brown fox")
///     let text = try chain.generateString(seed: "th", length: 40)
///
/// ## Value Semantics
///
/// `MarkovChain` is a `struct`, so it has value semantics. Assigning it to a
/// new variable copies all state. Mutating methods (`train`, `trainString`)
/// are marked `mutating`.
public struct MarkovChain {

    // -----------------------------------------------------------------------
    // MARK: Stored Properties
    // -----------------------------------------------------------------------

    /// The k in "order-k chain." k=1 is the standard Markov property.
    public let order: Int

    /// Lidstone smoothing parameter α.
    ///
    ///   α = 0.0  → no smoothing (zero-probability transitions stay zero)
    ///   α = 1.0  → Laplace smoothing (each unseen transition counts as 1)
    ///   α > 0    → Lidstone smoothing (fractional pseudo-counts)
    public let smoothing: Double

    /// Directed graph representing topology: nodes = context keys (k-gram
    /// strings), edges = observed or possible transitions.
    ///
    /// We keep this to track which transitions have ever been observed
    /// (for the `stationaryDistribution` ergodicity check).
    private var graph: Graph

    /// Raw observation counts: `counts[contextKey][targetState]`.
    ///
    /// `contextKey` is `sequence[i..<i+order].joined(separator:"\0")`.
    /// These are the *unnormalised* frequencies; we normalise into
    /// `transitions` after each training pass.
    private var counts: [String: [String: Double]]

    /// Normalised transition probabilities: `transitions[contextKey][targetState]`.
    ///
    /// Always in [0, 1]; rows sum to 1.0 (modulo floating-point rounding).
    private var transitions: [String: [String: Double]]

    /// All unique **base** states ever seen (not k-gram keys).
    ///
    /// Used to enumerate the state space when computing smoothed denominators
    /// and stationary distributions.
    private var allStates: [String]

    // -----------------------------------------------------------------------
    // MARK: Initialiser
    // -----------------------------------------------------------------------

    /// Create an empty Markov Chain.
    ///
    /// - Parameters:
    ///   - order:     The memory depth. Order 1 = standard Markov property
    ///                (next state depends only on current state).
    ///   - smoothing: Lidstone smoothing parameter α. Use 0 for no smoothing,
    ///                1 for Laplace. Defaults to 0.
    ///   - states:    Optional pre-declared state vocabulary. Useful when you
    ///                know the full alphabet in advance (e.g. {"A","B","C"})
    ///                so that smoothing denominator is computed correctly even
    ///                before any training.
    public init(order: Int = 1, smoothing: Double = 0.0, states: [String] = []) {
        self.order = order
        self.smoothing = smoothing
        self.graph = Graph()
        self.counts = [:]
        self.transitions = [:]
        // De-duplicate while preserving insertion order.
        var seen = Set<String>()
        self.allStates = states.filter { seen.insert($0).inserted }
        // Register each declared state as a node in the graph.
        for state in self.allStates {
            self.graph.addNode(state)
        }
    }

    // -----------------------------------------------------------------------
    // MARK: Training
    // -----------------------------------------------------------------------

    /// Train the chain on a sequence of states.
    ///
    /// This slides a window of size `order + 1` over the sequence:
    ///
    ///     sequence = [A, B, A, C, A, B]   (order=1)
    ///     windows:   (A→B), (B→A), (A→C), (C→A), (A→B)
    ///
    /// For order=2, windows are size 3:
    ///
    ///     sequence = [a, b, c, a, b, c]   (order=2)
    ///     windows:   (ab→c), (bc→a), (ca→b), (ab→c)   (4 windows)
    ///
    /// Counts accumulate: calling `train` multiple times is equivalent to
    /// calling it once with the concatenation of all sequences.
    ///
    /// - Parameter sequence: Ordered list of states to learn from.
    public mutating func train(_ sequence: [String]) {
        guard sequence.count > order else { return }

        // Register any new states into the vocabulary.
        for state in sequence {
            if !allStates.contains(state) {
                allStates.append(state)
                graph.addNode(state)
            }
        }

        // Slide the window across the sequence.
        // Window: sequence[i ..< i+order] → context key, sequence[i+order] → target.
        for i in 0...(sequence.count - order - 1) {
            let contextStates = Array(sequence[i..<(i + order)])
            let contextKey = contextStates.joined(separator: "\0")
            let target = sequence[i + order]

            // Increment the raw count for this (context → target) transition.
            if counts[contextKey] == nil {
                counts[contextKey] = [:]
            }
            counts[contextKey]![target, default: 0.0] += 1.0

            // Record the edge in the topology graph (safe to call redundantly).
            // The context key and target are both "nodes" in the graph from a
            // topological perspective. We use target as node name for simplicity.
            graph.addNode(contextKey)
            try? graph.addEdge(from: contextKey, to: target)
        }

        // After updating counts, re-normalise all rows to probabilities.
        recomputeTransitions()
    }

    /// Train the chain on a string, treating each character as a state.
    ///
    /// Convenience wrapper around `train(_:)` that converts a String into
    /// an array of single-character strings:
    ///
    ///     chain.trainString("abc")
    ///     // equivalent to: chain.train(["a", "b", "c"])
    ///
    /// - Parameter text: The text to learn from.
    public mutating func trainString(_ text: String) {
        train(text.map { String($0) })
    }

    // -----------------------------------------------------------------------
    // MARK: Private: Normalisation
    // -----------------------------------------------------------------------

    /// Convert raw counts into normalised probabilities with optional smoothing.
    ///
    /// For each context key, we apply Lidstone smoothing:
    ///
    ///     smoothed_count(context → target) = raw_count(context → target) + α
    ///     P(target | context) = smoothed_count / Σ_t smoothed_count(context → t)
    ///
    /// where the sum is over *all* known base states (not just those seen
    /// from this context). This ensures rows sum to 1.0 and avoids the chain
    /// getting stuck due to zero-probability transitions (when α > 0).
    ///
    /// The denominator is:
    ///
    ///     Σ raw_counts + α * |allStates|
    ///
    /// because there are |allStates| possible targets, each getting α added.
    private mutating func recomputeTransitions() {
        transitions = [:]
        let n = Double(allStates.count)

        for (contextKey, targetCounts) in counts {
            let rawTotal = targetCounts.values.reduce(0, +)
            let denominator = rawTotal + smoothing * n

            if denominator == 0 {
                continue
            }

            var row: [String: Double] = [:]
            for state in allStates {
                let rawCount = targetCounts[state, default: 0.0]
                let smoothedCount = rawCount + smoothing
                row[state] = smoothedCount / denominator
            }
            transitions[contextKey] = row
        }
    }

    // -----------------------------------------------------------------------
    // MARK: Sampling
    // -----------------------------------------------------------------------

    /// Sample the next state from the given current state.
    ///
    /// Internally this uses **CDF (inverse transform) sampling**: we accumulate
    /// probabilities from the current state's row until we exceed a uniformly
    /// random threshold:
    ///
    ///     row = {A: 0.5, B: 0.3, C: 0.2}
    ///     r   = 0.65
    ///     cum 0.5 < 0.65 → keep going
    ///     cum 0.8 ≥ 0.65 → return "B"
    ///
    /// - Parameter current: The current state.
    /// - Returns: The sampled next state.
    /// - Throws: `MarkovError.unknownState` if `current` was never trained on.
    public func nextState(_ current: String) throws -> String {
        // For order-1 chains, the context key IS the current state.
        let contextKey = current

        guard let row = transitions[contextKey] else {
            throw MarkovError.unknownState(current)
        }

        return sampleFromRow(row)
    }

    /// Sample a next state from a context key (for order-k chains).
    ///
    /// For order > 1, a context key is a null-separated k-gram (e.g. "a\0b").
    /// If the exact k-gram context was never seen as a *source* in training
    /// (it appeared only at the very end of a sequence, making it a target
    /// but never a source), we back off to a shorter suffix of the context
    /// before giving up. This is called **back-off smoothing** — a common
    /// technique in language models that gracefully handles sparse contexts
    /// by using less-specific (but known) contexts as a fallback.
    ///
    /// Back-off order for a 2-gram context "a\0b":
    ///   1. Try exact context "a\0b"
    ///   2. Try last 1-gram "b"         (drop leading state)
    ///   3. If smoothing > 0, sample uniformly from all known states
    ///   4. Throw unknownState
    ///
    /// The uniform fallback in step 3 is justified when smoothing > 0: the
    /// chain was constructed with the expectation that every state should
    /// have some probability of transitioning anywhere. Contexts that only
    /// appear at the tail of training sequences (never as a source) would
    /// get Laplace-smoothed rows if we re-ran normalisation over them, but
    /// since no counts were ever recorded we synthesise the uniform row here.
    ///
    /// - Parameter contextKey: The null-separated k-gram key.
    /// - Returns: A sampled next state.
    /// - Throws: `MarkovError.unknownState` if no back-off level finds a row
    ///           and smoothing is 0.
    private func nextStateForContext(_ contextKey: String) throws -> String {
        // Try exact context first.
        if let row = transitions[contextKey] {
            return sampleFromRow(row)
        }

        // Back-off: progressively drop the leading state from the context.
        var parts = contextKey.split(separator: "\0", omittingEmptySubsequences: false).map(String.init)
        while parts.count > 1 {
            parts.removeFirst()
            let shorterKey = parts.joined(separator: "\0")
            if let row = transitions[shorterKey] {
                return sampleFromRow(row)
            }
        }

        // If smoothing > 0, synthesise a uniform distribution over all known
        // states as a final fallback. This handles "tail" contexts that were
        // observed only as targets (never as sources) during training.
        if smoothing > 0 && !allStates.isEmpty {
            let uniformProb = 1.0 / Double(allStates.count)
            var uniformRow: [String: Double] = [:]
            for state in allStates {
                uniformRow[state] = uniformProb
            }
            return sampleFromRow(uniformRow)
        }

        // All back-off levels exhausted and no smoothing — state is truly unknown.
        let lastState = parts.last ?? contextKey
        throw MarkovError.unknownState(lastState)
    }

    /// Perform CDF sampling from a probability row.
    ///
    /// The row maps state names to probabilities that sum to ~1.0. We draw a
    /// uniform random number and walk the cumulative distribution until we
    /// exceed it. The states are sorted for deterministic behaviour given the
    /// same RNG seed.
    ///
    /// - Parameter row: A dictionary of `state → probability`.
    /// - Returns: The sampled state.
    private func sampleFromRow(_ row: [String: Double]) -> String {
        var rng = SystemRandomNumberGenerator()
        // Draw a uniform random Double in [0, 1).
        // `Double.random(in:using:)` draws from [0, 1).
        let r = Double.random(in: 0.0..<1.0, using: &rng)

        // Sort states for determinism (same RNG value → same result regardless
        // of dictionary iteration order, which is not guaranteed in Swift).
        let sorted = row.sorted { $0.key < $1.key }

        var cumulative = 0.0
        for (state, prob) in sorted {
            cumulative += prob
            if r < cumulative {
                return state
            }
        }

        // Fallback: floating-point rounding can leave cumulative slightly < 1.
        // Return the last state rather than throwing.
        return sorted.last!.key
    }

    // -----------------------------------------------------------------------
    // MARK: Generation
    // -----------------------------------------------------------------------

    /// Generate a sequence of states of the given length, starting from `start`.
    ///
    /// For order-1 chains, `start` is the first state and the chain follows
    /// transitions from there:
    ///
    ///     generate(start: "A", length: 4) → [A, B, A, C]
    ///
    /// For order-k chains (k > 1), `start` is treated as a single state that
    /// seeds the initial context. The context window slides forward one step
    /// at a time:
    ///
    ///     order=2, generate(start: "a", length: 4):
    ///       initial context = ["a", ?]  ← needs k states; see generateString
    ///       This method works for order-1; use generateString for order-k > 1.
    ///
    /// - Parameters:
    ///   - start:  The first state in the output sequence.
    ///   - length: Total number of states to return (including `start`).
    /// - Returns: Array of `length` states.
    /// - Throws: `MarkovError.unknownState` if any visited state has no transitions.
    public func generate(start: String, length: Int) throws -> [String] {
        guard length > 0 else { return [] }

        var result: [String] = [start]
        var contextWindow: [String] = [start]

        // For order > 1, we may not have enough context yet; pad with the start
        // state until the window reaches the required depth.
        while contextWindow.count < order {
            contextWindow.append(start)
        }

        for _ in 1..<length {
            let contextKey = contextWindow.suffix(order).joined(separator: "\0")
            let next = try nextStateForContext(contextKey)
            result.append(next)
            contextWindow.append(next)
        }

        return result
    }

    /// Generate a string of characters by following character-level transitions.
    ///
    /// The `seed` provides the initial context window. Its length must be at
    /// least `order` characters:
    ///
    ///     var chain = MarkovChain(order: 2)
    ///     chain.trainString("abcabcabc")
    ///     try chain.generateString(seed: "ab", length: 9)  // → "abcabcabc"
    ///
    /// The output length is *total* characters, including those from the seed
    /// that are incorporated into the context. The returned string has exactly
    /// `length` characters.
    ///
    /// - Parameters:
    ///   - seed:   Starting characters; must have at least `order` characters.
    ///   - length: Desired total output length in characters.
    /// - Returns: A string of `length` characters.
    /// - Throws: `MarkovError.seedTooShort` or `MarkovError.unknownState`.
    public func generateString(seed: String, length: Int) throws -> String {
        let seedChars = seed.map { String($0) }

        guard seedChars.count >= order else {
            throw MarkovError.seedTooShort(required: order, got: seedChars.count)
        }

        // Build output starting with the seed characters.
        var result: [String] = seedChars
        // The context window is the last `order` characters of the result so far.
        var contextWindow: [String] = Array(seedChars.suffix(order))

        // Generate until we have `length` characters total.
        while result.count < length {
            let contextKey = contextWindow.joined(separator: "\0")
            let next = try nextStateForContext(contextKey)
            result.append(next)
            contextWindow.append(next)
            if contextWindow.count > order {
                contextWindow.removeFirst()
            }
        }

        // Trim or extend to exactly `length`.
        return result.prefix(length).joined()
    }

    // -----------------------------------------------------------------------
    // MARK: Probability Query
    // -----------------------------------------------------------------------

    /// Return the transition probability from one state to another.
    ///
    /// For order-1 chains, `from` is the context state.
    /// For order-k chains with k > 1, this queries the single-state context
    /// (which may not match any trained k-gram key).
    ///
    /// Returns 0.0 if the transition was never observed and smoothing is 0.
    ///
    /// - Parameters:
    ///   - from: The source / context state.
    ///   - to:   The target state.
    /// - Returns: Probability in [0, 1].
    public func probability(from: String, to: String) -> Double {
        transitions[from]?[to] ?? 0.0
    }

    // -----------------------------------------------------------------------
    // MARK: Stationary Distribution
    // -----------------------------------------------------------------------

    /// Compute the stationary distribution via **power iteration**.
    ///
    /// The stationary distribution π satisfies:
    ///
    ///     π · T = π      (π is a left eigenvector of T for eigenvalue 1)
    ///
    /// Intuitively, π[s] is the long-run fraction of time the chain spends in
    /// state s. For the weather model: π[Sunny] ≈ 0.47 means "in the long
    /// run, about 47% of days are sunny."
    ///
    /// **Power iteration algorithm:**
    ///
    ///     1. Start with a uniform distribution: π[s] = 1/n for all s.
    ///     2. Multiply: π_new[j] = Σᵢ π[i] · T[i][j]
    ///     3. Repeat until max change < 1e-10.
    ///
    /// - Returns: Dictionary mapping each state to its stationary probability.
    /// - Throws: `MarkovError.notErgodic` if the chain fails to converge
    ///           (after 10000 iterations) or has no trained transitions.
    public func stationaryDistribution() throws -> [String: Double] {
        // We work with the set of order-1 states (base states, not k-gram keys).
        let stateList = allStates.sorted()
        let n = stateList.count

        guard n > 0 else {
            throw MarkovError.notErgodic
        }

        // For order-1 chains only: we need T[i][j] for base states.
        // Start with a uniform distribution.
        var pi: [String: Double] = [:]
        for s in stateList {
            pi[s] = 1.0 / Double(n)
        }

        let maxIterations = 10_000
        let tolerance = 1e-10

        for _ in 0..<maxIterations {
            var piNew: [String: Double] = [:]
            for sj in stateList {
                var sum = 0.0
                for si in stateList {
                    // T[si → sj] is the probability from state si to sj.
                    let prob = transitions[si]?[sj] ?? 0.0
                    sum += (pi[si] ?? 0.0) * prob
                }
                piNew[sj] = sum
            }

            // Check for convergence: max absolute difference across all states.
            let maxDiff = stateList.map { abs((piNew[$0] ?? 0) - (pi[$0] ?? 0)) }.max() ?? 0
            pi = piNew

            if maxDiff < tolerance {
                // Normalise to ensure exact sum-to-1 (counteract float drift).
                let total = pi.values.reduce(0, +)
                if total > 0 {
                    for key in pi.keys {
                        pi[key]! /= total
                    }
                }
                return pi
            }
        }

        // Didn't converge — chain is not ergodic (absorbing state / disconnected).
        throw MarkovError.notErgodic
    }

    // -----------------------------------------------------------------------
    // MARK: Inspection
    // -----------------------------------------------------------------------

    /// Return all known base states, sorted alphabetically.
    ///
    /// These are the individual state tokens, not the k-gram context keys.
    ///
    /// - Returns: Sorted array of state names.
    public func states() -> [String] {
        allStates.sorted()
    }

    /// Return the full transition probability table.
    ///
    /// Keys are context keys (for order-1, these are identical to state names;
    /// for order-k, they are null-separated k-gram strings). Values are
    /// dictionaries mapping target state → probability.
    ///
    /// - Returns: `[contextKey: [targetState: probability]]`
    public func transitionMatrix() -> [String: [String: Double]] {
        transitions
    }
}
