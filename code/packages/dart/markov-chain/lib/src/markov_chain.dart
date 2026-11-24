/// A general-purpose, order-k Markov chain (DT28).
///
/// A Markov chain models a system that moves between a finite set of
/// **states** over time, where the probability of the *next* state depends
/// **only on the current state** — not on any history of how the system got
/// here. This "memorylessness" property is the **Markov property**.
///
/// ## Intuition
///
/// Imagine reading a book one letter at a time. After the letter "q", the
/// letter "u" follows with very high probability in English. That is an
/// order-1 Markov chain over characters: the current state ("q") tells you
/// everything you need to predict the next state — you don't need to
/// remember whether "q" was preceded by "i" or "a" or anything else.
///
/// Andrei Markov (1856–1922) introduced this model in 1906 while analysing
/// the distribution of vowels and consonants in Pushkin's *Eugene Onegin*.
/// He wanted to show that the law of large numbers applied to *dependent*
/// events — a pointed rebuttal to critics who thought probability theory
/// was only for coin-flips.
///
/// ## Mathematical foundation
///
/// Given a finite state space S = {s₀, s₁, …, sₙ₋₁}, a Markov chain is
/// completely described by a **transition matrix** T where:
///
/// ```text
/// T[i][j]  =  P(next state = sⱼ  |  current state = sᵢ)
/// ```
///
/// Every row of T must sum to 1.0 — from any state the chain must go
/// *somewhere*, even if that somewhere is back to the same state.
///
/// ## Order-k chains
///
/// A standard (order-1) chain's next state depends on the last 1
/// observation. An **order-k** chain extends the memory window to k
/// observations:
///
/// ```text
/// P(sₙ | s_{n-k}, …, s_{n-1})  =  T[k-gram][sₙ]
/// ```
///
/// For text generation:
/// - Order 1: "e" → next-letter probabilities for each single letter
/// - Order 2: "th" → next-letter probabilities for each two-letter context
/// - Order 3: "the" → next-letter probabilities for each three-letter context
///
/// Higher orders produce more realistic output but require more training
/// data.
///
/// ## Graph representation
///
/// Internally we use a [Graph] (configured to allow self-loops, because
/// states *can* transition to themselves) to record the *topology* of the
/// chain: which states can follow which. Separate `_transitions` and
/// `_counts` maps store the edge probabilities and raw co-occurrence
/// counts respectively.
///
/// Why the graph at all? Because it gives us a reusable, well-tested data
/// structure for checking whether a transition is possible, and it keeps
/// the "topology" concern (which edges exist) separate from the "weight"
/// concern (how probable each edge is).
///
/// ## Smoothing
///
/// Without smoothing, a state the training data never left would have *no*
/// outgoing transition probabilities — the chain gets stuck. **Laplace
/// smoothing** (α = 1) adds a phantom count of α to every (context, target)
/// pair before normalising, ensuring every known state can transition to
/// every other known state with at least a small probability.
///
/// ```text
/// smoothed_count(i → j) = raw_count(i → j) + α
/// T[i][j]               = smoothed_count(i → j) / sum_k(smoothed_count(i → k))
/// ```
///
/// ## Stationary distribution (power iteration)
///
/// For an **ergodic** chain — one where every state is reachable from every
/// other state — there exists a unique **stationary distribution** π such
/// that:
///
/// ```text
/// π · T = π        (π is the left eigenvector of T for eigenvalue 1)
/// ```
///
/// This answers: "In the long run, what fraction of time does the chain
/// spend in each state?" We compute it by **power iteration**: start with a
/// uniform distribution and repeatedly apply T until the distribution stops
/// changing.
///
/// ```text
/// π₀  =  { s: 1/|S| for all s in S }           # uniform start
/// πₙ₊₁[sⱼ]  =  Σᵢ  πₙ[sᵢ] · T[sᵢ][sⱼ]       # one step
/// stop when  max|πₙ₊₁[s] - πₙ[s]| < 1e-10     # convergence
/// ```
library;

import 'dart:math';

import 'package:coding_adventures_directed_graph/directed_graph.dart';
import 'errors.dart';

/// The null-byte separator used to join k-gram tokens into a single key.
///
/// We use `\x00` because it cannot appear in normal text or state names,
/// so it is safe to use as a delimiter without ambiguity.
///
/// For example, the order-2 context ["a", "b"] becomes the key "a\x00b".
const String _contextSep = '\x00';

