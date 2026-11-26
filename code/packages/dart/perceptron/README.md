# coding_adventures_perceptron (Dart)

Logistic perceptron trained by gradient descent. The Dart lane of the Rust
`perceptron` crate: full-batch training on binary cross-entropy through a
sigmoid unit, wired from the in-lane matrix, activation, and loss
packages.

```dart
import 'package:coding_adventures_perceptron/coding_adventures_perceptron.dart';

final p = Perceptron(1.0, 200);
p.fit([
  [0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]
], [
  [0.0], [0.0], [0.0], [1.0]
], 1000000);
print(p.predict([[1.0, 1.0]])); // [0.874...]
```

## API

- `Perceptron(learningRate, epochs)` — public `learningRate`, `epochs`,
  `weights` (null until fitted), `bias`.
- `fit(xData, yData, logSteps, {log})` — runs `epochs + 1` gradient
  updates from zero weights, reporting `Epoch … | BCE Loss: … | Bias: …`
  every `logSteps`-th epoch (counting zero) through `log`. Throws
  [ArgumentError] on empty/ragged inputs, [RangeError] when labels run
  out, [UnsupportedError] on zero `logSteps`.
- `predict(xData)` — one probability per row. Throws [StateError] before
  the first `fit`.

Two deliberate deltas from the reference: progress logging goes to the
optional `log` callback, silent by default (pass `log: print` for the
exact reference line format) so the package stays pure; and unfitted
prediction throws [StateError] instead of panicking on an unwrap. Both
are documented on the members.

## Validation

The reference ships no tests, so this lane defines the contract: 17 unit
tests (AND/OR learning to perfection with margins, loss decrease,
determinism, refit reset, log format/count, all four rejection modes),
5 seeded property tests (shapes, determinism, probability range, loss
decrease, refit reset), and a 2,000-iteration fuzz campaign asserting
exact outcome modeling (shapes → [ArgumentError], overrun →
[RangeError], zero logSteps → [UnsupportedError]) plus differential
checks on accepted inputs. Ad-hoc 60,000-iteration runs over three more
seeds: zero crashes. Coverage 100% lines, branches, and functions.
