# coding_adventures_markov_chain (Dart)

Order-k Markov chains with training, sampling, Laplace/Lidstone
smoothing, and stationary distributions via power iteration. The Dart
lane of the Rust `markov-chain` crate (spec DT28).

```dart
import 'package:coding_adventures_markov_chain/markov_chain.dart';

final chain = MarkovChain(2, smoothing: 1.0);
chain.trainString('the quick brown fox jumps over the lazy dog ' * 20);
print(chain.generateString('th', 60));
print(chain.probability('t\x00h', 'e'));
print(chain.stationaryDistribution());
```

## API

- `MarkovChain(order, {smoothing, states, random})` — order-k chain;
  throws [ArgumentError] for order 0 (the reference panics; Dart has no
  panics). Pass `random: Random(seed)` for reproducible sampling.
- Training: `train` (throws [SeedTooShortError] below `order + 1`
  tokens; counts accumulate across calls), `trainString` (Unicode scalar
  values, so astral characters count as one state).
- Sampling: `nextState` (inverse-CDF over scalar-sorted rows; throws
  [UnknownStateError]), `generate`, `generateString` (a `length` shorter
  than the seed returns the seed verbatim, as in the reference).
- Queries: `probability` (0.0 for unknown pairs),
  `stationaryDistribution` (power iteration, tol 1e-10, max 10,000
  steps; throws [NotErgodicError]), `states`, `transitionMatrix`.
- Errors: [MarkovError] with [UnknownStateError]/[NotErgodicError]/
  [SeedTooShortError] subtypes carrying the reference's exact message
  texts and `need`/`got` counts.
- Topology: transitions are recorded in the in-lane directed-graph
  package (`Graph(allowSelfLoops: true)`), mirroring the reference's
  `Graph::new_allow_self_loops`.

Row iteration sorts by Unicode scalar value (not UTF-16 code unit), so
sampling matches the reference's byte-order string sort even for mixes
of high-BMP and astral state labels.

## Validation

- **Unit** (`test/markov_chain_test.dart`): mirrors all 14 reference
  tests case-for-case, plus seeded-RNG determinism, astral-plane
  accounting, scalar-order sampling across BMP/astral, exact error
  values, and oscillation-detected non-ergodicity. 24 tests.
- **Property** (`test/properties_test.dart`): row-sum laws,
  probability/count agreement with an independent recount,
  multi-train accumulation, smoothing coverage, generation shape,
  seeded determinism, stationary fixed points, exact errors, and
  order-k context shape — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration script-driven
  differential campaign (hostile token sequences, lone surrogates,
  malformed UTF-8, NUL-bearing labels) against an independent
  sliding-window model predicting alphabet and probabilities after
  every op; only exact [MarkovError] may escape. Ad-hoc 20,000-iteration
  runs over two more seeds: zero divergences.
- **Coverage**: 99% lines, 96% branches, 100% functions. The two
  misses are defensive-only paths: the float-rounding sampling
  fallback and the zero-total normalise guard.