/// Maximum number of power-iteration steps before declaring non-convergence.
const int _maxIter = 10000;

/// Convergence tolerance for power iteration.
///
/// We declare convergence when the maximum absolute change across all
/// states in one step is smaller than this value. 1e-10 gives roughly 10
/// decimal digits of accuracy — more than enough for any practical use.
const double _convergenceTol = 1e-10;

/// A general-purpose, order-k Markov chain.
///
/// ## Construction
///
/// ```dart
/// // Order-1 chain with no pre-registered alphabet and no smoothing.
/// final chain = MarkovChain(1);
///
/// // Order-2 chain with a fixed alphabet {A, B, C} and Laplace smoothing.
/// final chain2 = MarkovChain(2, smoothing: 1.0, states: ['A', 'B', 'C']);
/// ```
///
/// ## Training
///
/// ```dart
/// final chain = MarkovChain(1);
/// chain.train(['A', 'B', 'A', 'C', 'A', 'B']);
/// assert((chain.probability('A', 'B') - 2 / 3).abs() < 1e-9);
/// ```
///
/// ## Generation
///
/// ```dart
/// final chain = MarkovChain(1, smoothing: 1.0);
/// chain.trainString('abcabc');
/// final seq = chain.generate('a', 5);
/// assert(seq.length == 5);
/// ```
class MarkovChain {
  /// Creates a new Markov chain.
  ///
  /// - [order] — how many past states form the context (1 = standard).
  /// - [smoothing] — Laplace/Lidstone smoothing parameter α (0 = none).
  /// - [states] — pre-registered alphabet; pass `[]` (the default) to let
  ///   the alphabet grow from training data.
  /// - [random] — the source of randomness for sampling. Defaults to an
  ///   unseeded [Random] (ambient entropy, like the reference's
  ///   `thread_rng`). Pass `Random(seed)` for reproducible generation.
  ///
  /// Throws [ArgumentError] if [order] is 0 — a zero-order chain is
  /// undefined. (The reference panics; Dart has no panics, so an argument
  /// error is the faithful mapping.)
  MarkovChain(
    this.order, {
    this.smoothing = 0.0,
    List<String> states = const [],
    Random? random,
  })  : alphabet = List<String>.of(states),
        _random = random ?? Random() {
    if (order < 1) {
      throw ArgumentError('Markov chain order must be at least 1');
    }
  }

  /// The order k of the chain.
  ///
  /// Order 1 = standard Markov chain (next state depends on current state).
  /// Order 2 = next state depends on previous 2 states (bigram context).
  /// Order k = next state depends on previous k states (k-gram context).
  final int order;

  /// Laplace / Lidstone smoothing parameter α.
  ///
  /// α = 0.0 → no smoothing (zero-probability transitions stay zero).
  /// α = 1.0 → Laplace smoothing (every (context, target) pair gets count 1).
  /// α > 0   → Lidstone smoothing (every pair gets count α).
  final double smoothing;

  /// The set of all known states (the "alphabet").
  ///
  /// States are registered either via the constructor (pre-registered
  /// alphabet) or implicitly as training data is observed.
  ///
  /// For order-k chains, this is the set of *individual* tokens, not
  /// k-grams. The k-gram contexts are computed on the fly during
  /// training/generation.
  final List<String> alphabet;

  /// The source of randomness used when sampling transitions.
  final Random _random;

  /// The directed graph storing transition topology.
  ///
  /// Nodes are k-gram context keys (e.g., "a\x00b" for the order-2 context
  /// ["a", "b"]). An edge from node C to node S exists iff the chain has
  /// observed at least one transition from context C to successor state S.
  ///
  /// Self-loops are allowed because a state can legitimately transition to
  /// itself (e.g., in a weather model "Sunny → Sunny").
  final Graph _graph = Graph(allowSelfLoops: true);

  /// Normalised transition probabilities.
  ///
  /// `_transitions[contextKey][targetState]` is the probability
  /// T[context][target].
  ///
  /// This is the primary query table. It is rebuilt from `_counts` every
  /// time [train] or [trainString] is called.
  final Map<String, Map<String, double>> _transitions = {};

