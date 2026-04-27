//! # Markov Chain — DT28
//!
//! A Markov Chain models a system that moves between a finite set of **states**
//! over time, where the probability of the *next* state depends **only on the
//! current state** — not on any history of how the system got here.  This
//! "memorylessness" property is the **Markov property**.
//!
//! ## Intuition
//!
//! Imagine you are reading a book one letter at a time.  After the letter "q",
//! the letter "u" follows with very high probability in English.  That is an
//! order-1 Markov chain over characters: the current state ("q") tells you
//! everything you need to predict the next state — you don't need to remember
//! whether "q" was preceded by "i" or "a" or anything else.
//!
//! Andrei Markov (1856–1922) introduced this model in 1906 while analysing the
//! distribution of vowels and consonants in Pushkin's *Eugene Onegin*.  He
//! wanted to show that the law of large numbers applied to *dependent* events —
//! a pointed rebuttal to critics who thought probability theory was only for
//! coin-flips.
//!
//! ## Mathematical Foundation
//!
//! Given a finite state space S = {s₀, s₁, …, sₙ₋₁}, a Markov chain is
//! completely described by a **transition matrix** T where:
//!
//! ```text
//! T[i][j]  =  P(next state = sⱼ  |  current state = sᵢ)
//! ```
//!
//! Every row of T must sum to 1.0 — from any state the chain must go
//! *somewhere*, even if that somewhere is back to the same state.
//!
//! ## Order-k Chains
//!
//! A standard (order-1) chain's next state depends on the last 1 observation.
//! An **order-k** chain extends the memory window to k observations:
//!
//! ```text
//! P(sₙ | s_{n-k}, …, s_{n-1})  =  T[k-gram][sₙ]
//! ```
//!
//! For text generation:
//! - Order 1: "e" → next-letter probabilities for each single letter
//! - Order 2: "th" → next-letter probabilities for each two-letter context
//! - Order 3: "the" → next-letter probabilities for each three-letter context
//!
//! Higher orders produce more realistic output but require more training data.
//!
//! ## Graph Representation
//!
//! Internally we use a [`directed_graph::Graph`] (configured to allow
//! self-loops, because states *can* transition to themselves) to record the
//! *topology* of the chain: which states can follow which.  Separate
//! `transitions` and `counts` `HashMap`s store the edge probabilities and
//! raw co-occurrence counts respectively.
//!
//! Why the graph at all?  Because it gives us a reusable, well-tested
//! data structure for checking whether a transition is possible, and it
//! keeps the "topology" concern (which edges exist) separate from the
//! "weight" concern (how probable each edge is).
//!
//! ## Smoothing
//!
//! Without smoothing, a state the training data never left would have *no*
//! outgoing transition probabilities — the chain gets stuck.  **Laplace
//! smoothing** (α = 1) adds a phantom count of α to every (context, target)
//! pair before normalising, ensuring every known state can transition to
//! every other known state with at least a small probability.
//!
//! ```text
//! smoothed_count(i → j) = raw_count(i → j) + α
//! T[i][j]               = smoothed_count(i → j) / sum_k(smoothed_count(i → k))
//! ```
//!
//! ## Stationary Distribution (Power Iteration)
//!
//! For an **ergodic** chain — one where every state is reachable from every
//! other state — there exists a unique **stationary distribution** π such that:
//!
//! ```text
//! π · T = π        (π is the left eigenvector of T for eigenvalue 1)
//! ```
//!
//! This answers: "In the long run, what fraction of time does the chain spend
//! in each state?"  We compute it by **power iteration**: start with a uniform
//! distribution and repeatedly apply T until the distribution stops changing.
//!
//! ```text
//! π₀  =  { s: 1/|S| for all s in S }           # uniform start
//! πₙ₊₁[sⱼ]  =  Σᵢ  πₙ[sᵢ] · T[sᵢ][sⱼ]       # one step
//! stop when  max|πₙ₊₁[s] - πₙ[s]| < 1e-10     # convergence
//! ```

use std::collections::HashMap;

use directed_graph::Graph;
use rand::Rng;

// Re-export the error type so downstream crates only need one import.
pub use errors::MarkovError;

pub mod errors;

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

/// The null-byte separator used to join k-gram tokens into a single string key.
///
/// We use `\x00` because it cannot appear in normal text or state names,
/// so it is safe to use as a delimiter without ambiguity.
///
/// For example, the order-2 context ["a", "b"] becomes the key "a\x00b".
const CONTEXT_SEP: char = '\x00';

