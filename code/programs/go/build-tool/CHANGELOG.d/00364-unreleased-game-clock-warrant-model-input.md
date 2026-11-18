## Unreleased — the GAME10 constructor Warrant model selects its Rust consumer

- The existing `rust/game-clock` repository-input row now covers the exact `code/specs/game-clock.warrant` path as well as the versioned v1 fixture. Its test verifies that the spec selects both the clock consumer and the global Rust Warrant checker, while other supported Warrant documents continue to select their own consumers. Lookalike spec names and fixture versions do not select the clock package.