  /// Raw co-occurrence counts accumulated during training.
  ///
  /// `_counts[contextKey][targetState]` is the number of times the chain
  /// observed `targetState` immediately following `contextKey` in the
  /// training data. Storing raw counts (instead of probabilities) lets us
  /// call [train] multiple times and accumulate counts correctly before
  /// re-normalising.
  final Map<String, Map<String, int>> _counts = {};

  // -------------------------------------------------------------------------
  // Training
  // -------------------------------------------------------------------------

  /// Trains the chain on a sequence of state labels.
  ///
  /// This method slides a window of size `order + 1` over [sequence]:
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
  /// Calling [train] multiple times accumulates counts before re-normalising,
  /// so the chain learns from the combined corpus.
  ///
  /// Throws [SeedTooShortError] if the sequence is shorter than
  /// `order + 1` (not enough data for even one window).
  void train(List<String> sequence) {
    // Need at least (order + 1) elements to form one window.
    if (sequence.length < order + 1) {
      throw SeedTooShortError(order + 1, sequence.length);
    }

    // Register any new states into the alphabet, preserving insertion
    // order and avoiding duplicates.
    for (final state in sequence) {
      _registerState(state);
    }

    // Slide the window.
    //
    // For each position i, the context is sequence[i..i+order] and the
    // target is sequence[i+order].
    for (var i = 0; i <= sequence.length - order - 1; i++) {
      final contextTokens = sequence.sublist(i, i + order);
      final target = sequence[i + order];
      final contextKey = contextTokens.join(_contextSep);

      // Increment the raw count for this (context, target) pair.
      final row = _counts.putIfAbsent(contextKey, () => {});
      row[target] = (row[target] ?? 0) + 1;

      // Record the edge in the topology graph (idempotent if it exists).
      // The graph node for the context is the k-gram key string.
      // The graph node for the target is the individual state label.
      _graph.addNode(contextKey);
      _graph.addEdge(contextKey, target);
    }

    // Re-normalise all rows after each training call.
    // We rebuild from scratch from `_counts` so that multiple
    // training calls accumulate correctly.
    _normalise();
  }

  /// Trains on a plain string, treating each character as a state.
  ///
  /// This is a convenience wrapper around [train] for character-level
  /// chains. "Character" means Unicode scalar value (Dart [Runes]), which
  /// matches the reference's `char` iteration — astral-plane characters
  /// such as emoji count as one state, not two UTF-16 code units.
  ///
  /// ## Note on the `probability` key format
  ///
  /// For order-k chains, [probability] takes the k-gram context key using
  /// the null-byte separator. For a character chain trained with order 2,
  /// the context for ["a", "b"] is the string `"a\x00b"`.
  ///
  /// [generateString] handles the key assembly automatically — users of
  /// that API never need to worry about the separator format.
  void trainString(String text) {
    final chars = text.runes.map(String.fromCharCode).toList();
    train(chars);
  }

  // -------------------------------------------------------------------------
  // Generation
  // -------------------------------------------------------------------------

  /// Samples the next state from a given state (or k-gram context key).
  ///
  /// For order-1 chains, [current] is a single state label like `"A"`.
  /// For order-k chains, [current] is a k-gram key like `"a\x00b"`.
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
  /// Rows iterate in sorted order for determinism — the same random value
  /// always produces the same state regardless of map ordering. Sorting is
  /// by Unicode scalar value so it matches the reference's byte-order
  /// string sort for every input, including mixtures of BMP and astral
  /// characters (where UTF-16 code-unit order and scalar order disagree).
  ///
  /// Throws [UnknownStateError] if [current] is not a known context key
  /// (either it was never trained or was never registered).
  String nextState(String current) {
    final row = _transitions[current];
    if (row == null) {
      throw UnknownStateError(current);
    }

    // Roulette-wheel (inverse CDF) sampling.
    final r = _random.nextDouble();

    final sortedPairs = row.entries.toList()
      ..sort((a, b) => _compareByScalar(a.key, b.key));

    var cumulative = 0.0;
    for (final entry in sortedPairs) {
      cumulative += entry.value;
      if (r < cumulative) {
        return entry.key;
      }
    }

    // Floating-point precision: if r is very close to 1.0 and rounding
    // means cumulative never reaches it, return the last state.
    return sortedPairs.last.key;
  }