/// Maximum number of power-iteration steps before declaring non-convergence.
const MAX_ITER: usize = 10_000;

/// Convergence tolerance for power iteration.
///
/// We declare convergence when the maximum absolute change across all states
/// in one step is smaller than this value.  1e-10 gives us roughly 10 decimal
/// digits of accuracy — more than enough for any practical use.
const CONVERGENCE_TOL: f64 = 1e-10;

// ---------------------------------------------------------------------------
// MarkovChain struct
// ---------------------------------------------------------------------------

/// A general-purpose, order-k Markov Chain.
///
/// # Construction
///
/// ```rust
/// use coding_adventures_markov_chain::MarkovChain;
///
/// // Order-1 chain with no pre-registered alphabet and no smoothing.
/// let mut chain = MarkovChain::new(1, 0.0, vec![]);
///
/// // Order-2 chain with a fixed alphabet {A, B, C} and Laplace smoothing.
/// let chain2 = MarkovChain::new(2, 1.0, vec![
///     "A".into(), "B".into(), "C".into()
/// ]);
/// ```
///
/// # Training
///
/// ```rust
/// use coding_adventures_markov_chain::MarkovChain;
///
/// let mut chain = MarkovChain::new(1, 0.0, vec![]);
/// chain.train(&["A", "B", "A", "C", "A", "B"]).unwrap();
/// assert!((chain.probability("A", "B") - 2.0/3.0).abs() < 1e-9);
/// ```
///
/// # Generation
///
/// ```rust
/// use coding_adventures_markov_chain::MarkovChain;
///
/// let mut chain = MarkovChain::new(1, 1.0, vec![]);
/// chain.train_string("abcabc").unwrap();
/// let seq = chain.generate("a", 5).unwrap();
/// assert_eq!(seq.len(), 5);
/// ```
pub struct MarkovChain {
    /// The order k of the chain.
    ///
    /// Order 1 = standard Markov chain (next state depends on current state).
    /// Order 2 = next state depends on previous 2 states (bigram context).
    /// Order k = next state depends on previous k states (k-gram context).
    order: usize,

    /// Laplace / Lidstone smoothing parameter α.
    ///
    /// α = 0.0 → no smoothing (zero-probability transitions stay zero).
    /// α = 1.0 → Laplace smoothing (every (context, target) pair gets count 1).
    /// α > 0   → Lidstone smoothing (every pair gets count α).
    smoothing: f64,

    /// The set of all known states (the "alphabet").
    ///
    /// States are registered either via the constructor (pre-registered
    /// alphabet) or implicitly as training data is observed.  Stored as a
    /// `Vec` to preserve insertion order and allow stable indexing.
    ///
    /// For order-k chains, this is the set of *individual* tokens, not k-grams.
    /// The k-gram contexts are computed on the fly during training/generation.
    alphabet: Vec<String>,

    /// The directed graph storing transition topology.
    ///
    /// Nodes in the graph are k-gram context keys (e.g., "a\x00b" for the
    /// order-2 context ["a", "b"]).  An edge from node C to node S exists
    /// iff the chain has observed at least one transition from context C to
    /// successor state S.
    ///
    /// We use `new_allow_self_loops()` because a state can legitimately
    /// transition to itself (e.g., in a weather model "Sunny → Sunny").
    graph: Graph,

    /// Normalised transition probabilities.
    ///
    /// `transitions[context_key][target_state]` is the probability T[context][target].
    ///
    /// This is the primary query table.  It is rebuilt from `counts` every
    /// time `train` or `train_string` is called.
    transitions: HashMap<String, HashMap<String, f64>>,

    /// Raw co-occurrence counts accumulated during training.
    ///
    /// `counts[context_key][target_state]` is the number of times the chain
    /// observed `target_state` immediately following `context_key` in the
    /// training data.  Storing raw counts (instead of probabilities) lets us
    /// call `train` multiple times and accumulate counts correctly before
    /// re-normalising.
    counts: HashMap<String, HashMap<String, u64>>,
}

