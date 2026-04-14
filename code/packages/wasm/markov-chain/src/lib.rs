//! # Markov Chain WASM Bindings — DT28
//!
//! This crate is a thin `wasm-bindgen` wrapper around the
//! [`coding-adventures-markov-chain`] Rust crate.  It exposes
//! `WasmMarkovChain` to JavaScript/TypeScript consumers via WebAssembly.
//!
//! ## Why a separate crate?
//!
//! The core Rust crate is a pure-Rust library that compiles to a standard
//! `.rlib` and is used directly by other Rust packages.  Adding `wasm-bindgen`
//! attributes to that crate would couple it to the WASM toolchain.  Keeping the
//! bindings in a dedicated WASM crate means:
//! - The core crate stays WASM-free and can be used in CLI tools, tests, FFI, etc.
//! - The WASM crate can be rebuilt independently as `wasm-bindgen` evolves.
//!
//! ## JS/Wasm boundary challenges
//!
//! `wasm-bindgen` can pass scalars (`f64`, `bool`, `usize`) and strings (`&str`,
//! `String`) across the boundary trivially.  Complex Rust types like
//! `Vec<String>`, `HashMap<String, f64>`, and `HashMap<String, HashMap<String, f64>>`
//! are **not** directly passable.  We use two strategies:
//!
//! 1. **`Vec<String>` ↔ `Vec<String>`** — wasm-bindgen can pass `Vec<String>`
//!    as JS arrays automatically (it serialises each element).
//! 2. **`HashMap` ↔ JSON string** — maps are serialised to a JSON string on the
//!    Rust side and parsed by the JS caller.  Methods ending in `_json` return
//!    `String` containing a JSON object.
//!
//! ## Error handling
//!
//! Rust errors are mapped to `JsValue` via `to_js_error()`, which converts any
//! `Display` value to a JS string.  On the JS side, `try { … } catch (e) { … }`
//! will receive the error message string.