  /// Generates a sequence of [length] states starting from [start].
  ///
  /// The output includes [start] (and, for order-k chains, each successive
  /// token appended to a sliding window of the last k tokens).
  ///
  /// For an **order-1** chain, [start] is a single state label.
  /// For an **order-k** chain, [start] is a k-gram key `"s₁\x00…\x00sₖ"`.
  ///
  /// The output list starts with the seed tokens, then fills up to
  /// [length] *individual* state tokens (not k-gram keys) — so when
  /// [length] is shorter than the seed, the seed is returned verbatim.
  ///
  /// ## How it works (order-2 example)
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
  /// Generation continues until `output.length == length`.
  ///
  /// Throws [UnknownStateError] if [start] is not a known context key,
  /// and [SeedTooShortError] if the seed contains fewer tokens than
  /// [order].
  List<String> generate(String start, int length) {
    if (length == 0) {
      return [];
    }

    // For order-1, the context is just the single start state.
    // For order-k, split the k-gram key back into tokens.
    final seedTokens = start.split(_contextSep);

    if (seedTokens.length < order) {
      throw SeedTooShortError(order, seedTokens.length);
    }

    // Verify the start context is known.
    if (!_transitions.containsKey(start)) {
      throw UnknownStateError(start);
    }

    // Build the output list. We start by collecting the seed tokens
    // (just `order` many), then keep appending sampled states.
    final output = List<String>.of(seedTokens);

    // The sliding window is maintained as the last `order` tokens in
    // `output`.
    while (output.length < length) {
      // Assemble the current context key from the last `order` tokens.
      final startIdx = output.length - order;
      final contextTokens = output.sublist(startIdx);
      final contextKey = contextTokens.join(_contextSep);

      // Sample the next state.
      final next = nextState(contextKey);
      output.add(next);
    }

    return output;
  }

  /// Generates a string of [length] characters from a character-level chain.
  ///
  /// [seed] must contain at least [order] characters. The output string
  /// contains exactly [length] characters (counted as Unicode scalar
  /// values, matching [trainString]), except that a [length] shorter
  /// than [order] returns the order seed characters verbatim.
  ///
  /// Throws [SeedTooShortError] if `seed.runes.length < order`, and
  /// [UnknownStateError] if the first k-gram in the seed is not a known
  /// context.
  String generateString(String seed, int length) {
    final chars = seed.runes.map(String.fromCharCode).toList();

    if (chars.length < order) {
      throw SeedTooShortError(order, chars.length);
    }

    // Build the context key from the first `order` chars of the seed.
    final contextKey = chars.sublist(0, order).join(_contextSep);

    // Use the generic `generate` on the context key.
    final tokens = generate(contextKey, length);

    // Concatenate all tokens into a single string.
    return tokens.join('');
  }

  // -------------------------------------------------------------------------
  // Queries
  // -------------------------------------------------------------------------

  /// Returns the transition probability from [from] to [to].
  ///
  /// For an order-1 chain, [from] is a single state label like `"A"`.
  /// For an order-k chain, [from] is a k-gram key like `"a\x00b"`.
  ///
  /// Returns `0.0` if either state is unknown or the transition was never
  /// observed (and smoothing is 0).
  double probability(String from, String to) => _transitions[from]?[to] ?? 0.0;

  /// Computes the stationary distribution using power iteration.
  ///
  /// The stationary distribution π satisfies π · T = π: multiplying the
  /// distribution by the transition matrix leaves it unchanged. It
  /// represents the long-run fraction of time the chain spends in each
  /// state.
  ///
  /// ## Algorithm
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
  /// Throws [NotErgodicError] if the iteration fails to converge within
  /// [_maxIter] steps. This typically means the chain is not strongly
  /// connected (some state has no outgoing transitions that reach all
  /// other states).
  Map<String, double> stationaryDistribution() {
    // Collect the canonical states. For an order-1 chain the states are
    // the individual labels in `alphabet`. For a higher-order chain the
    // "states" for power iteration are the context keys (k-grams) that
    // appear as rows in the transition table.
    //
    // We use the alphabet (individual tokens) for order-1, and the full
    // set of context keys for higher orders, to keep the semantics clean.
    final List<String> states;
    if (order == 1) {
      states = List<String>.of(alphabet);
    } else {
      states = _transitions.keys.toList()
        ..sort((a, b) => _compareByScalar(a, b));
    }

    final n = states.length;
    if (n == 0) {
      throw NotErgodicError();
    }

    // Initialise π as a uniform distribution.
    final uniform = 1.0 / n;
    var pi = {for (final s in states) s: uniform};

    // Power iteration.
    for (var iter = 0; iter < _maxIter; iter++) {
      final piNew = {for (final s in states) s: 0.0};

      // π_{n+1}[sⱼ] = Σᵢ  π_n[sᵢ] · T[sᵢ][sⱼ]
      for (final sI in states) {
        final piI = pi[sI] ?? 0.0;
        final row = _transitions[sI];
        if (row != null) {
          for (final entry in row.entries) {
            if (piNew.containsKey(entry.key)) {
              piNew[entry.key] = piNew[entry.key]! + piI * entry.value;
            }
          }
        }
      }

      // Check convergence: max absolute change across all states.
      var maxDelta = 0.0;
      for (final s in states) {
        final delta = ((piNew[s] ?? 0.0) - (pi[s] ?? 0.0)).abs();
        if (delta > maxDelta) {
          maxDelta = delta;
        }
      }

      pi = piNew;

      if (maxDelta < _convergenceTol) {
        return pi;
      }
    }

    throw NotErgodicError();
  }

