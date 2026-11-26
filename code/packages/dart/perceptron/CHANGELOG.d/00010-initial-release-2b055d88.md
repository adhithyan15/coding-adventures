## 0.1.0

- Initial Dart lane of the perceptron identity: logistic unit with
  full-batch gradient-descent `fit` and `predict`, wired from the in-lane
  matrix, activation-functions, and loss-functions packages. Progress
  logging is an optional callback (silent by default); unfitted prediction
  throws `StateError`.
- The reference ships no tests, so this lane defines the contract: 17 unit
  tests (AND/OR learning, loss decrease, determinism, logging, all
  rejection modes), 5 seeded property tests over the in-lane `pbt`
  package, and a 2,000-iteration fuzz campaign over the in-lane `fuzz`
  package with exact outcome modeling; 100% line/branch/function coverage.