use coding_adventures_markov_chain::MarkovChain;
use wasm_bindgen::prelude::*;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Convert any Rust error that implements `Display` into a `JsValue` string,
/// which becomes a JS `Error` message when the WASM function throws.
fn to_js_error(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

// ── WasmMarkovChain ────────────────────────────────────────────────────────

/// A Markov Chain data structure exposed to JavaScript via WebAssembly.
///
/// States are always strings in this WASM binding (mirroring the underlying
/// Rust crate, which is `String`-keyed so that the directed-graph DT01 can
/// store them as graph nodes).
///
/// # Quick-start (JavaScript)
///
/// ```js
/// import init, { WasmMarkovChain } from './markov_chain_wasm.js';
/// await init();
///
/// const chain = new WasmMarkovChain(1, 0.0, []);
/// chain.train(["sunny","cloudy","rainy","sunny","sunny","cloudy"]);
/// console.log(chain.nextState("sunny"));          // e.g. "cloudy"
/// console.log(chain.probability("sunny","rainy")); // e.g. 0.333
/// console.log(JSON.parse(chain.stationaryDistributionJson()));
/// ```
#[wasm_bindgen]
pub struct WasmMarkovChain {
    /// The underlying Rust implementation.  All method calls delegate here.
    inner: MarkovChain,
}

#[wasm_bindgen]
impl WasmMarkovChain {
    /// Create a new Markov Chain.
    ///
    /// - `order` — memory window length (1 = standard, 2 = bigram, …)
    /// - `smoothing` — Laplace/Lidstone smoothing parameter (0.0 = none, 1.0 = Laplace)
    /// - `states` — optional pre-registered alphabet; pass an empty array `[]` to
    ///   discover states from training data only.
    ///
    /// Pre-registering states matters when `smoothing > 0` and you need
    /// `probability(A, C)` to work for a state `C` that never appeared as a
    /// *source* in the training data but should still be a possible target.
    #[wasm_bindgen(constructor)]
    pub fn new(order: usize, smoothing: f64, states: Vec<String>) -> Self {
        Self {
            inner: MarkovChain::new(order, smoothing, states),
        }
    }

    /// Train the chain on a sequence of state strings.
    ///
    /// Counts accumulate — calling `train` multiple times is equivalent to
    /// calling it once with the concatenated sequences (before re-normalising).
    /// The transition probabilities are re-normalised after every call.
    pub fn train(&mut self, sequence: Vec<String>) -> Result<(), JsValue> {
        // Convert Vec<String> to Vec<&str> for the Rust API.
        let refs: Vec<&str> = sequence.iter().map(String::as_str).collect();
        self.inner.train(&refs).map_err(to_js_error)
    }

    /// Convenience method: treat every character of `text` as a state.
    ///
    /// Equivalent to `train(text.split('').filter(Boolean))` in JS, but more
    /// efficient because no JS array allocation is needed.
    #[wasm_bindgen(js_name = "trainString")]
    pub fn train_string(&mut self, text: &str) -> Result<(), JsValue> {
        self.inner.train_string(text).map_err(to_js_error)
    }

    /// Sample one transition from the current state.
    ///
    /// Returns the next state string, or throws if `current` is unknown.
    /// Uses the thread-local random number generator (`rand::thread_rng`).
    #[wasm_bindgen(js_name = "nextState")]
    pub fn next_state(&self, current: &str) -> Result<String, JsValue> {
        self.inner.next_state(current).map_err(to_js_error)
    }

    /// Generate a sequence of exactly `length` states starting from `start`.
    ///
    /// Returns a `Vec<String>` (which wasm-bindgen serialises as a JS `Array`).
    /// For order-k chains, `start` must be the joined k-gram context key
    /// (the same format returned by the training internals).
    pub fn generate(&self, start: &str, length: usize) -> Result<Vec<String>, JsValue> {
        self.inner.generate(start, length).map_err(to_js_error)
    }

    /// Generate a string of exactly `length` characters, starting with `seed`.
    ///
    /// `seed` must be at least `order` characters long.  For an order-1 chain,
    /// `seed` is just the starting character; for order-2, at least two characters.
    #[wasm_bindgen(js_name = "generateString")]
    pub fn generate_string(&self, seed: &str, length: usize) -> Result<String, JsValue> {
        self.inner.generate_string(seed, length).map_err(to_js_error)
    }

    /// Return `T[from][to]`, or `0.0` if the transition was never observed
    /// (and smoothing is 0) or if either state is unknown.
    pub fn probability(&self, from: &str, to: &str) -> f64 {
        self.inner.probability(from, to)
    }

    /// Return the stationary distribution as a JSON string.
    ///
    /// The stationary distribution π answers: "In the long run, what fraction
    /// of time does the chain spend in each state?"  Computed via power
    /// iteration until convergence (|π_{n+1} − π_n| < 1e-10).
    ///
    /// Throws if the chain is not ergodic (not all states mutually reachable).
    ///
    /// # JavaScript example
    /// ```js
    /// const dist = JSON.parse(chain.stationaryDistributionJson());
    /// // { "sunny": 0.47, "cloudy": 0.28, "rainy": 0.25 }
    /// ```
    #[wasm_bindgen(js_name = "stationaryDistributionJson")]
    pub fn stationary_distribution_json(&self) -> Result<String, JsValue> {
        let dist = self.inner.stationary_distribution().map_err(to_js_error)?;
        serde_json::to_string(&dist).map_err(to_js_error)
    }

    /// Return all known states as a JS array of strings.
    ///
    /// Includes both pre-registered states and states discovered from training.
    /// The order is deterministic (sorted alphabetically by the underlying
    /// `HashMap` iteration, then sorted here for stability).
    pub fn states(&self) -> Vec<String> {
        let mut s = self.inner.states();
        s.sort();
        s
    }

    /// Return the full transition matrix as a JSON string.
    ///
    /// Format: `{ "from_state": { "to_state": probability, … }, … }`
    ///
    /// Only non-zero transitions are included when smoothing is 0.  With
    /// smoothing > 0, every `(from, to)` pair over the known alphabet
    /// will be present.
    ///
    /// # JavaScript example
    /// ```js
    /// const matrix = JSON.parse(chain.transitionMatrixJson());
    /// console.log(matrix["sunny"]["rainy"]); // e.g. 0.1
    /// ```
    #[wasm_bindgen(js_name = "transitionMatrixJson")]
    pub fn transition_matrix_json(&self) -> String {
        let matrix = self.inner.transition_matrix();
        serde_json::to_string(&matrix).expect("transition matrix is always serialisable")
    }
}

// ── Native (non-WASM) tests ────────────────────────────────────────────────
//
// wasm-bindgen doesn't support running tests under `cargo test` on native
// targets.  The `#[cfg(not(target_arch = "wasm32"))]` guard lets us test the
// WASM bindings using a regular `cargo test` run without needing a WASM
// runtime.  The tests exercise the same public API that JS consumers will use.

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn simple_chain() -> WasmMarkovChain {
        // Weather model: Sunny → Cloudy → Rainy → Sunny (cycle)
        let mut chain = WasmMarkovChain::new(1, 0.0, vec![]);
        chain
            .train(vec![
                "sunny".to_string(),
                "cloudy".to_string(),
                "rainy".to_string(),
                "sunny".to_string(),
                "sunny".to_string(),
                "cloudy".to_string(),
                "sunny".to_string(),
            ])
            .unwrap();
        chain
    }

    #[test]
    fn train_and_probability() {
        let chain = simple_chain();
        // "sunny" appeared 4 times; it transitioned to "cloudy" 3 times and
        // to "sunny" 0 times in this short sequence.
        // Check that probability is a valid float in [0, 1].
        let p = chain.probability("sunny", "cloudy");
        assert!(p >= 0.0 && p <= 1.0, "probability out of range: {p}");
    }

    #[test]
    fn next_state_returns_known_state() {
        let chain = simple_chain();
        let next = chain.next_state("sunny").unwrap();
        // The result must be one of the known states.
        let states = chain.states();
        assert!(states.contains(&next), "next_state returned unknown state: {next}");
    }

    #[test]
    fn next_state_unknown_throws() {
        // `to_js_error` calls `JsValue::from_str` which invokes the wasm-bindgen
        // FFI layer and panics on native (non-wasm32) targets.  We test the
        // identical error path via the underlying Rust type instead, since the
        // WASM wrapper is a thin `map_err(to_js_error)` delegation.
        use coding_adventures_markov_chain::MarkovChain;
        let mut inner = MarkovChain::new(1, 0.0, vec![]);
        inner
            .train(&[
                "sunny", "cloudy", "rainy", "sunny", "sunny", "cloudy", "sunny",
            ])
            .unwrap();
        assert!(
            inner.next_state("unknown_state_xyz").is_err(),
            "expected error for unknown state"
        );
    }

    #[test]
    fn generate_returns_correct_length() {
        let chain = simple_chain();
        let seq = chain.generate("sunny", 8).unwrap();
        assert_eq!(seq.len(), 8, "generate should return exactly length items");
    }

    #[test]
    fn generate_string_character_chain() {
        let mut chain = WasmMarkovChain::new(2, 0.0, vec![]);
        chain.train_string("abcabcabc").unwrap();
        let result = chain.generate_string("ab", 9).unwrap();
        // Order-2 chain on "abcabcabc" has deterministic transitions.
        assert_eq!(result, "abcabcabc");
    }

    #[test]
    fn stationary_distribution_sums_to_one() {
        // Build an ergodic chain: each state can reach every other.
        let mut chain = WasmMarkovChain::new(1, 1.0, vec![]);
        chain
            .train(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "a".to_string(),
                "c".to_string(),
                "b".to_string(),
            ])
            .unwrap();
        let json = chain.stationary_distribution_json().unwrap();
        let dist: std::collections::HashMap<String, f64> =
            serde_json::from_str(&json).unwrap();
        let total: f64 = dist.values().sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "stationary distribution should sum to 1.0, got {total}"
        );
    }

    #[test]
    fn transition_matrix_json_is_valid_json() {
        let chain = simple_chain();
        let json = chain.transition_matrix_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object(), "transition matrix JSON must be an object");
    }

    #[test]
    fn states_returns_sorted_list() {
        let chain = simple_chain();
        let states = chain.states();
        let mut sorted = states.clone();
        sorted.sort();
        assert_eq!(states, sorted, "states() should return a sorted list");
    }

    #[test]
    fn laplace_smoothing_unseen_transition() {
        let mut chain =
            WasmMarkovChain::new(1, 1.0, vec!["A".into(), "B".into(), "C".into()]);
        chain.train(vec!["A".to_string(), "B".to_string()]).unwrap();
        // With 3 states and Laplace smoothing: P(A→C) = (0+1)/(1+3) = 0.25
        let p = chain.probability("A", "C");
        assert!(
            (p - 0.25).abs() < 1e-9,
            "Laplace-smoothed P(A→C) should be 0.25, got {p}"
        );
    }

    #[test]
    fn train_string_convenience() {
        let mut chain = WasmMarkovChain::new(1, 0.0, vec![]);
        chain.train_string("aab").unwrap();
        // "a" appears twice; transitions: a→a (1), a→b (1)
        let p_aa = chain.probability("a", "a");
        let p_ab = chain.probability("a", "b");
        assert!(
            (p_aa + p_ab - 1.0).abs() < 1e-9,
            "row should sum to 1.0: {p_aa} + {p_ab}"
        );
    }
}