impl MarkovChain {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new Markov Chain.
    ///
    /// # Parameters
    ///
    /// - `order`    — how many past states form the context (1 = standard).
    /// - `smoothing` — Laplace/Lidstone smoothing parameter α (0 = none).
    /// - `states`   — pre-registered alphabet; pass `vec![]` to let the
    ///                alphabet grow from training data.
    ///
    /// # Panics
    ///
    /// Panics if `order` is 0 — a zero-order chain is undefined.
    pub fn new(order: usize, smoothing: f64, states: Vec<String>) -> Self {
        assert!(order >= 1, "Markov chain order must be at least 1");

        MarkovChain {
            order,
            smoothing,
            alphabet: states,
            graph: Graph::new_allow_self_loops(),
            transitions: HashMap::new(),
            counts: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Training
    // -----------------------------------------------------------------------

    /// Train the chain on a sequence of state labels.
    ///
    /// This method slides a window of size `order + 1` over `sequence`:
    ///
    /// ```text
    /// sequence: [s₀, s₁, s₂, s₃, s₄]    (order = 2)
    ///
    /// Windows:
    ///   [s₀, s₁, s₂]  →  context = "s₀\x00s₁",  target = s₂
    ///   [s₁, s₂, s₃]  →  context = "s₁\x00s₂",  target = s₃
    ///   [s₂, s₃, s₄]  →  context = "s₂\x00s₃",  target = s₄
    /// ```
    ///
    /// After processing all windows, counts are normalised (with smoothing)
    /// to produce the transition probability table.
    ///
    /// Calling `train` multiple times accumulates counts before re-normalising,
    /// so the chain learns from the combined corpus.
    ///
    /// # Errors
    ///
    /// Returns [`MarkovError::SeedTooShort`] if the sequence is shorter than
    /// `order + 1` (not enough data for even one window).
    pub fn train(&mut self, sequence: &[&str]) -> Result<(), MarkovError> {
        // Need at least (order + 1) elements to form one window.
        if sequence.len() < self.order + 1 {
            return Err(MarkovError::SeedTooShort(self.order + 1, sequence.len()));
        }

        // Register any new states into the alphabet.
        //
        // We use a small helper that only pushes if the state is not already
        // present, preserving insertion order and avoiding duplicates.
        for &state in sequence {
            self.register_state(state);
        }

        // Slide the window.
        //
        // For each position i, the context is sequence[i..i+order] and the
        // target is sequence[i+order].
        for i in 0..=(sequence.len() - self.order - 1) {
            let context_tokens = &sequence[i..i + self.order];
            let target = sequence[i + self.order];
            let context_key = context_tokens.join(&CONTEXT_SEP.to_string());

            // Increment the raw count for this (context, target) pair.
            self.counts
                .entry(context_key.clone())
                .or_default()
                .entry(target.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);

            // Record the edge in the topology graph (idempotent if it exists).
            // The graph node for the context is the k-gram key string.
            // The graph node for the target is the individual state label.
            let _ = self.graph.add_node(&context_key);
            let _ = self.graph.add_edge(&context_key, target);
        }

        // Re-normalise all rows after each training call.
        // We rebuild from scratch from `self.counts` so that multiple
        // training calls accumulate correctly.
        self.normalise();

        Ok(())
    }

    /// Train on a plain string, treating each character as a state.
    ///
    /// This is a convenience wrapper around `train` for character-level chains.
    ///
    /// ```rust
    /// use coding_adventures_markov_chain::MarkovChain;
    ///
    /// let mut chain = MarkovChain::new(2, 0.0, vec![]);
    /// chain.train_string("abcabcabc").unwrap();
    /// assert_eq!(chain.probability("a\x00b", "c"), 1.0);
    /// ```
    ///
    /// # Note on the `probability` key format
    ///
    /// For order-k chains, `probability` takes the k-gram context key using
    /// the null-byte separator.  For a character chain trained with order 2,
    /// the context for ["a", "b"] is the string `"a\x00b"`.
    ///
    /// `generate_string` handles the key assembly automatically — users of
    /// that API never need to worry about the separator format.
    pub fn train_string(&mut self, text: &str) -> Result<(), MarkovError> {
        let chars: Vec<String> = text.chars().map(|c| c.to_string()).collect();
        let refs: Vec<&str> = chars.iter().map(|s| s.as_str()).collect();
        self.train(&refs)
    }

    // -----------------------------------------------------------------------
    // Generation
    // -----------------------------------------------------------------------

    /// Sample the next state from a given state (or k-gram context key).
    ///
    /// For order-1 chains, `current` is a single state label like `"A"`.
    /// For order-k chains, `current` is a k-gram key like `"a\x00b"`.
    ///
    /// Sampling uses the **inverse CDF** (a.k.a. roulette-wheel selection):
    ///
    /// ```text
    /// r = uniform random in [0, 1)
    /// cumulative = 0
    /// for (state, probability) in sorted(T[current]):
    ///   cumulative += probability
    ///   if r < cumulative: return state
    /// ```
    ///
    /// This correctly samples from any discrete probability distribution.
    ///
    /// # Errors
    ///
    /// Returns [`MarkovError::UnknownState`] if `current` is not a known
    /// context key (either it was never trained or was never registered).
    pub fn next_state(&self, current: &str) -> Result<String, MarkovError> {
        let row = self
            .transitions
            .get(current)
            .ok_or_else(|| MarkovError::UnknownState(current.to_string()))?;

        // Roulette-wheel (inverse CDF) sampling.
        //
        // We iterate in sorted order for determinism — the same random value
        // should always produce the same state regardless of HashMap ordering.
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();

        let mut sorted_pairs: Vec<(&String, &f64)> = row.iter().collect();
        sorted_pairs.sort_by_key(|(s, _)| s.as_str());

        let mut cumulative = 0.0_f64;
        for (state, &prob) in &sorted_pairs {
            cumulative += prob;
            if r < cumulative {
                return Ok((*state).clone());
            }
        }

        // Floating-point precision: if r is very close to 1.0 and rounding
        // means cumulative never reaches it, return the last state.
        Ok(sorted_pairs.last().unwrap().0.clone())
    }

    /// Generate a sequence of `length` states starting from `start`.
    ///
    /// The output includes `start` (and, for order-k chains, each successive
    /// token appended to a sliding window of the last k tokens).
    ///
    /// For an **order-1** chain, `start` is a single state label.
    /// For an **order-k** chain, `start` is a k-gram key `"s₁\x00…\x00sₖ"`.
    ///
    /// The output `Vec` always contains exactly `length` *individual* state
    /// tokens (not k-gram keys).
    ///
    /// # How it works (order-2 example)
    ///
    /// ```text
    /// start   = "a\x00b"    (context = ["a", "b"])
    /// step 1: sample next from "a\x00b" → "c";  output = ["a", "b", "c"]
    ///         new context = "b\x00c"
    /// step 2: sample next from "b\x00c" → "a";  output = ["a", "b", "c", "a"]
    ///         new context = "c\x00a"
    /// ...
    /// ```
    ///
    /// We keep generating until `output.len() == length`.
    ///
    /// # Errors
    ///
    /// - [`MarkovError::UnknownState`] if `start` is not a known context key.
    /// - [`MarkovError::SeedTooShort`] if the seed contains fewer tokens than `order`.
    pub fn generate(&self, start: &str, length: usize) -> Result<Vec<String>, MarkovError> {
        if length == 0 {
            return Ok(vec![]);
        }

        // For order-1, the context is just the single start state.
        // For order-k, split the k-gram key back into tokens.
        let seed_tokens: Vec<&str> = start.split(CONTEXT_SEP).collect();

        if seed_tokens.len() < self.order {
            return Err(MarkovError::SeedTooShort(self.order, seed_tokens.len()));
        }

        // Verify the start context is known.
        if !self.transitions.contains_key(start) {
            return Err(MarkovError::UnknownState(start.to_string()));
        }

        // Build the output vector.  We start by collecting the seed tokens
        // (just `order` many), then keep appending sampled states.
        let mut output: Vec<String> = seed_tokens.iter().map(|s| s.to_string()).collect();

        // The sliding window is maintained as a deque-like slice of the last
        // `order` tokens in `output`.
        while output.len() < length {
            // Assemble the current context key from the last `order` tokens.
            let start_idx = output.len() - self.order;
            let context_tokens: Vec<&str> = output[start_idx..].iter().map(|s| s.as_str()).collect();
            let context_key = context_tokens.join(&CONTEXT_SEP.to_string());

            // Sample the next state.
            let next = self.next_state(&context_key)?;
            output.push(next);
        }

        Ok(output)
    }

    /// Generate a string of `length` characters from a character-level chain.
    ///
    /// `seed` must contain at least `order` characters.  The output string
    /// always contains exactly `length` characters.
    ///
    /// # Errors
    ///
    /// - [`MarkovError::SeedTooShort`] if `seed.chars().count() < order`.
    /// - [`MarkovError::UnknownState`] if the first k-gram in the seed is
    ///   not a known context.
    pub fn generate_string(&self, seed: &str, length: usize) -> Result<String, MarkovError> {
        let chars: Vec<String> = seed.chars().map(|c| c.to_string()).collect();

        if chars.len() < self.order {
            return Err(MarkovError::SeedTooShort(self.order, chars.len()));
        }

        // Build the context key from the first `order` chars of the seed.
        let context_key = chars[..self.order].join(&CONTEXT_SEP.to_string());

        // Use the generic `generate` on the context key.
        let tokens = self.generate(&context_key, length)?;

        // Concatenate all tokens into a single string.
        Ok(tokens.join(""))
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Return the transition probability from `from` to `to`.
    ///
    /// For an order-1 chain, `from` is a single state label like `"A"`.
    /// For an order-k chain, `from` is a k-gram key like `"a\x00b"`.
    ///
    /// Returns `0.0` if either state is unknown or the transition was never
    /// observed (and smoothing is 0).
    pub fn probability(&self, from: &str, to: &str) -> f64 {
        self.transitions
            .get(from)
            .and_then(|row| row.get(to))
            .copied()
            .unwrap_or(0.0)
    }

    /// Compute the stationary distribution using power iteration.
    ///
    /// The stationary distribution π satisfies π · T = π: multiplying the
    /// distribution by the transition matrix leaves it unchanged.  It
    /// represents the long-run fraction of time the chain spends in each
    /// state.
    ///
    /// # Algorithm
    ///
    /// ```text
    /// π₀[s] = 1 / |S|          for all states s    (uniform start)
    ///
    /// loop:
    ///   π_{n+1}[sⱼ] = Σᵢ  π_n[sᵢ] · T[sᵢ][sⱼ]
    ///   if max|π_{n+1} - π_n| < 1e-10: break
    ///   π = π_{n+1}
    ///
    /// return π
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`MarkovError::NotErgodic`] if the iteration fails to converge
    /// within [`MAX_ITER`] steps.  This typically means the chain is not
    /// strongly connected (some state has no outgoing transitions that reach
    /// all other states).
    pub fn stationary_distribution(&self) -> Result<HashMap<String, f64>, MarkovError> {
        // Collect the canonical states.  For an order-1 chain the states are
        // the individual labels in `self.alphabet`.  For a higher-order chain
        // the "states" for power iteration are the context keys (k-grams) that
        // appear as rows in the transition table.
        //
        // We use the alphabet (individual tokens) for order-1, and the full
        // set of context keys for higher orders, to keep the semantics clean.
        let states: Vec<String> = if self.order == 1 {
            self.alphabet.clone()
        } else {
            let mut keys: Vec<String> = self.transitions.keys().cloned().collect();
            keys.sort();
            keys
        };

        let n = states.len();
        if n == 0 {
            return Err(MarkovError::NotErgodic);
        }

        // Initialise π as a uniform distribution.
        let uniform = 1.0 / n as f64;
        let mut pi: HashMap<String, f64> = states.iter().map(|s| (s.clone(), uniform)).collect();

        // Power iteration.
        for _ in 0..MAX_ITER {
            let mut pi_new: HashMap<String, f64> = states.iter().map(|s| (s.clone(), 0.0)).collect();

            // π_{n+1}[sⱼ] = Σᵢ  π_n[sᵢ] · T[sᵢ][sⱼ]
            for s_i in &states {
                let pi_i = *pi.get(s_i).unwrap_or(&0.0);
                if let Some(row) = self.transitions.get(s_i) {
                    for (s_j, &prob) in row {
                        if let Some(entry) = pi_new.get_mut(s_j) {
                            *entry += pi_i * prob;
                        }
                    }
                }
            }

            // Check convergence: max absolute change across all states.
            let max_delta = states
                .iter()
                .map(|s| (pi_new.get(s).unwrap_or(&0.0) - pi.get(s).unwrap_or(&0.0)).abs())
                .fold(0.0_f64, f64::max);

            pi = pi_new;

            if max_delta < CONVERGENCE_TOL {
                return Ok(pi);
            }
        }

        Err(MarkovError::NotErgodic)
    }

    /// Return the list of all known individual states (the alphabet).
    ///
    /// This is the set of individual state labels, not k-gram context keys.
    /// The order matches insertion order from training.
    pub fn states(&self) -> Vec<String> {
        self.alphabet.clone()
    }

    /// Return the full transition matrix as a nested `HashMap`.
    ///
    /// `result[context_key][target_state]` = probability T[context][target].
    ///
    /// For an order-1 chain, `context_key` is a single state label.
    /// For an order-k chain, `context_key` is a k-gram joined by `\x00`.
    pub fn transition_matrix(&self) -> HashMap<String, HashMap<String, f64>> {
        self.transitions.clone()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Register a new individual state into the alphabet (idempotent).
    ///
    /// We check for existence before pushing to keep the alphabet free of
    /// duplicates while preserving insertion order.
    fn register_state(&mut self, state: &str) {
        if !self.alphabet.contains(&state.to_string()) {
            self.alphabet.push(state.to_string());
        }
    }

    /// Normalise all rows in `self.counts` into `self.transitions`.
    ///
    /// For each context key C in `counts`:
    ///
    /// ```text
    /// total = Σ_{all states s} (counts[C][s] + smoothing)
    ///       = Σ_{observed s} counts[C][s]  +  smoothing * |alphabet|
    ///
    /// T[C][s] = (counts[C].get(s, 0) + smoothing) / total
    ///           for all s in alphabet   (if smoothing > 0)
    ///           for all observed s only (if smoothing == 0)
    /// ```
    ///
    /// With smoothing = 0, unobserved transitions are simply absent from the
    /// row (equivalent to probability 0).
    ///
    /// With smoothing > 0, every state in `self.alphabet` gets a non-zero
    /// entry, even if it was never observed after this context.
    fn normalise(&mut self) {
        self.transitions.clear();

        for (context_key, obs_counts) in &self.counts {
            let raw_total: u64 = obs_counts.values().sum();
            let n_states = self.alphabet.len() as f64;

            // Total denominator with smoothing.
            //
            // Even if smoothing = 0, we must not divide by zero.  If somehow
            // a context has zero raw counts (shouldn't happen, but defensive),
            // we skip it.
            let total = raw_total as f64 + self.smoothing * n_states;
            if total == 0.0 {
                continue;
            }

            let mut row: HashMap<String, f64> = HashMap::new();

            if self.smoothing > 0.0 {
                // With smoothing: emit a probability for *every* alphabet state.
                for state in &self.alphabet {
                    let raw = *obs_counts.get(state).unwrap_or(&0) as f64;
                    row.insert(state.clone(), (raw + self.smoothing) / total);
                }
            } else {
                // No smoothing: only emit probabilities for *observed* transitions.
                for (state, &raw) in obs_counts {
                    row.insert(state.clone(), raw as f64 / total);
                }
            }

            self.transitions.insert(context_key.clone(), row);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a freshly-trained chain for reuse across tests.
    fn train_abc() -> MarkovChain {
        // sequence: [A, B, A, C, A, B, B, A]
        // Transitions:
        //   A→B: 2,  A→C: 1   → A: {B: 2/3, C: 1/3}
        //   B→A: 2,  B→B: 1   → B: {A: 2/3, B: 1/3}
        //   C→A: 1             → C: {A: 1}
        let mut chain = MarkovChain::new(1, 0.0, vec![]);
        chain
            .train(&["A", "B", "A", "C", "A", "B", "B", "A"])
            .unwrap();
        chain
    }

    // -----------------------------------------------------------------------
    // Test 1: Construction — empty chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_construction_empty() {
        // An empty chain has no states and no transitions.
        let chain = MarkovChain::new(1, 0.0, vec![]);
        assert_eq!(chain.states().len(), 0);
        assert_eq!(chain.transition_matrix().len(), 0);
    }

    // -----------------------------------------------------------------------
    // Test 2: Train a single pair
    // -----------------------------------------------------------------------

    #[test]
    fn test_train_single_pair() {
        // Training on [A, B] means there is exactly one observed transition: A→B.
        // With no smoothing, T[A][B] = 1.0 and T[A][anything else] = 0.0.
        let mut chain = MarkovChain::new(1, 0.0, vec![]);
        chain.train(&["A", "B"]).unwrap();

        let p = chain.probability("A", "B");
        assert!(
            (p - 1.0).abs() < 1e-9,
            "probability(A,B) should be 1.0, got {}",
            p
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Train on a longer sequence
    // -----------------------------------------------------------------------

    #[test]
    fn test_train_sequence_probabilities() {
        let chain = train_abc();

        // A→B should be ≈ 2/3
        let pab = chain.probability("A", "B");
        assert!(
            (pab - 2.0 / 3.0).abs() < 1e-9,
            "P(A→B) expected ~0.667, got {}",
            pab
        );

        // A→C should be ≈ 1/3
        let pac = chain.probability("A", "C");
        assert!(
            (pac - 1.0 / 3.0).abs() < 1e-9,
            "P(A→C) expected ~0.333, got {}",
            pac
        );

        // B→A should be ≈ 2/3
        let pba = chain.probability("B", "A");
        assert!(
            (pba - 2.0 / 3.0).abs() < 1e-9,
            "P(B→A) expected ~0.667, got {}",
            pba
        );

        // B→B should be ≈ 1/3
        let pbb = chain.probability("B", "B");
        assert!(
            (pbb - 1.0 / 3.0).abs() < 1e-9,
            "P(B→B) expected ~0.333, got {}",
            pbb
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Laplace smoothing
    // -----------------------------------------------------------------------

    #[test]
    fn test_laplace_smoothing() {
        // Pre-register 3 states: A, B, C.  Train on [A, B] with smoothing=1.0.
        //
        // Raw counts: A→B: 1
        // After Laplace smoothing with |S|=3:
        //   total = 1 + 1 + 1 + 1*3 = (raw_sum=1) + (alpha * n_states=1*3) = 4
        //   P(A→A) = (0+1)/4 = 0.25
        //   P(A→B) = (1+1)/4 = 0.50
        //   P(A→C) = (0+1)/4 = 0.25
        let mut chain = MarkovChain::new(
            1,
            1.0,
            vec!["A".into(), "B".into(), "C".into()],
        );
        chain.train(&["A", "B"]).unwrap();

        let pac = chain.probability("A", "C");
        assert!(
            (pac - 0.25).abs() < 1e-9,
            "P(A→C) with Laplace smoothing expected 0.25, got {}",
            pac
        );

        let pab = chain.probability("A", "B");
        assert!(
            (pab - 0.5).abs() < 1e-9,
            "P(A→B) with Laplace smoothing expected 0.50, got {}",
            pab
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Generate returns the right length
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_length() {
        let chain = train_abc();
        let seq = chain.generate("A", 10).unwrap();
        assert_eq!(seq.len(), 10, "generate(A, 10) should return exactly 10 states");
    }

    // -----------------------------------------------------------------------
    // Test 6: Generate string length
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_string_length() {
        // Train a character chain on a small English phrase repeated many times
        // so that every bigram has enough data to generate from.
        let corpus = "the quick brown fox ".repeat(30);
        let mut chain = MarkovChain::new(1, 1.0, vec![]);
        chain.train_string(&corpus).unwrap();

        let result = chain.generate_string("t", 50).unwrap();
        assert_eq!(
            result.chars().count(),
            50,
            "generate_string should return exactly 50 chars"
        );
        assert!(
            result.starts_with('t'),
            "generate_string should start with the seed char"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Stationary distribution sums to 1
    // -----------------------------------------------------------------------

    #[test]
    fn test_stationary_distribution_sums_to_one() {
        // Use the ABC chain which is strongly connected (ergodic), so power
        // iteration should converge.
        let chain = train_abc();
        let dist = chain.stationary_distribution().unwrap();

        let sum: f64 = dist.values().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "stationary distribution should sum to 1.0, got {}",
            sum
        );
        assert!(dist.len() > 0, "stationary distribution should not be empty");
    }

    // -----------------------------------------------------------------------
    // Test 8: Order-2 chain on "abcabcabc"
    // -----------------------------------------------------------------------

    #[test]
    fn test_order2_chain_deterministic() {
        // With order=2, no smoothing, training on "abcabcabc":
        //   context "a\x00b" → "c" with probability 1.0  (seen 2 times, only successor)
        //   context "b\x00c" → "a" with probability 1.0
        //   context "c\x00a" → "b" with probability 1.0
        //
        // generate_string("ab", 9) must produce "abcabcabc" because every
        // transition is deterministic (probability 1.0 → no randomness).
        let mut chain = MarkovChain::new(2, 0.0, vec![]);
        chain.train_string("abcabcabc").unwrap();

        // Check probability of "ab" → "c".
        // The context key for ["a","b"] is "a\x00b".
        let p = chain.probability("a\x00b", "c");
        assert!(
            (p - 1.0).abs() < 1e-9,
            "P(ab→c) should be 1.0, got {}",
            p
        );

        // generate_string("ab", 9) should produce "abcabcabc" deterministically.
        let result = chain.generate_string("ab", 9).unwrap();
        assert_eq!(
            result, "abcabcabc",
            "order-2 chain on abcabcabc should regenerate the original"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Unknown state raises an error
    // -----------------------------------------------------------------------

    #[test]
    fn test_unknown_state_error() {
        let chain = train_abc();
        let err = chain.next_state("UNKNOWN");
        assert!(
            err.is_err(),
            "next_state on an unknown state should return Err"
        );
        match err.unwrap_err() {
            MarkovError::UnknownState(s) => {
                assert_eq!(s, "UNKNOWN");
            }
            other => panic!("Expected UnknownState, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: Multi-train accumulation
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_train_accumulation() {
        // Train on [A, B] twice.
        // After both calls:
        //   counts: A→B = 2
        //   total from A = 2
        //   P(A→B) should still be 1.0 (no new transitions introduced).
        let mut chain = MarkovChain::new(1, 0.0, vec![]);
        chain.train(&["A", "B"]).unwrap();
        chain.train(&["A", "B"]).unwrap();

        let p = chain.probability("A", "B");
        assert!(
            (p - 1.0).abs() < 1e-9,
            "P(A→B) after two identical train calls should be 1.0, got {}",
            p
        );

        // Now add a third training run with a new transition A→C.
        // counts: A→B = 2, A→C = 1  → P(A→B) ≈ 2/3.
        chain.train(&["A", "C"]).unwrap();

        let pab = chain.probability("A", "B");
        let pac = chain.probability("A", "C");
        assert!(
            (pab - 2.0 / 3.0).abs() < 1e-9,
            "P(A→B) should be ~2/3 after accumulation, got {}",
            pab
        );
        assert!(
            (pac - 1.0 / 3.0).abs() < 1e-9,
            "P(A→C) should be ~1/3 after accumulation, got {}",
            pac
        );
    }

    // -----------------------------------------------------------------------
    // Extra: train_string for order-1 character chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_train_string_character_level() {
        let mut chain = MarkovChain::new(1, 0.0, vec![]);
        // Train on "aaab" → a→a: 2, a→b: 1
        chain.train_string("aaab").unwrap();

        let paa = chain.probability("a", "a");
        let pab = chain.probability("a", "b");

        assert!(
            (paa - 2.0 / 3.0).abs() < 1e-9,
            "P(a→a) expected ~0.667, got {}",
            paa
        );
        assert!(
            (pab - 1.0 / 3.0).abs() < 1e-9,
            "P(a→b) expected ~0.333, got {}",
            pab
        );
    }

    // -----------------------------------------------------------------------
    // Extra: states() returns the registered alphabet
    // -----------------------------------------------------------------------

    #[test]
    fn test_states_returns_alphabet() {
        let mut chain = MarkovChain::new(1, 0.0, vec![]);
        chain.train(&["X", "Y", "Z", "X"]).unwrap();
        let mut s = chain.states();
        s.sort();
        assert_eq!(s, vec!["X", "Y", "Z"]);
    }

    // -----------------------------------------------------------------------
    // Extra: transition_matrix() round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn test_transition_matrix_round_trip() {
        let chain = train_abc();
        let matrix = chain.transition_matrix();

        // Every row in the matrix should sum to approximately 1.0.
        for (context, row) in &matrix {
            let sum: f64 = row.values().sum();
            assert!(
                (sum - 1.0).abs() < 1e-9,
                "Row '{}' sums to {} (expected 1.0)",
                context,
                sum
            );
        }
    }

    // -----------------------------------------------------------------------
    // Extra: generate_string errors on seed shorter than order
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_string_seed_too_short() {
        let mut chain = MarkovChain::new(2, 0.0, vec![]);
        chain.train_string("abcabc").unwrap();
        let err = chain.generate_string("a", 5);
        assert!(err.is_err());
        match err.unwrap_err() {
            MarkovError::SeedTooShort(need, got) => {
                assert_eq!(need, 2);
                assert_eq!(got, 1);
            }
            other => panic!("Expected SeedTooShort, got {:?}", other),
        }
    }
}
