## 0.4.0 - hierarchical shrink budget (VER05e section 4.1)

- **Security fix (CWE-400).** The shrink budget is now a tree charged all-or-nothing up
  to the outermost root (one lock, overflow-safe `cost > max - used`). Public entry
  points (`Runner.check`, `Gen.shrink`, `shrinkInt`, `shrinkList`, `shrinkBytes`,
  `simulateReplay`) open a child node with a local cap; internal combinators join.
  Total work of one top-level check is capped by the root whatever the nesting
  (property calling `Runner.check`, custom shrinker or filter predicate recursing).
- The property-depth classification (per-root counter, then per-thread cell) is removed.
- `Runner.check` charges every case after the first to an enclosing shrink; `shrinkInt`,
  `simulateReplay`, initial property checks and the Runner's private counter are charged.
- Closed scopes: a parent is the nearest open ancestor; a thread outliving the outermost
  root gets a fresh root. Pool-thread escape is documented and pinned by a test.
- `ShrinkBudget.defaultMaxWork` stays a package-private test hook read only at root creation.
- Behaviour note: a re-entrant inner check may be starved near outer exhaustion (affects
  minimality, never pass/fail). pbt-v1 stays 25/25 exact.