  /// Returns the list of all known individual states (the alphabet).
  ///
  /// This is the set of individual state labels, not k-gram context keys.
  /// The order matches insertion order from training.
  List<String> states() => List<String>.of(alphabet);

  /// Returns the full transition matrix as a nested map.
  ///
  /// `result[contextKey][targetState]` = probability T[context][target].
  ///
  /// For an order-1 chain, `contextKey` is a single state label.
  /// For an order-k chain, `contextKey` is a k-gram joined by `\x00`.
  Map<String, Map<String, double>> transitionMatrix() => {
        for (final entry in _transitions.entries)
          entry.key: Map<String, double>.of(entry.value),
      };

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /// Registers a new individual state into the alphabet (idempotent).
  ///
  /// We check for existence before pushing to keep the alphabet free of
  /// duplicates while preserving insertion order.
  void _registerState(String state) {
    if (!alphabet.contains(state)) {
      alphabet.add(state);
    }
  }

  /// Normalises all rows in `_counts` into `_transitions`.
  ///
  /// For each context key C in `_counts`:
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
  /// With smoothing > 0, every state in [alphabet] gets a non-zero entry,
  /// even if it was never observed after this context.
  void _normalise() {
    _transitions.clear();

    for (final countsEntry in _counts.entries) {
      final obsCounts = countsEntry.value;
      final rawTotal = obsCounts.values.fold<int>(0, (a, b) => a + b);
      final nStates = alphabet.length.toDouble();

      // Total denominator with smoothing.
      //
      // Even if smoothing = 0, we must not divide by zero. If somehow
      // a context has zero raw counts (shouldn't happen, but defensive),
      // we skip it.
      final total = rawTotal.toDouble() + smoothing * nStates;
      if (total == 0.0) {
        continue;
      }

      final row = <String, double>{};

      if (smoothing > 0.0) {
        // With smoothing: emit a probability for *every* alphabet state.
        for (final state in alphabet) {
          final raw = (obsCounts[state] ?? 0).toDouble();
          row[state] = (raw + smoothing) / total;
        }
      } else {
        // No smoothing: only emit probabilities for *observed* transitions.
        for (final observed in obsCounts.entries) {
          row[observed.key] = observed.value.toDouble() / total;
        }
      }

      _transitions[countsEntry.key] = row;
    }
  }
}

/// Compares two strings by Unicode scalar value sequence.
///
/// Dart's default string ordering compares UTF-16 code units, which
/// disagrees with scalar-value order when a string starting with a
/// high BMP character (U+E000–U+FFFF) is compared against one starting
/// with an astral character: the astral lead surrogate (0xD800–0xDBFF)
/// sorts *below* the BMP value in UTF-16 order but *above* it in scalar
/// order. The reference sorts Rust `str`s, whose byte order equals scalar
/// order, so comparing rune-by-rune keeps both lanes identical.
int _compareByScalar(String a, String b) {
  final aRunes = a.runes.iterator;
  final bRunes = b.runes.iterator;
  while (true) {
    final aHas = aRunes.moveNext();
    final bHas = bRunes.moveNext();
    if (!aHas && !bHas) {
      return 0;
    }
    if (!aHas) {
      return -1;
    }
    if (!bHas) {
      return 1;
    }
    if (aRunes.current != bRunes.current) {
      return aRunes.current.compareTo(bRunes.current);
    }
  }
}
