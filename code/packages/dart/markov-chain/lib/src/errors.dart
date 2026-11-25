/// Error types for the Markov chain package.
///
/// Three things can go wrong when working with a Markov chain:
///
/// 1. You ask about a state the chain has never seen ([UnknownStateError]).
/// 2. You ask for the stationary distribution of a chain that is not ergodic
///    (not strongly connected), so power iteration cannot converge
///    ([NotErgodicError]).
/// 3. You provide a seed that is too short for the chain's order
///    ([SeedTooShortError]).
///
/// These mirror the Rust reference's `MarkovError` variants, including the
/// display messages, so behavior stays identical across languages.
library;

import 'markov_chain.dart' show MarkovChain;

/// Base class for all errors thrown by [MarkovChain] operations.
class MarkovError implements Exception {
  /// Creates an error with the given display [message].
  const MarkovError(this.message);

  /// Human-readable description of the failure.
  final String message;

  @override
  String toString() => message;
}

/// The given state (or k-gram context key) is not in the chain's
/// transition table.
///
/// This happens when you call [MarkovChain.nextState] or
/// [MarkovChain.generate] with a state that was never seen during training,
/// or was never registered in the initial alphabet.
class UnknownStateError extends MarkovError {
  /// Creates an error naming the unknown [state].
  UnknownStateError(this.state) : super('Unknown state: $state');

  /// The state (or context key) the chain does not know.
  final String state;
}

/// The chain's transition matrix did not converge to a stationary
/// distribution within the maximum number of power-iteration steps.
///
/// This typically means the chain is not **ergodic** — i.e., it is not
/// strongly connected (some state cannot reach all other states).
///
/// For example, a chain with two disconnected components:
///
/// ```text
/// A → B → A      C → D → C
/// ```
///
/// cannot have a single stationary distribution because the chain never
/// mixes between {A, B} and {C, D}.
///
/// Increase training data or add Laplace smoothing to connect the chain.
class NotErgodicError extends MarkovError {
  /// Creates a not-ergodic error.
  NotErgodicError() : super('Chain did not converge (not ergodic)');
}

/// The seed provided to `generate` or `generateString` is shorter than
/// the chain's order.
///
/// An order-k chain requires at least k tokens in the seed to form the
/// first context window.
class SeedTooShortError extends MarkovError {
  /// Creates an error recording the [need]ed and [got] token counts.
  SeedTooShortError(this.need, this.got)
      : super('Seed too short: need $need chars, got $got');

  /// The number of tokens required.
  final int need;

  /// The number of tokens that were actually provided.
  final int got;
}
