//! Error types for the Markov Chain crate.
//!
//! There are three things that can go wrong when working with a Markov chain:
//!
//! 1. You ask about a state the chain has never seen.
//! 2. You ask for the stationary distribution of a chain that is not ergodic
//!    (not strongly connected), so power iteration cannot converge.
//! 3. You provide a seed that is too short for the chain's order.
//!
//! We use [`thiserror`] to derive the boilerplate `Display` and
//! `std::error::Error` implementations, keeping the error definitions concise
//! and readable.

use thiserror::Error;

/// All errors that can be returned by [`MarkovChain`] operations.
///
/// [`MarkovChain`]: crate::MarkovChain
#[derive(Debug, Error)]
pub enum MarkovError {
    /// The given state (or k-gram context key) is not in the chain's
    /// transition table.
    ///
    /// This happens when you call `next_state` or `generate` with a state
    /// that was never seen during training, or was never registered in the
    /// initial alphabet.
    ///
    /// # Example
    ///
    /// ```rust
    /// use coding_adventures_markov_chain::{MarkovChain, MarkovError};
    ///
    /// let chain = MarkovChain::new(1, 0.0, vec![]);
    /// let err = chain.next_state("ghost");
    /// assert!(matches!(err, Err(MarkovError::UnknownState(_))));
    /// ```
    #[error("Unknown state: {0}")]
    UnknownState(String),

    /// The chain's transition matrix did not converge to a stationary
    /// distribution within the maximum number of power-iteration steps.
    ///
    /// This typically means the chain is not **ergodic** — i.e., it is not
    /// strongly connected (some state cannot reach all other states).
    ///
    /// For example, a chain with two disconnected components:
    /// ```text
    /// A → B → A      C → D → C
    /// ```
    /// cannot have a single stationary distribution because the chain never
    /// mixes between {A, B} and {C, D}.
    ///
    /// Increase training data or add Laplace smoothing to connect the chain.
    #[error("Chain did not converge (not ergodic)")]
    NotErgodic,

    /// The seed provided to `generate` or `generate_string` is shorter than
    /// the chain's order.
    ///
    /// An order-k chain requires at least k tokens in the seed to form the
    /// first context window.
    ///
    /// The first field is the number of tokens required; the second is the
    /// number of tokens that were actually provided.
    ///
    /// # Example
    ///
    /// ```rust
    /// use coding_adventures_markov_chain::{MarkovChain, MarkovError};
    ///
    /// let mut chain = MarkovChain::new(2, 0.0, vec![]);
    /// chain.train_string("abcabc").unwrap();
    /// let err = chain.generate_string("a", 5);  // seed length 1 < order 2
    /// assert!(matches!(err, Err(MarkovError::SeedTooShort(2, 1))));
    /// ```
    #[error("Seed too short: need {0} chars, got {1}")]
    SeedTooShort(usize, usize),
}
