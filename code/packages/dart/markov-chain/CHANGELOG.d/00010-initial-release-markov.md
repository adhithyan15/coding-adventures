## 0.1.0

- Initial Dart lane of the markov-chain identity: order-k `MarkovChain`
  with `train`/`trainString`, `nextState`/`generate`/`generateString`,
  Laplace/Lidstone smoothing, power-iteration
  `stationaryDistribution`, `probability`/`states`/`transitionMatrix`
  queries, and the `UnknownState`/`NotErgodic`/`SeedTooShort` error
  type; transitions recorded in the in-lane directed-graph package.
- Tests mirror the reference suite (14 unit tests), add 10 lane-extra
  unit tests, 9 seeded property tests over the in-lane `pbt` package,
  and a 2,000-iteration differential fuzz campaign over the in-lane
  `fuzz` package; 99% line, 96% branch, 100% function coverage.
